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
mod wait;
mod worker;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use ring::rand::{SecureRandom as _, SystemRandom};

use subscription::Subscriber;
pub use subscription::{Caller, Grant, Next, QUEUE_BYTES, QUEUE_ITEMS, Subscription};
pub use wait::{LOOK_EVERY, RunClaim, Watch};
use worker::Worker;
pub use worker::{OUTPUT_BYTES_MAX, OUTPUT_LINES_MAX};

use super::control::event::{Body, CloseReason, Event, EventFilter, EventLine, Topic};
use super::control::{PROTOCOL_VERSION, printable};

/// Streams open at once, for every caller together: each is a thread.
pub const STREAMS_MAX: usize = 16;
/// Streams open at once for one caller, so one agent's loop can't take them all.
pub const STREAMS_PER_CALLER: usize = 4;

/// The bit of the interest mask for waiters that look at a screen: no topic, since nothing is
/// published for it, only a wake-up (`Bus::pulse`). Above the topics' bits.
const SCREEN_BIT: u32 = 1 << 30;

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
    /// The terminals the user hid from agents ("Hide from Agents"): nothing about them is told to a
    /// subscriber that does not run inside them.
    hidden: HashSet<i64>,
    /// The terminals a `run` is under way in (`Bus::claim_run`).
    runs: Vec<i64>,
}

/// The bus.
#[derive(Debug)]
pub struct Bus {
    /// The topics somebody listens to, a bit each: what publishers read first.
    interest: AtomicU32,
    state: Mutex<State>,
    boot: String,
    /// Reads what terminals print for the `output` topic; idle until somebody asks for it.
    worker: Arc<Worker>,
}

impl Bus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            interest: AtomicU32::new(0),
            state: Mutex::new(State::default()),
            boot: random_boot(),
            worker: Arc::default(),
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

    fn deliver(&self, mut event: Event) {
        let mut state = self.lock();
        // A hidden terminal is not in a split's list either; a split of nothing else is not told at all.
        if let Body::Split { terminals, focused, .. } = &mut event.body
            && !state.hidden.is_empty()
        {
            let had = terminals.len();
            terminals.retain(|id| !state.hidden.contains(id));
            if focused.is_some_and(|id| state.hidden.contains(&id)) {
                *focused = None;
            }
            if terminals.is_empty() && had > 0 {
                return;
            }
        }
        // What happens in a hidden terminal is told only to a subscriber that runs inside it.
        let hidden = event.terminal.filter(|id| state.hidden.contains(id));
        // Text a terminal printed goes only to those who may read that terminal, now.
        let text = event.is_terminal_text();
        let listeners: Vec<Arc<Subscriber>> = state
            .subscribers
            .iter()
            .filter(|s| {
                s.filter.matches(&event)
                    && hidden.is_none_or(|id| s.is_inside(id))
                    && (!text || s.may_read(event.terminal))
            })
            .cloned()
            .collect();
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
        self.register(filter, None, grant, caller)
    }

    /// [`subscribe`](Self::subscribe), for a waiter when `watch` is given.
    fn register(
        self: &Arc<Self>,
        filter: EventFilter,
        watch: Option<Watch>,
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
        let subscriber = Arc::new(Subscriber::new(state.next_id, caller, filter, watch, grant));
        state.subscribers.push(subscriber.clone());
        self.interest.store(interest_of(&state), Ordering::Relaxed);
        let started = Started { seq: state.seq, boot: self.boot.clone() };
        Ok((Subscription::new(self.clone(), subscriber, state.seq), started))
    }

    /// Whether some stream asked for what `terminal` of `window` prints and may read it. Read by the
    /// worker and by [`pulse`](Self::pulse) after the cheap check of the topic's bit.
    pub(crate) fn output_wanted(&self, window: i64, terminal: i64) -> bool {
        let state = self.lock();
        let hidden = state.hidden.contains(&terminal);
        state
            .subscribers
            .iter()
            .any(|s| s.wants_output_of(window, terminal) && (!hidden || s.is_inside(terminal)))
    }

    /// The user hid `terminal` from agents, or showed it again. While it is hidden nothing about it is told
    /// to a stream of someone who does not run inside it, and a wait on it that is under way ends
    /// (`not_allowed`) as if what let it read were taken back.
    pub fn set_hidden(&self, terminal: i64, hidden: bool) {
        let mut state = self.lock();
        if !hidden {
            state.hidden.remove(&terminal);
            return;
        }
        state.hidden.insert(terminal);
        let subscribers = state.subscribers.clone();
        drop(state);
        for subscriber in subscribers {
            if subscriber.watch.is_some_and(|watch| watch.terminal == terminal)
                && !subscriber.is_inside(terminal)
            {
                subscriber.queue.close(CloseReason::NotAllowed);
            }
        }
    }

    /// `terminal` is closed: nothing is kept about it.
    pub fn forget_terminal(&self, terminal: i64) {
        self.lock().hidden.remove(&terminal);
    }

    /// Terminal `terminal` of `window` printed. With nobody asking for output (one relaxed load) this
    /// does nothing; otherwise it notes it for the worker, which looks at the terminal soon, off every
    /// thread the program and the window run on. `reader` is called only when the note is new.
    pub fn pulse(
        self: &Arc<Self>,
        window: i64,
        terminal: i64,
        reader: impl FnOnce() -> Option<wormhole_term::WeakScreenReader>,
    ) {
        let interest = self.interest_bits();
        if interest & SCREEN_BIT != 0 {
            self.poke_screens(terminal);
        }
        if interest & Topic::Output.bit() != 0 && self.output_wanted(window, terminal) {
            self.worker.pulse(self, window, terminal, reader);
        }
    }

    fn interest_bits(&self) -> u32 {
        self.interest.load(Ordering::Relaxed)
    }

    /// Start reading `terminal`'s output from where it is now: called for the terminals a new stream
    /// watches, so what is printed right after the subscription is told. Only if somebody asks.
    pub fn watch_output(&self, window: i64, terminal: i64, reader: &wormhole_term::ScreenReader) {
        if self.output_wanted(window, terminal) {
            self.worker.track(window, terminal, reader);
        }
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

    /// An approval was taken back (the user pressed Revoke, or the program's session ended): the streams
    /// that were started under it are closed (`not_allowed`).
    pub fn revoke_holder(&self, holder: u64) {
        let subscribers = self.lock().subscribers.clone();
        for subscriber in subscribers {
            subscriber.revoke_holder(holder);
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
        let interest = interest_of(&state);
        let output_gone = self.interest.swap(interest, Ordering::Relaxed) & Topic::Output.bit() != 0
            && interest & Topic::Output.bit() == 0;
        if output_gone {
            self.worker.reset();
        }
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        self.worker.close();
    }
}

fn interest_of(state: &State) -> u32 {
    state.subscribers.iter().fold(0, |bits, s| {
        let screen = if s.watch.is_some_and(|watch| watch.screen) { SCREEN_BIT } else { 0 };
        bits | s.filter.topics.bits() | screen
    })
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
mod output_tests;
#[cfg(test)]
mod tests;
