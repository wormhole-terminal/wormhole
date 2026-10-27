//! A request that waits for the user: the app queued a question and hands the socket the other end of a
//! channel; a thread of its own holds the connection until the answer comes (or the time is up),
//! then runs the request again, which now finds the user's answer, or refuses it. The socket thread never
//! waits for a person.

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

use super::control::{ASK_WAIT_S, Reply};

/// What the user's answer to a question means to the request that waited for it.
#[derive(Debug)]
pub enum Decision {
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

impl AskJob {
    /// A job and the end the app answers it on.
    pub fn channel() -> (SyncSender<Decision>, AskJob) {
        let (tx, rx) = mpsc::sync_channel(1);
        (tx, AskJob { decision: rx })
    }

    /// The decision, waiting up to `limit` for it: what a test reads of a question's end.
    pub fn decision_within(&self, limit: Duration) -> Option<Decision> {
        self.decision.recv_timeout(limit).ok()
    }

    /// Wait for the decision, or `None` when the app went away or never decided.
    pub(super) fn wait(&self) -> Option<Decision> {
        self.decision.recv_timeout(Duration::from_secs(ASK_WAIT_S) + GRACE).ok()
    }
}
