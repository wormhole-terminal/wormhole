//! The event bus: what happens in the app, told to the clients that watch.
//!
//! Publishers (the app's windows, on its UI thread) call [`Bus::wants`] and [`Bus::publish`]; with
//! nobody subscribed the first is one atomic load and the second does nothing, so no event is built
//! and nothing is allocated. Subscribers ([`Subscription`]) each have a bounded queue that drops its
//! oldest items, so publishing never waits for a client, however slow it is. A stream thread
//! (`ipc::server::stream`) reads a subscription and writes the lines.
//!
//! The bus is core's and knows no GPUI: the app gives it events (`wormhole::automation`).

mod queue;
mod subscription;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use ring::rand::{SecureRandom as _, SystemRandom};

use subscription::Subscriber;
pub use subscription::{Caller, Grant, Next, QUEUE_BYTES, QUEUE_ITEMS, Subscription};

use super::control::event::{CloseReason, Event, EventFilter, EventLine, Topic};
use super::control::{PROTOCOL_VERSION, printable};

/// Streams open at once, for every caller together: each is a thread.
pub const STREAMS_MAX: usize = 16;
/// Streams open at once for one caller, so one agent's loop can't take them all.
pub const STREAMS_PER_CALLER: usize = 4;

/// An event, numbered and dated, written once for those who may not read the terminal's text and,
/// when it has some, once for those who may.
#[derive(Debug)]
pub struct Published {
    pub seq: u64,
    pub at_ms: u64,
    pub event: Event,
    public: String,
    full: OnceLock<String>,
}

impl Published {
    fn new(seq: u64, at_ms: u64, event: Event) -> Self {
        let shown = if event.has_gated_fields() { event.without_gated_fields() } else { event.clone() };
        let public = printable::json_line(&EventLine { version: PROTOCOL_VERSION, seq, at_ms, event: shown });
        Self { seq, at_ms, event, public, full: OnceLock::new() }
    }

    /// The line to write, without its newline: with the terminal's text when `granted`.
    pub fn line(&self, granted: bool) -> &str {
        if !granted || !self.event.has_gated_fields() {
            return &self.public;
        }
        self.full.get_or_init(|| {
            printable::json_line(&EventLine {
                version: PROTOCOL_VERSION,
                seq: self.seq,
                at_ms: self.at_ms,
                event: self.event.clone(),
            })
        })
    }
}

/// Where a new subscription starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The seq of the last event before it: everything it hears is later.
    pub seq: u64,
    /// Random for each run of the app.
    pub boot: String,
}

/// Why a subscription was not made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// [`STREAMS_MAX`] streams are open.
    Full,
    /// The caller has [`STREAMS_PER_CALLER`] open.
    CallerFull,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Full => write!(f, "{STREAMS_MAX} streams are open"),
            Refused::CallerFull => write!(f, "this caller has {STREAMS_PER_CALLER} streams open"),
        }
    }
}

#[derive(Debug, Default)]
struct State {
    seq: u64,
    next_id: u64,
    subscribers: Vec<Arc<Subscriber>>,
}

/// The bus.
#[derive(Debug)]
pub struct Bus {
    /// The topics somebody listens to, a bit each: what publishers read first.
    interest: AtomicU32,
    state: Mutex<State>,
    boot: String,
}

impl Bus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            interest: AtomicU32::new(0),
            state: Mutex::new(State::default()),
            boot: random_boot(),
        })
    }

    /// Whether somebody listens for `topic`: one relaxed load, so a publisher asks before it builds
    /// anything.
    pub fn wants(&self, topic: Topic) -> bool {
        self.interest.load(Ordering::Relaxed) & topic.bit() != 0
    }

    /// Tell `topic`'s listeners what `build` makes. `build` runs only when somebody listens.
    pub fn publish(&self, topic: Topic, build: impl FnOnce() -> Event) {
        if !self.wants(topic) {
            return;
        }
        let event = build();
        debug_assert_eq!(event.body.topic(), Some(topic), "published under the wrong topic");
        self.deliver(event);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn deliver(&self, event: Event) {
        let mut state = self.lock();
        let listeners: Vec<Arc<Subscriber>> =
            state.subscribers.iter().filter(|s| s.filter.matches(&event)).cloned().collect();
        if listeners.is_empty() {
            return;
        }
        // The number is taken under the lock that also hands the event to every queue, so all see
        // one order.
        state.seq += 1;
        let published = Arc::new(Published::new(state.seq, wormhole_term::hosted::now_ms(), event));
        for listener in listeners {
            listener.queue.push(published.clone(), published.public.len());
        }
    }

    /// Make a subscription for `caller` to what `filter` lets through, with what it may read. The
    /// seq it starts after is read in the same step, so a snapshot taken by the caller before it
    /// publishes anything more is exact.
    pub fn subscribe(
        self: &Arc<Self>,
        filter: EventFilter,
        grant: Grant,
        caller: Caller,
    ) -> Result<(Subscription, Started), Refused> {
        let mut state = self.lock();
        if state.subscribers.len() >= STREAMS_MAX {
            return Err(Refused::Full);
        }
        if state.subscribers.iter().filter(|s| s.caller == caller).count() >= STREAMS_PER_CALLER {
            return Err(Refused::CallerFull);
        }
        state.next_id += 1;
        let subscriber = Arc::new(Subscriber::new(state.next_id, caller, filter, grant));
        state.subscribers.push(subscriber.clone());
        self.interest.store(interest_of(&state), Ordering::Relaxed);
        let started = Started { seq: state.seq, boot: self.boot.clone() };
        Ok((Subscription::new(self.clone(), subscriber, state.seq), started))
    }

    /// How many subscriptions there are.
    pub fn subscribers(&self) -> usize {
        self.lock().subscribers.len()
    }

    /// The setting that lets the command line read every terminal went off: streams that needed it
    /// are closed (`not_allowed`); what a terminal reads of itself stays.
    pub fn revoke_reads(&self) {
        let subscribers = self.lock().subscribers.clone();
        for subscriber in subscribers {
            subscriber.revoke_reads();
        }
    }

    /// End every stream (the app is quitting).
    pub fn close_all(&self, reason: CloseReason) {
        let subscribers = self.lock().subscribers.clone();
        for subscriber in subscribers {
            subscriber.queue.close(reason);
        }
    }

    pub(super) fn unregister(&self, id: u64) {
        let mut state = self.lock();
        state.subscribers.retain(|s| s.id != id);
        self.interest.store(interest_of(&state), Ordering::Relaxed);
    }
}

fn interest_of(state: &State) -> u32 {
    state.subscribers.iter().fold(0, |bits, s| bits | s.filter.topics.bits())
}

/// A name for this run of the app: 16 hex digits from the system's random source.
fn random_boot() -> String {
    let mut bytes = [0u8; 8];
    if SystemRandom::new().fill(&mut bytes).is_err() {
        // Without a random source the clock still tells two runs apart.
        bytes = wormhole_term::hosted::now_ms().to_be_bytes();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
