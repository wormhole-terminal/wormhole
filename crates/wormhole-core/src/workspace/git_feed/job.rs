//! Bookkeeping for results that arrive later.
//!
//! Git work runs on a thread of the shell's and its result comes back some time after it was asked
//! for. By then the question may have changed: the terminal moved to another folder. A [`Job`]
//! remembers the one request a terminal is still waiting for, and a result is applied only if it
//! carries that request's [`Ticket`]; anything else is stale and dropped, however late it lands.
//! (The app's `window/repo/job.rs`, whose rules these are.)

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Names one request. Numbers come from a single process-wide counter, so a ticket is never issued
/// twice: not to another terminal, not after a state was replaced by a fresh one, not in another
/// window. A counter per owner would restart at 1 with the owner and let a late result of the old
/// one pass for the new one's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ticket(u64);

impl Ticket {
    /// A ticket nobody holds: it never matches a request.
    pub const NONE: Ticket = Ticket(0);

    fn issue() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// What the work itself holds: its ticket, and whether anyone still waits for it. Long work checks
/// [`Self::still_wanted`] between its steps and stops early, so a superseded request does not keep a
/// core busy for nothing.
#[derive(Clone, Debug)]
pub struct Wanted {
    ticket: Ticket,
    latest: Arc<AtomicU64>,
}

impl Wanted {
    pub fn ticket(&self) -> Ticket {
        self.ticket
    }

    /// False once the job started another request, was cancelled or was dropped.
    pub fn still_wanted(&self) -> bool {
        self.latest.load(Ordering::Relaxed) == self.ticket.0
    }
}

/// The request a terminal is waiting for: what it asked (`key`) and the ticket the answer must
/// carry. One request at a time; a new one supersedes the old.
#[derive(Debug)]
pub(super) struct Job<K> {
    pending: Option<(K, Ticket)>,
    latest: Arc<AtomicU64>,
}

impl<K> Default for Job<K> {
    fn default() -> Self {
        Self { pending: None, latest: Arc::default() }
    }
}

impl<K> Job<K> {
    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
}

impl<K: PartialEq> Job<K> {
    /// Ask for `key`. `None` when the same request is already in flight: its answer will do, so
    /// nothing is started again. Otherwise the request supersedes whatever was pending, and the
    /// returned [`Wanted`] goes with the work.
    pub(super) fn begin(&mut self, key: K) -> Option<Wanted> {
        if self.pending.as_ref().is_some_and(|(pending, _)| *pending == key) {
            return None;
        }
        let ticket = Ticket::issue();
        self.latest.store(ticket.0, Ordering::Relaxed);
        self.pending = Some((key, ticket));
        Some(Wanted { ticket, latest: self.latest.clone() })
    }

    /// An answer arrived. `Some(key)`, the request it answers, if `ticket` is the one awaited (the
    /// job is then idle again); `None` for a stale answer, which the caller drops.
    pub(super) fn finish(&mut self, ticket: Ticket) -> Option<K> {
        match self.pending.take() {
            Some((key, awaited)) if awaited == ticket => Some(key),
            other => {
                self.pending = other;
                None
            }
        }
    }
}

impl<K> Drop for Job<K> {
    /// A terminal that goes away stops the work it was waiting for.
    fn drop(&mut self) {
        self.latest.store(Ticket::NONE.0, Ordering::Relaxed);
    }
}
