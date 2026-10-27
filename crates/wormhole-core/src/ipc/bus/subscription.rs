//! One client's place on the bus: what it asked for, what it may read, its queue, and the handle
//! that gives the place back when it is dropped.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use super::queue::{Pop, Queue};
use super::wait::Watch;
use super::{Bus, Published};
use crate::ipc::control::event::{CloseReason, EventFilter, Topic};

/// Items one subscriber's queue keeps before it drops its oldest.
pub const QUEUE_ITEMS: usize = 2048;
/// Bytes of event lines one subscriber's queue keeps before it drops its oldest.
pub const QUEUE_BYTES: usize = 4 * 1024 * 1024;

/// Who asked: a terminal's id (`from`), or `None` for a shell outside the app.
pub type Caller = Option<i64>;

/// What a subscriber may read of terminals' text (the `command` of a block, and `output`): a
/// terminal can always read itself, and any terminal can be read while `control_read` is on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grant {
    /// Every terminal, by the setting.
    pub reads_all: bool,
    /// The terminals the caller runs in, by its place in the process tree.
    pub own: Vec<i64>,
}

impl Grant {
    /// Whether the text of `terminal` may be told to the subscriber.
    pub fn can_read(&self, terminal: Option<i64>) -> bool {
        self.reads_all || terminal.is_some_and(|id| self.own.contains(&id))
    }
}

/// The bus's side of a subscription.
#[derive(Debug)]
pub(super) struct Subscriber {
    pub(super) id: u64,
    pub(super) caller: Caller,
    pub(super) filter: EventFilter,
    /// Set for a `wait` or `run`: the terminal it looks at the screen of.
    pub(super) watch: Option<Watch>,
    grant: Mutex<Grant>,
    pub(super) queue: Queue<Arc<Published>>,
}

impl Subscriber {
    pub(super) fn new(
        id: u64,
        caller: Caller,
        filter: EventFilter,
        watch: Option<Watch>,
        grant: Grant,
    ) -> Self {
        Self {
            id,
            caller,
            filter,
            watch,
            grant: Mutex::new(grant),
            queue: Queue::new(QUEUE_ITEMS, QUEUE_BYTES),
        }
    }

    pub(super) fn grant(&self) -> Grant {
        self.grant.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Whether it may be told `terminal`'s text now.
    pub(super) fn may_read(&self, terminal: Option<i64>) -> bool {
        self.grant.lock().unwrap_or_else(PoisonError::into_inner).can_read(terminal)
    }

    /// Whether what `terminal` in `window` prints is for it: it asked for `output` from that terminal
    /// and window, and may read it.
    pub(super) fn wants_output_of(&self, window: i64, terminal: i64) -> bool {
        let filter = &self.filter;
        filter.topics.contains(Topic::Output)
            && (filter.terminals.is_empty() || filter.terminals.contains(&terminal))
            && (filter.windows.is_empty() || filter.windows.contains(&window))
            && self.may_read(Some(terminal))
    }

    /// The setting that let it read everything was turned off: it keeps what it can read by being
    /// in a terminal, and a stream that needed more is closed.
    pub(super) fn revoke_reads(&self) {
        let mut grant = self.grant.lock().unwrap_or_else(PoisonError::into_inner);
        if !grant.reads_all {
            return;
        }
        grant.reads_all = false;
        let only_own = !self.filter.terminals.is_empty()
            && self.filter.terminals.iter().all(|terminal| grant.own.contains(terminal));
        // A wait on a screen's text goes on only for a terminal its caller runs in.
        let wait_needs =
            self.watch.is_some_and(|watch| watch.needs_read && !grant.own.contains(&watch.terminal));
        if (self.filter.topics.contains(Topic::Output) && !only_own) || wait_needs {
            drop(grant);
            self.queue.close(CloseReason::NotAllowed);
        }
    }
}

/// What the next wait on a subscription ends with.
#[derive(Debug)]
pub enum Next {
    /// An event to write.
    Event(Arc<Published>),
    /// This many events were dropped for lack of room, before the next.
    Lagged(u64),
    /// The stream is over, and why.
    Closed(CloseReason),
    /// The terminal a waiter watches printed since it last looked (any number of times is one).
    Changed,
    /// Nothing happened in the time given.
    Idle,
}

/// A client's subscription. Dropping it gives its place back to the bus.
pub struct Subscription {
    bus: Arc<Bus>,
    subscriber: Arc<Subscriber>,
    last_seq: AtomicU64,
}

impl Subscription {
    pub(super) fn new(bus: Arc<Bus>, subscriber: Arc<Subscriber>, seq: u64) -> Self {
        Self { bus, subscriber, last_seq: AtomicU64::new(seq) }
    }

    /// The next thing to tell, waiting up to `wait` for it.
    pub fn next(&self, wait: Duration) -> Next {
        match self.subscriber.queue.pop(wait) {
            Pop::Item(published) => {
                self.last_seq.store(published.seq, Ordering::Relaxed);
                Next::Event(published)
            }
            Pop::Lagged(dropped) => Next::Lagged(dropped),
            Pop::Closed(reason) => Next::Closed(reason),
            Pop::Poked => Next::Changed,
            Pop::Empty => Next::Idle,
        }
    }

    /// The line to write for `published`: with the terminal's text only when this subscriber may
    /// read that terminal now.
    pub fn line<'a>(&self, published: &'a Published) -> &'a str {
        let granted =
            published.event.has_gated_fields() && self.subscriber.grant().can_read(published.event.terminal);
        published.line(granted)
    }

    /// Whether the text of `terminal` may be read on this subscription's say now: the setting is on,
    /// or its caller runs in it. Asked when the text is about to be read, so a revocation counts.
    pub fn may_read(&self, terminal: i64) -> bool {
        self.subscriber.may_read(Some(terminal))
    }

    /// The seq of the last event handed out, or the one the subscription started after.
    pub fn last_seq(&self) -> u64 {
        self.last_seq.load(Ordering::Relaxed)
    }

    /// End the subscription from the bus's side (the next wait says `reason`).
    pub fn close(&self, reason: CloseReason) {
        self.subscriber.queue.close(reason);
    }

    /// Who asked.
    pub fn caller(&self) -> Caller {
        self.subscriber.caller
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.bus.unregister(self.subscriber.id);
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.subscriber.id)
            .field("caller", &self.subscriber.caller)
            .field("last_seq", &self.last_seq())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::Grant;

    #[test]
    fn a_terminal_can_always_read_itself_and_the_setting_reads_all() {
        let own = Grant { reads_all: false, own: vec![3] };
        assert!(own.can_read(Some(3)));
        assert!(!own.can_read(Some(4)) && !own.can_read(None));
        let all = Grant { reads_all: true, own: vec![] };
        assert!(all.can_read(Some(4)) && all.can_read(None));
        assert!(!Grant::default().can_read(Some(1)));
    }
}
