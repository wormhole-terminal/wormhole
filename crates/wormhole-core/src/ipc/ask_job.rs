//! A request that waits for the user: the app queued a question and hands the socket the other end of a
//! channel; a thread of its own holds the connection until the answer comes (or the time is up),
//! then runs the request again, which now finds the user's answer, or refuses it. The socket thread never
//! waits for a person.
//!
//! The wait has two clocks: the question's turn to be shown (it may queue behind others), then the
//! minute the user has once it is on the screen ([`Decision::Shown`] starts it). A client never gives up
//! while its sheet is still up.

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};

use super::control::{ASK_QUEUE_MAX, ASK_WAIT_S, Reply};

/// What the app tells a request that waits for the user.
#[derive(Debug)]
pub enum Decision {
    /// Its question is on the screen now: the minute starts. Not an answer.
    Shown,
    /// Run the request again: the answer was recorded, and decides it.
    Retry,
    /// Answer with this and nothing more: the user said no, or nobody answered, or the question could not
    /// be shown.
    Refuse(Reply),
}

/// The waiting end of a question.
#[derive(Debug)]
pub struct AskJob {
    decision: Receiver<Decision>,
}

/// A little longer than the question lives, so the app's own timeout is the one that speaks.
const GRACE: Duration = Duration::from_secs(5);

/// How long a question may wait for its turn: every question ahead of it gets its minute.
const QUEUE_WAIT: Duration = Duration::from_secs((ASK_QUEUE_MAX as u64) * ASK_WAIT_S);

impl AskJob {
    /// A job and the end the app answers it on (room for the signal that the question is shown and the answer).
    pub fn channel() -> (SyncSender<Decision>, AskJob) {
        let (tx, rx) = mpsc::sync_channel(4);
        (tx, AskJob { decision: rx })
    }

    /// The answer, waiting up to `limit` for it: what a test reads of a question's end.
    pub fn decision_within(&self, limit: Duration) -> Option<Decision> {
        let until = Instant::now() + limit;
        loop {
            match self.decision.recv_timeout(until.saturating_duration_since(Instant::now())).ok()? {
                Decision::Shown => {}
                answer => return Some(answer),
            }
        }
    }

    /// Wait for the answer: for the turn to be shown, then for the user. `None` when the app went away or
    /// nobody decided.
    pub(super) fn wait(&self) -> Option<Decision> {
        let mut until = Instant::now() + QUEUE_WAIT + Duration::from_secs(ASK_WAIT_S) + GRACE;
        let mut shown = false;
        loop {
            match self.decision.recv_timeout(until.saturating_duration_since(Instant::now())).ok()? {
                Decision::Shown if !shown => {
                    shown = true;
                    until = Instant::now() + Duration::from_secs(ASK_WAIT_S) + GRACE;
                }
                Decision::Shown => {}
                answer => return Some(answer),
            }
        }
    }
}
