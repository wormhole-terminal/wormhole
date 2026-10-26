//! The thread of a stream: it waits on a subscription and writes what comes, one line at a time,
//! to a client that read its reply and then listens.
//!
//! A client that stops reading must cost the app nothing but this thread: the write times out and
//! the stream ends. One that is quiet for [`HEARTBEAT`] gets a line saying the app is alive, which is
//! also how a client that went away is found.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::ipc::bus::{Next, Subscription};
use crate::ipc::control::event::{CloseReason, HEARTBEAT, Notice, NoticeLine};
use crate::ipc::control::printable::json_line;
use crate::platform::socket::Stream;

/// How long writing one line may block before the client is taken to have stopped reading.
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
/// The thread only waits and writes.
const STACK_BYTES: usize = 256 * 1024;

/// The times a stream keeps; the tests shorten them.
#[derive(Debug, Clone, Copy)]
pub(super) struct Timing {
    pub(super) heartbeat: Duration,
    pub(super) write: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self { heartbeat: HEARTBEAT, write: WRITE_TIMEOUT }
    }
}

/// Hand `stream` (whose reply line is written) and its `subscription` to a thread that writes the
/// events until the client goes, the subscription closes or the app ends. Dropping what is not
/// started (the thread could not be made) gives the subscription's place back.
pub(super) fn start(stream: Stream, subscription: Subscription, timing: Timing) -> io::Result<()> {
    thread::Builder::new()
        .name("wormhole-stream".into())
        .stack_size(STACK_BYTES)
        .spawn(move || run(stream, &subscription, timing))
        .map(drop)
}

fn run(mut stream: Stream, subscription: &Subscription, timing: Timing) {
    let _ = stream.set_write_timeout(Some(timing.write));
    let mut line = Vec::with_capacity(512);
    loop {
        line.clear();
        match subscription.next(timing.heartbeat) {
            Next::Event(published) => line.extend_from_slice(subscription.line(&published).as_bytes()),
            Next::Lagged(dropped) => push_notice(&mut line, Notice::Lagged { dropped }),
            Next::Idle => push_notice(&mut line, Notice::Heartbeat { seq: subscription.last_seq() }),
            Next::Closed(reason) => {
                push_notice(&mut line, Notice::Closed { reason });
                let _ = write_line(&mut stream, &mut line);
                tracing::debug!(?reason, "event stream closed");
                return;
            }
        }
        if let Err(err) = write_line(&mut stream, &mut line) {
            tell_why_it_ended(&mut stream, &err, subscription);
            return;
        }
    }
}

fn push_notice(line: &mut Vec<u8>, notice: Notice) {
    line.extend_from_slice(json_line(&NoticeLine::new(notice)).as_bytes());
}

/// One line and its newline in one write, so a line is never split by another.
fn write_line(stream: &mut Stream, line: &mut Vec<u8>) -> io::Result<()> {
    line.push(b'\n');
    stream.write_all(line)
}

/// A client that stopped reading gets a last line saying so, if it will take one; one that is gone
/// gets nothing.
fn tell_why_it_ended(stream: &mut Stream, err: &io::Error, subscription: &Subscription) {
    let slow = matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut);
    tracing::info!(caller = ?subscription.caller(), slow, "event stream ended: {err}");
    if slow {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
        let mut line = Vec::new();
        push_notice(&mut line, Notice::Closed { reason: CloseReason::TooSlow });
        let _ = write_line(stream, &mut line);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::Timing;
    use crate::ipc::bus::{Bus, Grant};
    use crate::ipc::control::event::{Body, CloseReason, Event, Notice, StreamLine, Subscribed, Topic};
    use crate::ipc::control::{Action, ErrorCode, EventsSpec, Listing, Reply, Request};
    use crate::ipc::{Answer, Incoming, Message};
    use crate::platform::socket::{Listener, Stream};

    /// Answer the way the app does: a subscription for an `events` request, hooks passed to the test.
    fn serve(timing: Timing) -> (tempfile::TempDir, PathBuf, Arc<Bus>, mpsc::Receiver<Message>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let (tx, rx) = smol::channel::unbounded();
        super::super::spawn_with(Listener::bind(&socket).expect("bind"), tx, timing).expect("spawn");
        let bus = Bus::new();
        let (hooks, hooked) = mpsc::channel();
        std::thread::spawn({
            let bus = bus.clone();
            move || {
                while let Ok(incoming) = smol::block_on(rx.recv()) {
                    match incoming {
                        Incoming::Message(message) => {
                            let _ = hooks.send(message);
                        }
                        Incoming::Control { request, reply, .. } => {
                            let _ = reply.send(answer(&bus, &request));
                        }
                    }
                }
            }
        });
        (dir, socket, bus, hooked)
    }

    fn answer(bus: &Arc<Bus>, request: &Request) -> Answer {
        let Action::Events(spec) = &request.action else { panic!("only events are asked") };
        let filter = match spec.filter() {
            Ok(filter) => filter,
            Err(error) => return Answer::Reply(error.into()),
        };
        match bus.subscribe(filter, Grant::default(), request.from_tab()) {
            Ok((subscription, started)) => {
                let result = Subscribed {
                    subscribed: true,
                    seq: started.seq,
                    boot: started.boot,
                    topics: Vec::new(),
                    listing: Listing::default(),
                };
                Answer::Stream { reply: Reply::ok(result), subscription }
            }
            Err(refused) => Answer::Reply(Reply::error(ErrorCode::RateLimited, refused.to_string())),
        }
    }

    /// Open a stream as `from`, read its reply, and return the rest of the connection.
    fn watch(socket: &Path, from: Option<&str>) -> (Reply, BufReader<Stream>) {
        let mut stream = Stream::connect(socket).expect("connect");
        let request = Request::new(Action::Events(EventsSpec::default()), from.map(str::to_string));
        writeln!(stream, "{}", serde_json::to_string(&Message::Control(request)).expect("serializes"))
            .expect("write");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("a reply line");
        (serde_json::from_str(&line).expect("a reply"), reader)
    }

    fn read_line(reader: &mut BufReader<Stream>) -> Option<String> {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        }
    }

    fn bell(bus: &Bus, terminal: i64) {
        bus.publish(Topic::Bell, || Event { window: Some(1), terminal: Some(terminal), body: Body::Bell });
    }

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline, "{what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_stream_gets_its_reply_then_the_events_and_a_heartbeat_when_quiet() {
        let timing = Timing { heartbeat: Duration::from_millis(150), write: Duration::from_secs(2) };
        let (_dir, socket, bus, _hooks) = serve(timing);
        let (reply, mut reader) = watch(&socket, Some("7"));
        assert!(reply.ok, "{reply:?}");
        assert_eq!(reply.result.as_ref().and_then(|r| r["subscribed"].as_bool()), Some(true));
        bell(&bus, 3);
        bell(&bus, 4);
        let first = read_line(&mut reader).expect("an event");
        let Some(StreamLine::Event(first)) = StreamLine::parse(&first) else { panic!("{first}") };
        assert_eq!((first.seq, first.event.terminal), (1, Some(3)));
        let second = read_line(&mut reader).expect("an event");
        let Some(StreamLine::Event(second)) = StreamLine::parse(&second) else { panic!("{second}") };
        assert_eq!(second.seq, 2);
        // Quiet now: the app says it is alive, and at which seq.
        let beat = read_line(&mut reader).expect("a heartbeat");
        assert_eq!(StreamLine::parse(&beat), Some(StreamLine::Notice(Notice::Heartbeat { seq: 2 })));
    }

    #[test]
    fn closing_the_bus_ends_a_stream_with_the_reason() {
        let (_dir, socket, bus, _hooks) = serve(Timing::default());
        let (_, mut reader) = watch(&socket, None);
        wait_until("subscribed", || bus.subscribers() == 1);
        bus.close_all(CloseReason::Quitting);
        let last = read_line(&mut reader).expect("a closing line");
        assert_eq!(
            StreamLine::parse(&last),
            Some(StreamLine::Notice(Notice::Closed { reason: CloseReason::Quitting }))
        );
        assert!(read_line(&mut reader).is_none(), "and then the connection ends");
        wait_until("the place is given back", || bus.subscribers() == 0);
    }

    #[test]
    fn a_client_that_stops_reading_is_let_go_after_the_write_timeout() {
        let timing = Timing { heartbeat: Duration::from_secs(15), write: Duration::from_millis(200) };
        let (_dir, socket, bus, _hooks) = serve(timing);
        let (_, _reader_that_never_reads) = watch(&socket, None);
        wait_until("subscribed", || bus.subscribers() == 1);
        // Far more than the socket's buffer holds; the bus drops what its queue can't keep.
        let name = "x".repeat(60_000);
        for _ in 0..400 {
            bus.publish(Topic::Title, || Event {
                window: Some(1),
                terminal: Some(1),
                body: Body::Title { title: None, name: name.clone() },
            });
        }
        wait_until("the stream is ended and its place given back", || bus.subscribers() == 0);
    }

    #[test]
    fn sixteen_open_streams_do_not_delay_a_hook_and_the_seventeenth_is_refused() {
        let (_dir, socket, bus, hooks) = serve(Timing::default());
        let mut open = Vec::new();
        for caller in 0..16 {
            // Four streams to a caller, so four callers fill the sixteen.
            let (reply, reader) = watch(&socket, Some(&(caller / 4).to_string()));
            assert!(reply.ok, "stream {caller}: {reply:?}");
            open.push(reader);
        }
        let (refused, _) = watch(&socket, Some("9"));
        assert_eq!(refused.err().map(|e| e.code), Some(ErrorCode::RateLimited));
        assert!(refused.err().is_some_and(|e| e.message.contains("16 streams")));
        wait_until("all subscribed", || bus.subscribers() == 16);
        for _ in 0..50 {
            bell(&bus, 1);
        }
        let start = Instant::now();
        let mut hook = Stream::connect(&socket).expect("connect");
        writeln!(hook, r#"{{"kind":"hook","tab":"5","event":"Stop"}}"#).expect("write");
        let message = hooks.recv_timeout(Duration::from_secs(2)).expect("the hook arrives");
        assert!(matches!(message, Message::Hook { ref tab, .. } if tab == "5"), "{message:?}");
        assert!(start.elapsed() < Duration::from_millis(100), "delayed {:?}", start.elapsed());
        drop(open);
        // A client that is gone is found by the next write: here, the next event.
        wait_until("places given back", || {
            bell(&bus, 1);
            bus.subscribers() == 0
        });
    }
}
