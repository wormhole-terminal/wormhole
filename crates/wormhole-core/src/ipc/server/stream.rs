//! The thread of a stream: it waits on a subscription and writes what comes, one line at a time,
//! to a client that read its reply and then listens.
//!
//! A client that stops reading must cost the app nothing but this thread: the write times out and
//! the stream ends. One that is quiet for [`HEARTBEAT`] gets a line saying the app is alive, which is
//! also how a client that went away is found.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::ipc::WaitJob;
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
    /// How long a control request waits for the app's answer.
    pub(super) reply_wait: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self { heartbeat: HEARTBEAT, write: WRITE_TIMEOUT, reply_wait: super::REPLY_WAIT }
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

/// A waiter evaluates patterns and copies a screen, so its stack is a little larger than a stream's.
const WAIT_STACK_BYTES: usize = 512 * 1024;

/// Hand `stream` and `job` to a thread that waits it out and writes the one reply line. The job's
/// subscription is given back when the thread ends, however it does.
pub(super) fn start_wait(stream: Stream, job: WaitJob) -> io::Result<()> {
    thread::Builder::new()
        .name("wormhole-wait".into())
        .stack_size(WAIT_STACK_BYTES)
        .spawn(move || {
            let mut stream = stream;
            let reply = match stream.try_clone() {
                Ok(mut probe) => job.finish(move || client_gone(&mut probe)),
                Err(_) => job.finish(|| false),
            };
            super::write_reply(&mut stream, &reply);
        })
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
            // Only a waiter is poked; an event stream has nothing to say about it.
            Next::Changed => continue,
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

/// Whether the client of `stream` has hung up. A client that sent its request and shut its writing side
/// is still listening for the answer; one that closed (it was killed, or interrupted) is not, and a write to
/// it fails with a broken pipe even when it has no bytes: that is how it is told, without reading anything.
/// (A system that does not say for an empty write leaves the wait to its own timeout.)
pub(super) fn client_gone(stream: &mut Stream) -> bool {
    matches!(
        stream.write(&[]),
        Err(err) if matches!(
            err.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset | io::ErrorKind::NotConnected
        )
    )
}

fn push_notice(line: &mut Vec<u8>, notice: Notice) {
    line.extend_from_slice(json_line(&NoticeLine::new(notice)).as_bytes());
}

/// One line and its newline in one write, so a line is never split by another.
fn write_line(stream: &mut Stream, line: &mut Vec<u8>) -> io::Result<()> {
    line.push(b'\n');
    stream.write_all(line)
}

/// The last line for a client that stopped reading. The write that timed out may have left half a
/// line in its buffer, so this one starts on a line of its own (a client skips an empty line).
fn too_slow_notice() -> Vec<u8> {
    let mut line = vec![b'\n'];
    push_notice(&mut line, Notice::Closed { reason: CloseReason::TooSlow });
    line
}

/// A client that stopped reading gets a last line saying so, if it will take one; one that is gone
/// gets nothing.
fn tell_why_it_ended(stream: &mut Stream, err: &io::Error, subscription: &Subscription) {
    let slow = matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut);
    tracing::info!(caller = ?subscription.caller(), slow, "event stream ended: {err}");
    if slow {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
        let mut line = too_slow_notice();
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

    use super::{Timing, client_gone, too_slow_notice};
    use crate::ipc::bus::{Bus, Grant};
    use crate::ipc::control::event::{Body, CloseReason, Event, Notice, StreamLine, Subscribed, Topic};
    use crate::ipc::control::{Action, ErrorCode, EventsSpec, Listing, Reply, Request};
    use crate::ipc::{Answer, Incoming, Message};
    use crate::platform::socket::{Listener, Stream};

    /// Answer the way the app does: a subscription for an `events` request, hooks passed to the test.
    fn serve(timing: Timing) -> (tempfile::TempDir, PathBuf, Arc<Bus>, mpsc::Receiver<Message>) {
        serve_slowly(timing, Duration::ZERO)
    }

    /// [`serve`] with an app that takes `delay` to hand over its answer, after it has subscribed.
    fn serve_slowly(
        timing: Timing,
        delay: Duration,
    ) -> (tempfile::TempDir, PathBuf, Arc<Bus>, mpsc::Receiver<Message>) {
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
                            let answered = answer(&bus, &request);
                            std::thread::sleep(delay);
                            let _ = reply.send(answered);
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
        let timing = Timing { heartbeat: Duration::from_millis(150), ..Timing::default() };
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
    fn a_client_that_closed_is_gone_and_one_that_only_shut_its_writing_side_is_not() {
        use std::net::Shutdown;

        let (mut ours, theirs) = Stream::pair().expect("a socket pair");
        assert!(!client_gone(&mut ours), "connected");
        theirs.shutdown(Shutdown::Write).expect("half close");
        assert!(!client_gone(&mut ours), "it sent its request and listens for the answer");
        drop(theirs);
        assert!(client_gone(&mut ours), "it closed");
    }

    #[test]
    fn the_last_line_for_a_slow_client_starts_on_a_line_of_its_own() {
        let bytes = too_slow_notice();
        assert_eq!(bytes[0], b'\n');
        let text = String::from_utf8(bytes).expect("text");
        assert_eq!(
            StreamLine::parse(text.trim()),
            Some(StreamLine::Notice(Notice::Closed { reason: CloseReason::TooSlow }))
        );
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
        let timing = Timing { write: Duration::from_millis(200), ..Timing::default() };
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

    fn title(bus: &Bus, name: &str) {
        bus.publish(Topic::Title, || Event {
            window: Some(1),
            terminal: Some(1),
            body: Body::Title { title: None, name: name.to_string() },
        });
    }

    /// The count is in the stream itself, not only in the queue: a client that was slower than the
    /// app reads a `lagged` line before it goes on.
    #[test]
    fn a_client_that_fell_behind_reads_how_many_events_it_lost() {
        let timing = Timing { write: Duration::from_secs(10), ..Timing::default() };
        let (_dir, socket, bus, _hooks) = serve(timing);
        let (_, mut reader) = watch(&socket, None);
        wait_until("subscribed", || bus.subscribers() == 1);
        // 10 MB at once against a queue of 4 MiB and a socket nobody reads yet.
        let name = "x".repeat(10_000);
        for _ in 0..1000 {
            title(&bus, &name);
        }
        let mut lost = None;
        for _ in 0..1100 {
            let Some(line) = read_line(&mut reader) else { break };
            if let Some(StreamLine::Notice(Notice::Lagged { dropped })) = StreamLine::parse(&line) {
                lost = Some(dropped);
                break;
            }
        }
        assert!(lost.is_some_and(|dropped| dropped > 0), "a lagged line: {lost:?}");
    }

    /// A write that times out may leave half a line behind: the stream ends, and a notice that
    /// follows it starts a line of its own.
    #[test]
    fn a_client_that_stops_reading_in_the_middle_of_a_line_ends_the_stream_cleanly() {
        use std::io::Read as _;

        let timing = Timing { write: Duration::from_millis(200), ..Timing::default() };
        let (_dir, socket, bus, _hooks) = serve(timing);
        let (_, mut reader) = watch(&socket, None);
        wait_until("subscribed", || bus.subscribers() == 1);
        title(&bus, &"y".repeat(400_000));
        wait_until("the stream ended", || bus.subscribers() == 0);
        let mut rest = Vec::new();
        let _ = reader.get_ref().set_read_timeout(Some(Duration::from_secs(5)));
        reader.read_to_end(&mut rest).expect("the connection ends: it is not left hanging");
        let text = String::from_utf8_lossy(&rest);
        if let Some(at) = text.find(r#""event":"closed""#) {
            assert!(text[..at].ends_with("{\"version\":1,") && text[..at].contains('\n'), "on its own line");
            assert!(
                text[..at].rfind('\n').is_some_and(|nl| text[nl + 1..at].starts_with('{')),
                "{}",
                &text[..at.min(80)]
            );
        }
    }

    /// The socket gave up waiting for the app (`reply_wait`); what the app hands over late is
    /// dropped with the channel, and its subscription goes with it.
    #[test]
    fn an_answer_that_comes_after_the_socket_gave_up_leaves_no_subscriber() {
        let timing = Timing { reply_wait: Duration::from_millis(200), ..Timing::default() };
        let (_dir, socket, bus, _hooks) = serve_slowly(timing, Duration::from_millis(700));
        let (reply, _reader) = watch(&socket, None);
        assert_eq!(reply.err().map(|e| e.code), Some(ErrorCode::Failed), "{reply:?}");
        wait_until("the late subscription is given back", || bus.subscribers() == 0);
    }
}
