//! A request that waits for the user's answer to a question: a thread of its own holds the connection (the
//! socket thread never waits for a person), and when the answer comes the request runs again, or the
//! refusal the app decided on is written. A few at a time at most.

use std::io::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};

use wormhole_term::ReadScratch;

use super::{Outcome, Timing, ask_app, deliver, write_reply};
use crate::ipc::control::{Error, ErrorCode, Reason, Reply, Request};
use crate::ipc::{AskJob, Decision, Incoming, Peer};
use crate::platform::socket::Stream;

/// How many requests wait for the user at once: the share of the streams that questions get. Past it the
/// request is refused, not queued, so a looping caller cannot hold a thread each.
const WAITERS_MAX: usize = 16;

static WAITERS: AtomicUsize = AtomicUsize::new(0);

/// Take the connection over for `request`, which waits on `job`. The connection is the thread's from here.
pub(super) fn start(
    stream: &mut Stream,
    request: Request,
    peer: Peer,
    tx: &smol::channel::Sender<Incoming>,
    timing: Timing,
    job: AskJob,
) -> Outcome {
    let Ok(mut writer) = stream.try_clone() else { return Outcome::Answered };
    if WAITERS.fetch_add(1, Ordering::SeqCst) >= WAITERS_MAX {
        WAITERS.fetch_sub(1, Ordering::SeqCst);
        let error = Error::new(ErrorCode::NotAllowed, "too many questions are waiting for the user")
            .because(Reason::TooManyQuestions);
        let _ = write_reply(&mut writer, &error.into());
        return Outcome::Streaming;
    }
    let tx = tx.clone();
    let spawned = std::thread::Builder::new().name("wormhole-ask".into()).spawn(move || {
        let _count = Counted;
        run(&mut writer, request, peer, &tx, timing, job);
        let _ = writer.flush();
    });
    match spawned {
        Ok(_) => Outcome::Streaming,
        Err(err) => {
            WAITERS.fetch_sub(1, Ordering::SeqCst);
            tracing::warn!("couldn't start a thread to wait for the user: {err}");
            Outcome::Answered
        }
    }
}

/// Gives the waiter's place back when its thread ends, however it ends.
struct Counted;

impl Drop for Counted {
    fn drop(&mut self) {
        WAITERS.fetch_sub(1, Ordering::SeqCst);
    }
}

fn run(
    stream: &mut Stream,
    request: Request,
    peer: Peer,
    tx: &smol::channel::Sender<Incoming>,
    timing: Timing,
    job: AskJob,
) {
    let refuse = |stream: &mut Stream, error: Error| {
        let _ = write_reply(stream, &Reply::from(error));
    };
    match job.wait() {
        Some(Decision::Refuse(reply)) => {
            let _ = write_reply(stream, &reply);
        }
        Some(Decision::Retry) => {
            let mut scratch = ReadScratch::default();
            let Some(answer) = ask_app(&request, &peer, tx, timing) else { return };
            // A second question for the same request is a loop: it is answered with the refusal.
            if let Outcome::Asking(_) = deliver(stream, answer, &mut scratch, timing) {
                refuse(
                    stream,
                    Error::new(ErrorCode::NotAllowed, "the program is not allowed yet")
                        .because(Reason::ApprovalNeeded),
                );
            }
        }
        None => refuse(stream, Error::new(ErrorCode::Failed, "Wormhole did not answer in time")),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::Shutdown;
    use std::time::Duration;

    use crate::ipc::control::{Action, ErrorCode, Reason, Reply, Request};
    use crate::ipc::test_support::recv_within;
    use crate::ipc::{Answer, AskJob, Decision, Incoming, Message};
    use crate::platform::socket::{Listener, Stream};

    fn server() -> (tempfile::TempDir, std::path::PathBuf, smol::channel::Receiver<Incoming>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let (tx, rx) = smol::channel::unbounded();
        super::super::spawn(Listener::bind(&socket).expect("bind"), tx).expect("spawn");
        (dir, socket, rx)
    }

    /// A request that waits: written, the client's side of it half closed, as `wh` does.
    fn ask(socket: &std::path::Path) -> Stream {
        let mut stream = Stream::connect(socket).expect("connect");
        let request = Message::Control(Request::new(Action::List, None).asking());
        writeln!(stream, "{}", serde_json::to_string(&request).expect("serializes")).expect("write");
        stream.shutdown(Shutdown::Write).expect("half close");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        stream
    }

    fn reply_of(stream: Stream) -> Reply {
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).expect("a reply line");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"))
    }

    fn control(rx: &smol::channel::Receiver<Incoming>) -> (Request, std::sync::mpsc::SyncSender<Answer>) {
        match recv_within(rx, Duration::from_secs(5)) {
            Some(Incoming::Control { request, reply, .. }) => (request, reply),
            other => panic!("expected a control request: {other:?}"),
        }
    }

    /// The socket thread hands the question to a thread of its own: another request goes through while the user
    /// has not answered, and a refusal is written as the one reply.
    #[test]
    fn a_request_that_waits_for_the_user_holds_nothing_up_and_gets_the_refusal() {
        let (_dir, socket, rx) = server();
        let (decide, job) = AskJob::channel();
        let waiting = ask(&socket);
        let (request, reply) = control(&rx);
        assert!(request.ask, "the flag reaches the app");
        reply.send(Answer::Ask(job)).expect("the connection waits");

        // Meanwhile the socket serves others.
        let other = ask(&socket);
        let (_, other_reply) = control(&rx);
        other_reply.send(Reply::ok(serde_json::json!({ "n": 1 })).into()).expect("answers");
        assert!(reply_of(other).ok);

        let refusal = crate::ipc::control::Error::new(ErrorCode::NotAllowed, "no").because(Reason::Declined);
        decide.send(Decision::Refuse(refusal.into())).expect("the thread waits");
        let reply = reply_of(waiting);
        assert_eq!(reply.err().and_then(|e| e.reason), Some(Reason::Declined));
    }

    /// An answer that says "run it again" runs the same request through the app a second time, and the
    /// second answer is what the client reads.
    #[test]
    fn a_yes_runs_the_request_again_and_writes_that_answer() {
        let (_dir, socket, rx) = server();
        let (decide, job) = AskJob::channel();
        let waiting = ask(&socket);
        let (_, reply) = control(&rx);
        reply.send(Answer::Ask(job)).expect("waits");
        decide.send(Decision::Retry).expect("the thread waits");
        let (again, reply) = control(&rx);
        assert!(matches!(again.action, Action::List));
        reply.send(Reply::ok(serde_json::json!({ "second": true })).into()).expect("answers");
        let reply = reply_of(waiting);
        assert!(reply.ok && reply.result.expect("a result")["second"] == true);
    }

    /// A question that comes up again for the request it just answered is a loop, and is refused.
    #[test]
    fn a_request_that_must_be_asked_a_second_time_is_refused_not_asked_again() {
        let (_dir, socket, rx) = server();
        let (decide, job) = AskJob::channel();
        let waiting = ask(&socket);
        let (_, reply) = control(&rx);
        reply.send(Answer::Ask(job)).expect("waits");
        decide.send(Decision::Retry).expect("the thread waits");
        let (_, reply) = control(&rx);
        let (_, again) = AskJob::channel();
        reply.send(Answer::Ask(again)).expect("the thread waits");
        let reply = reply_of(waiting);
        assert_eq!(reply.err().and_then(|e| e.reason), Some(Reason::ApprovalNeeded));
    }

    /// The app going away without an answer ends the wait with words, not a hang.
    #[test]
    fn a_question_the_app_drops_ends_the_wait() {
        let (_dir, socket, rx) = server();
        let (decide, job) = AskJob::channel();
        let waiting = ask(&socket);
        let (_, reply) = control(&rx);
        reply.send(Answer::Ask(job)).expect("waits");
        drop(decide);
        let reply = reply_of(waiting);
        assert_eq!(reply.err().map(|e| e.code), Some(ErrorCode::Failed));
    }
}
