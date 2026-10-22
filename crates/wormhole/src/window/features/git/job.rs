//! Bookkeeping for results that arrive later.
//!
//! Git work runs on the background executor and its result is posted back to
//! the UI thread some time after it was asked for. By then the question may
//! have changed: another file opened, another branch compared, another session in
//! front. A [`Job`] remembers the one request a view is still waiting for, and
//! a result is applied only if it carries that request's [`Ticket`]; anything
//! else is stale and dropped, however late it lands.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Names one request. Numbers come from a single process-wide counter, so a
/// ticket is never issued twice: not to another view, not after a state was
/// replaced by a fresh one (a new `DiffView`, a reset history), not in another
/// window. A counter per owner would restart at 1 with the owner and let a late
/// result of the old one pass for the new one's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Ticket(u64);

impl Ticket {
    /// A ticket nobody holds: it never matches a request.
    pub(crate) const NONE: Ticket = Ticket(0);

    pub(crate) fn issue() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// What the work itself holds: its ticket, and whether anyone still waits for
/// it. Long work checks [`Self::still_wanted`] between its steps and stops
/// early, so a superseded request does not keep a core busy for nothing.
#[derive(Clone)]
pub(crate) struct Wanted {
    ticket: Ticket,
    latest: Arc<AtomicU64>,
}

impl Wanted {
    pub(crate) fn ticket(&self) -> Ticket {
        self.ticket
    }

    /// False once the job started another request, was cancelled or was dropped.
    pub(crate) fn still_wanted(&self) -> bool {
        self.latest.load(Ordering::Relaxed) == self.ticket.0
    }
}

/// The request a view is waiting for: what it asked (`key`) and the ticket the
/// answer must carry. One request at a time; a new one supersedes the old.
pub(crate) struct Job<K> {
    pending: Option<(K, Ticket)>,
    latest: Arc<AtomicU64>,
}

impl<K> Default for Job<K> {
    fn default() -> Self {
        Self { pending: None, latest: Arc::default() }
    }
}

impl<K> Job<K> {
    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn pending(&self) -> Option<&K> {
        self.pending.as_ref().map(|(key, _)| key)
    }

    /// Nobody waits for the answer any more; whatever arrives is dropped.
    pub(crate) fn cancel(&mut self) {
        self.pending = None;
        self.latest.store(Ticket::NONE.0, Ordering::Relaxed);
    }
}

impl<K: PartialEq> Job<K> {
    /// Ask for `key`. `None` when the same request is already in flight: its
    /// answer will do, so nothing is started again. Otherwise the request
    /// supersedes whatever was pending, and the returned [`Wanted`] goes with
    /// the work.
    pub(crate) fn begin(&mut self, key: K) -> Option<Wanted> {
        if self.pending().is_some_and(|pending| *pending == key) {
            return None;
        }
        let ticket = Ticket::issue();
        self.latest.store(ticket.0, Ordering::Relaxed);
        self.pending = Some((key, ticket));
        Some(Wanted { ticket, latest: self.latest.clone() })
    }

    /// An answer arrived. `Some(key)`, the request it answers, if `ticket` is
    /// the one awaited (the job is then idle again); `None` for a stale answer,
    /// which the caller drops.
    pub(crate) fn finish(&mut self, ticket: Ticket) -> Option<K> {
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
    /// A view that goes away stops the work it was waiting for.
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! A stand-in for the background executor: work runs when it is spawned, but
    //! its completion is held until the test delivers it, in any order. That is
    //! all a result arriving late takes, so stale-result rules can be tested
    //! without a window.

    /// A completion waiting to be posted back, taken once it is.
    type Delivery<S> = Option<Box<dyn FnOnce(&mut S)>>;

    pub(crate) struct ManualExecutor<S> {
        queue: Vec<Delivery<S>>,
    }

    impl<S> Default for ManualExecutor<S> {
        fn default() -> Self {
            Self { queue: Vec::new() }
        }
    }

    impl<S> ManualExecutor<S> {
        /// Run `work` now (the part that happens off the UI thread) and hold
        /// `done`, which would run on the UI thread when the result is posted back.
        pub(crate) fn spawn<T: 'static>(
            &mut self,
            work: impl FnOnce() -> T,
            done: impl FnOnce(&mut S, T) + 'static,
        ) {
            let result = work();
            self.queue.push(Some(Box::new(move |state| done(state, result))));
        }

        pub(crate) fn pending(&self) -> usize {
            self.queue.iter().flatten().count()
        }

        /// Post back the `ix`-th spawned work's result (in spawn order, from 0).
        pub(crate) fn deliver(&mut self, ix: usize, state: &mut S) {
            if let Some(done) = self.queue.get_mut(ix).and_then(Option::take) {
                done(state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Job, Ticket};

    #[test]
    fn an_identical_request_in_flight_is_not_started_again() {
        let mut job = Job::default();
        assert!(job.begin("a").is_some());
        assert!(job.begin("a").is_none());
        assert!(job.is_pending());
    }

    #[test]
    fn a_different_request_supersedes_the_pending_one() {
        let mut job = Job::default();
        let a = job.begin("a").expect("started");
        let b = job.begin("b").expect("started");
        assert_ne!(a.ticket(), b.ticket());
        // The late answer of the old request is stale and must not clear the new wait.
        assert_eq!(job.finish(a.ticket()), None);
        assert_eq!(job.pending(), Some(&"b"));
        assert_eq!(job.finish(b.ticket()), Some("b"));
    }

    #[test]
    fn finishing_makes_the_job_idle() {
        let mut job = Job::default();
        let a = job.begin(1).expect("started");
        assert_eq!(job.finish(a.ticket()), Some(1));
        assert!(!job.is_pending());
        // An answer is taken once; a duplicate is stale.
        assert_eq!(job.finish(a.ticket()), None);
        // And the same request can be asked again.
        assert!(job.begin(1).is_some());
    }

    #[test]
    fn tickets_are_never_shared_between_jobs() {
        let mut first = Job::default();
        let mut second = Job::default();
        let old = first.begin("same").expect("started");
        let new = second.begin("same").expect("started");
        assert_ne!(old.ticket(), new.ticket());
        // The new owner (a fresh view after the old one was replaced) ignores the old one's answer.
        assert_eq!(second.finish(old.ticket()), None);
        assert_eq!(second.finish(new.ticket()), Some("same"));
    }

    #[test]
    fn nothing_matches_the_unissued_ticket() {
        let mut job = Job::default();
        job.begin(1);
        assert_eq!(job.finish(Ticket::NONE), None);
        assert_eq!(job.finish(Ticket::default()), None);
        assert!(job.is_pending());
    }

    #[test]
    fn work_notices_when_it_was_superseded_cancelled_or_orphaned() {
        let mut job = Job::default();
        let a = job.begin(1).expect("started");
        assert!(a.still_wanted());
        let b = job.begin(2).expect("started");
        assert!(!a.still_wanted() && b.still_wanted());
        job.cancel();
        assert!(!b.still_wanted());
        assert_eq!(job.finish(b.ticket()), None);

        let c = job.begin(3).expect("started");
        drop(job);
        assert!(!c.still_wanted());
    }
}
