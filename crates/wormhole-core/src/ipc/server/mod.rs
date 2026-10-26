//! The listener thread: reads hook messages from every connection, and answers control requests.
//!
//! Connections are served one at a time (a hook sends one short line), so
//! each gets a deadline for the whole connection, not per read, and a size
//! cap: a client that trickles bytes or never ends its line can delay the
//! others by at most the deadline and costs at most the cap in memory.
//!
//! Only this user's processes are served: the socket is 0600, and the peer's uid is checked all
//! the same (a socket file shared by mistake, or root, gets nothing). A control request is handed
//! to the app with a channel for its answer, which is written back on the connection; the app
//! answers on its UI thread in a moment, and a connection waits [`REPLY_WAIT`] at most for it.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use super::control::{self, ErrorCode, Reply, Request};
use wormhole_term::ReadScratch;

use super::{Answer, Incoming, Message};
use crate::platform::socket::{Listener, Stream, peer_is_same_user};

mod peer;
mod stream;

use stream::Timing;

pub use peer::{ANCESTORS_MAX, Peer};

/// How long one connection may take to send what it has, start to end.
const DEADLINE: Duration = Duration::from_secs(2);
/// How much one connection may send (a hook message is a few hundred bytes; a control request
/// at most a prompt's worth of text).
const MAX_BYTES: u64 = 64 * 1024;
/// How long a control request waits for the app's answer.
const REPLY_WAIT: Duration = Duration::from_secs(5);

/// Serve `listener` on its own thread, forwarding every message to `tx`.
pub(super) fn spawn(listener: Listener, tx: smol::channel::Sender<Incoming>) -> io::Result<()> {
    spawn_with(listener, tx, Timing::default())
}

/// [`spawn`] with the streams' times given (the tests shorten them).
fn spawn_with(listener: Listener, tx: smol::channel::Sender<Incoming>, timing: Timing) -> io::Result<()> {
    std::thread::Builder::new().name("wormhole-socket".into()).spawn(move || {
        // What a read copies the screen's rows into, kept between reads.
        let mut scratch = ReadScratch::default();
        loop {
            match listener.accept() {
                Ok((stream, _)) => serve(stream, &tx, timing, &mut scratch),
                Err(err) => {
                    // E.g. out of file descriptors: retrying at once would spin.
                    tracing::warn!("hook socket accept failed: {err}");
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    })?;
    Ok(())
}

/// Read one connection's messages until it closes, its deadline passes or it
/// sent too much; answer the control requests among them.
fn serve(
    mut stream: Stream,
    tx: &smol::channel::Sender<Incoming>,
    timing: Timing,
    scratch: &mut ReadScratch,
) {
    let accepted = SystemTime::now();
    if !peer_is_same_user(&stream) {
        tracing::warn!("hook socket: a connection from another user was refused");
        return;
    }
    let Ok(reading) = stream.try_clone() else { return };
    let reader = Deadline { stream: reading, until: Instant::now() + DEADLINE }.take(MAX_BYTES);
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        match serde_json::from_str::<Message>(&line) {
            Ok(Message::Control(request)) => {
                // A stream took the connection over: this thread has nothing more to read from it.
                if answer(&mut stream, accepted, request, tx, timing, scratch) == Outcome::Streaming {
                    return;
                }
            }
            Ok(message) => {
                let _ = tx.send_blocking(Incoming::Message(message));
            }
            // A control request this build can't read (an action newer than the app) is answered
            // at once, so the caller doesn't wait out its read timeout for nothing.
            Err(_) => {
                if let Some(reply) = control::unreadable(&line) {
                    let _ = write_reply(&mut stream, &reply);
                }
            }
        }
    }
}

/// What became of a connection after its request.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The request was answered; the connection may carry more.
    Answered,
    /// A stream thread has the connection now.
    Streaming,
}

/// Hand `request` to the app and write its answer back on `stream`; a stream's connection goes on to
/// a thread of its own.
fn answer(
    stream: &mut Stream,
    accepted: SystemTime,
    request: Request,
    tx: &smol::channel::Sender<Incoming>,
    timing: Timing,
    scratch: &mut ReadScratch,
) -> Outcome {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    let peer = Peer::of(stream, accepted);
    if tx.send_blocking(Incoming::Control { request, peer, reply: reply_tx }).is_err() {
        return Outcome::Answered;
    }
    let answer = reply_rx.recv_timeout(timing.reply_wait).unwrap_or_else(|_| {
        Answer::Reply(Reply::error(ErrorCode::Failed, "Wormhole did not answer in time"))
    });
    match answer {
        Answer::Reply(reply) => {
            let _ = write_reply(stream, &reply);
        }
        Answer::Read(job) => {
            let _ = write_reply(stream, &job.run(scratch));
        }
        Answer::Stream { reply, subscription } => {
            if !write_reply(stream, &reply) {
                return Outcome::Answered;
            }
            let Ok(writer) = stream.try_clone() else { return Outcome::Answered };
            match stream::start(writer, subscription, timing) {
                Ok(()) => return Outcome::Streaming,
                Err(err) => tracing::warn!("couldn't start an event stream: {err}"),
            }
        }
    }
    Outcome::Answered
}

/// Write `reply` as one line; a client that has gone is no error. Whether it was written.
fn write_reply(stream: &mut Stream, reply: &Reply) -> bool {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    writeln!(stream, "{}", control::printable::json_line(reply)).is_ok()
}

/// A stream whose reads fail once `until` has passed.
struct Deadline {
    stream: Stream,
    until: Instant,
}

impl Read for Deadline {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Best effort: macOS rejects setsockopt (EINVAL) on a socket whose peer already hung up,
        // and a hook is exactly that (it connects, writes one line and exits). The read then
        // returns what was sent followed by EOF at once, so no deadline is needed for it.
        let _ = self.stream.set_read_timeout(Some(left));
        self.stream.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::Shutdown;
    use std::time::{Duration, Instant};

    use super::spawn;
    use crate::ipc::control::{Action, ErrorCode, Reply, Request, Summary, TerminalState};
    use crate::ipc::test_support::{recv_message_within, recv_within};
    use crate::ipc::{Incoming, Message};
    use crate::platform::socket::{Listener, Stream};

    fn server() -> (tempfile::TempDir, std::path::PathBuf, smol::channel::Receiver<Incoming>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let (tx, rx) = smol::channel::unbounded();
        spawn(Listener::bind(&socket).expect("bind"), tx).expect("spawn");
        (dir, socket, rx)
    }

    fn hook(socket: &std::path::Path, tab: &str) {
        let mut stream = Stream::connect(socket).expect("connect");
        writeln!(stream, r#"{{"kind":"hook","tab":"{tab}","event":"Stop"}}"#).expect("write");
    }

    #[test]
    fn a_hook_that_has_already_hung_up_is_still_delivered() {
        // Nobody is accepting yet: the client connects, writes its line and closes first, as a
        // `Wormhole hook` process does when the server is busy.
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let listener = Listener::bind(&socket).expect("bind");
        hook(&socket, "3");
        let (tx, rx) = smol::channel::unbounded();
        spawn(listener, tx).expect("spawn");
        let message = recv_message_within(&rx, Duration::from_secs(3));
        assert!(matches!(message, Some(Message::Hook { ref tab, .. }) if tab == "3"), "{message:?}");
    }

    #[test]
    fn a_slow_client_holds_up_others_only_until_its_deadline() {
        let (_dir, socket, rx) = server();
        // A client that trickles a byte a second, never finishing a line.
        let slow = std::thread::spawn({
            let socket = socket.clone();
            move || {
                let mut stream = Stream::connect(socket).expect("connect");
                for _ in 0..8 {
                    if stream.write_all(b"{").is_err() {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        });
        std::thread::sleep(Duration::from_millis(100));
        let start = Instant::now();
        hook(&socket, "1");
        let message = recv_message_within(&rx, Duration::from_secs(6));
        assert!(matches!(message, Some(Message::Hook { ref tab, .. }) if tab == "1"), "{message:?}");
        assert!(start.elapsed() < Duration::from_millis(2500), "waited {:?}", start.elapsed());
        let _ = slow.join();
    }

    #[test]
    fn an_endless_line_is_cut_off_at_once() {
        let (_dir, socket, rx) = server();
        // A client streaming one line with no end: it must not be buffered
        // whole, nor hold up the next client until a deadline.
        let flood = std::thread::spawn({
            let socket = socket.clone();
            move || {
                let mut stream = Stream::connect(socket).expect("connect");
                let chunk = vec![b'x'; 1 << 16];
                let start = Instant::now();
                while start.elapsed() < Duration::from_secs(4) && stream.write_all(&chunk).is_ok() {}
            }
        });
        std::thread::sleep(Duration::from_millis(100));
        let start = Instant::now();
        hook(&socket, "2");
        let message = recv_message_within(&rx, Duration::from_secs(6));
        assert!(matches!(message, Some(Message::Hook { ref tab, .. }) if tab == "2"), "{message:?}");
        assert!(start.elapsed() < Duration::from_secs(1), "waited {:?}", start.elapsed());
        let _ = flood.join();
    }

    /// An older app drops a line it can't read and the caller waits out its timeout; from here on
    /// it is answered `unsupported` at once, so every later addition fails fast.
    #[test]
    fn an_action_this_build_does_not_know_is_answered_at_once() {
        let (_dir, socket, _rx) = server();
        let mut stream = Stream::connect(&socket).expect("connect");
        writeln!(
            stream,
            r#"{{"kind":"control","version":1,"action":"teleport","target":{{"kind":"terminal","id":3}}}}"#
        )
        .expect("write");
        stream.shutdown(Shutdown::Write).expect("half close");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let start = Instant::now();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).expect("a reply line");
        let reply: Reply = serde_json::from_str(&line).expect("parses");
        assert_eq!(reply.err().map(|e| e.code), Some(ErrorCode::Unsupported), "{line}");
        assert!(start.elapsed() < Duration::from_secs(1), "waited {:?}", start.elapsed());
    }

    /// A read is answered by the socket thread itself from the screen the app handed over: the app's
    /// thread only decides and returns the job.
    #[test]
    fn a_read_is_carried_out_on_the_socket_thread_from_the_job_the_app_hands_over() {
        use wormhole_term::testing::TestTerm;
        use wormhole_term::{CaptureRange, CaptureSpec};

        use crate::ipc::{Answer, ReadJob};

        let (_dir, socket, rx) = server();
        let app = std::thread::spawn(move || match recv_within(&rx, Duration::from_secs(5)) {
            Some(Incoming::Control { reply, .. }) => {
                let mut term = TestTerm::new(20, 3, 10);
                term.feed(b"hello\r\nworld");
                let spec = CaptureSpec {
                    range: CaptureRange::Screen,
                    join: false,
                    styles: false,
                    max_bytes: 1000,
                    max_rows: 100,
                };
                let job = ReadJob::lines(term.into_reader(), 3, Some("a title".into()), spec);
                reply.send(Answer::Read(job)).expect("the connection waits");
            }
            other => panic!("expected a control request: {other:?}"),
        });
        let mut stream = Stream::connect(&socket).expect("connect");
        let request = Message::Control(Request::new(Action::List, None));
        writeln!(stream, "{}", serde_json::to_string(&request).expect("serializes")).expect("write");
        stream.shutdown(Shutdown::Write).expect("half close");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).expect("a reply line");
        let reply: Reply = serde_json::from_str(&line).expect("parses");
        assert!(reply.ok, "{line}");
        let result = reply.result.expect("a result");
        assert_eq!(result["lines"], serde_json::json!(["hello", "world", ""]));
        assert_eq!((result["terminal"].as_i64(), result["title"].as_str()), (Some(3), Some("a title")));
        assert_eq!(result["cursor"]["col"].as_u64(), Some(5));
        app.join().expect("the app's side");
    }

    /// The round trip `Wormhole list` makes: the request reaches the app with a way to answer, and
    /// the answer comes back on the connection, after which hooks flow as before.
    #[test]
    fn a_control_request_is_answered_on_its_connection() {
        let (_dir, socket, rx) = server();
        // The app's side: answer the one request that arrives.
        let app = std::thread::spawn({
            let rx = rx.clone();
            move || match recv_within(&rx, Duration::from_secs(5)) {
                Some(Incoming::Control { request, peer, reply }) => {
                    assert_eq!(request.action, Action::List);
                    // The connection came from this process, whose parent chain follows.
                    assert_eq!(peer.pid, Some(std::process::id()));
                    assert!(peer.ancestors.len() <= super::ANCESTORS_MAX);
                    assert_eq!(request.from.as_deref(), Some("7"));
                    reply
                        .send(Reply::ok(Summary::of(1, [TerminalState::Working])).into())
                        .expect("the connection waits");
                }
                other => panic!("expected a control request: {other:?}"),
            }
        });
        let mut stream = Stream::connect(&socket).expect("connect");
        let request = Message::Control(Request::new(Action::List, Some("7".into())));
        writeln!(stream, "{}", serde_json::to_string(&request).expect("serializes")).expect("write");
        stream.shutdown(Shutdown::Write).expect("half close");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).expect("a reply line");
        let reply: Reply = serde_json::from_str(&line).expect("parses");
        assert!(reply.ok, "{reply:?}");
        assert_eq!(reply.result.as_ref().and_then(|r| r["working"].as_u64()), Some(1));
        app.join().expect("the app's side");
        // The connection is done; the next hook is delivered as ever.
        hook(&socket, "9");
        let message = recv_message_within(&rx, Duration::from_secs(3));
        assert!(matches!(message, Some(Message::Hook { ref tab, .. }) if tab == "9"), "{message:?}");
    }
}
