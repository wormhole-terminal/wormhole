//! The events tool. MCP has no way to push a model a message of our own choosing: a server's
//! notifications are about the protocol (progress of a call, a changed tool list, logging, which the
//! newest revision deprecates), and a client does not hand them to the model. So the stream of `wh events`
//! is exposed as a bounded call, `wait_events`: it takes a cursor and waits up to a given time for what
//! came after it.
//!
//! Between two calls events must not be lost, so the first call starts a subscription (`events`, over the
//! control socket, as `wh events` does) that a thread of this process drains into a buffer of the newest
//! [`BUFFER_MAX`] events. The cursor is explicit (`seq` and `boot`, handed back by each call and passed in
//! the next), as the stateless revision asks of anything that spans calls; the buffer is only a cache of
//! the app's stream, and a cursor it cannot honour is told (`gap`) with a fresh snapshot to start from.
//! After [`IDLE_END`] without a call the subscription is dropped, giving its place among the app's streams
//! back; the next call starts it again.

use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::json;

use super::exec::{self, Endpoint};
use super::result::Outcome;
use crate::ipc::control::event::{EventFilter, EventLine, Notice, StreamLine, Subscribed};
use crate::ipc::control::events::{self, Style};
use crate::ipc::control::{Action, EventsSpec, Reply, Request};
use crate::ipc::exchange::{Caller, Cancel, asking_if_needed, exchange_stream};

/// The longest one call waits for the first event, in seconds.
pub(super) const EVENTS_WAIT_MAX: u64 = 30;
/// The most events one call returns.
pub(super) const EVENTS_PER_CALL_MAX: usize = 500;
/// How many events are kept for the next call.
const BUFFER_MAX: usize = 1000;
/// A subscription nobody has asked about for this long is ended.
const IDLE_END: Duration = Duration::from_secs(300);
/// How long a first call waits for the app to accept the subscription.
const START_WAIT: Duration = Duration::from_secs(15);
/// How often a waiting call looks at whether it was cancelled.
const LOOK: Duration = Duration::from_millis(100);

/// A `wait_events` call.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct EventsQuery {
    /// The cursor from the last call; none on the first.
    pub(super) since: Option<u64>,
    /// The run of the app the cursor belongs to.
    pub(super) boot: Option<String>,
    pub(super) timeout: Duration,
    pub(super) filter: EventFilter,
    /// Use the subscription that also carries what terminals print.
    pub(super) output: bool,
    pub(super) max_events: usize,
}

/// The subscriptions this process may hold: one for the default topics, and, for `output` (which needs the
/// right to read, or terminals that are the caller's own), one for each set of terminals asked for, since the
/// app decides the grant by the terminals a subscription names.
pub(super) struct Feeds {
    plain: Feed,
    outputs: Mutex<Vec<Arc<Feed>>>,
}

/// How many `output` subscriptions are kept (the app lets a caller hold four streams in all).
const OUTPUT_FEEDS_MAX: usize = 3;

impl Feeds {
    pub(super) fn new() -> Self {
        Self { plain: Feed::new(false, Vec::new()), outputs: Mutex::new(Vec::new()) }
    }

    /// Answer a `wait_events` call, or `None` when it was cancelled.
    pub(super) fn query(&self, at: &Endpoint, query: &EventsQuery, cancel: &Cancel) -> Option<Outcome> {
        if !query.output {
            return self.plain.query(at, query, cancel);
        }
        let mut terminals = query.filter.terminals.clone();
        terminals.sort_unstable();
        terminals.dedup();
        let feed = {
            let mut outputs = self.outputs.lock().unwrap_or_else(PoisonError::into_inner);
            match outputs.iter().find(|feed| feed.terminals == terminals) {
                Some(feed) => Arc::clone(feed),
                None => {
                    // Room is made by forgetting a watch that is not running.
                    outputs.retain(|feed| !feed.is_stopped());
                    if outputs.len() >= OUTPUT_FEEDS_MAX {
                        return Some(Outcome::failed(
                            "rate_limited",
                            format!(
                                "at most {OUTPUT_FEEDS_MAX} different sets of terminals can be watched for output at once"
                            ),
                        ));
                    }
                    let feed = Arc::new(Feed::new(true, terminals));
                    outputs.push(Arc::clone(&feed));
                    feed
                }
            }
        };
        feed.query(at, query, cancel)
    }
}

struct Feed {
    output: bool,
    /// The terminals the subscription is limited to (sorted), when it is for output.
    terminals: Vec<i64>,
    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    /// Nothing is subscribed.
    Idle,
    Starting,
    Live,
    /// The app refused or could not be reached; the next call tries again.
    Failed(Outcome),
    /// The stream ended (the app quit, we were too slow, nobody asked for a while).
    Ended(String),
}

struct State {
    phase: Phase,
    /// Which subscription the thread draining it belongs to: a thread of an earlier one lets go.
    generation: u64,
    boot: String,
    /// The seq before the subscription's first event.
    first_seq: u64,
    /// The seq of the newest event heard.
    last_seq: u64,
    events: VecDeque<EventLine>,
    /// The seq of the newest event dropped from the buffer to make room.
    evicted_through: u64,
    /// Events up to this seq may have been dropped on the way to us (`lagged`).
    lost_through: u64,
    lag_pending: bool,
    last_used: Instant,
    /// A call has told that the stream ended: the next one starts it again.
    ended_told: bool,
}

impl State {
    fn new() -> Self {
        Self {
            phase: Phase::Idle,
            generation: 0,
            boot: String::new(),
            first_seq: 0,
            last_seq: 0,
            events: VecDeque::new(),
            evicted_through: 0,
            lost_through: 0,
            lag_pending: false,
            last_used: Instant::now(),
            ended_told: false,
        }
    }

    fn push(&mut self, line: EventLine) {
        if self.lag_pending {
            self.lag_pending = false;
            self.lost_through = self.lost_through.max(line.seq.saturating_sub(1));
        }
        self.last_seq = self.last_seq.max(line.seq);
        if self.events.len() == BUFFER_MAX
            && let Some(dropped) = self.events.pop_front()
        {
            self.evicted_through = self.evicted_through.max(dropped.seq);
        }
        self.events.push_back(line);
    }

    /// Whether events after `since` may be missing from what is kept.
    fn has_gap_after(&self, since: u64) -> bool {
        since < self.first_seq || since < self.evicted_through || since < self.lost_through
    }
}

impl Feed {
    fn new(output: bool, terminals: Vec<i64>) -> Self {
        Self {
            output,
            terminals,
            shared: Arc::new(Shared { state: Mutex::new(State::new()), changed: Condvar::new() }),
        }
    }

    /// Nothing is subscribed and nothing is starting.
    fn is_stopped(&self) -> bool {
        matches!(self.shared.lock().phase, Phase::Idle | Phase::Failed(_) | Phase::Ended(_))
    }

    /// What the app is asked to stream.
    fn spec(&self) -> EventsSpec {
        EventsSpec { output: self.output, terminals: self.terminals.clone(), ..EventsSpec::default() }
    }

    fn query(&self, at: &Endpoint, query: &EventsQuery, cancel: &Cancel) -> Option<Outcome> {
        if let Err(failure) = self.ensure_live(at, cancel, query.since.is_none()) {
            return failure.map(|outcome| *outcome);
        }
        let deadline = Instant::now() + query.timeout;
        let started = Instant::now();
        let mut state = self.shared.lock();
        let mut resync = query.since.is_none();
        match (&query.boot, query.since) {
            (Some(boot), Some(_)) if *boot != state.boot => resync = true,
            (None, Some(since)) if since > state.last_seq && state.first_seq > 0 => resync = true,
            _ => {}
        }
        if resync {
            let (boot, seq) = (state.boot.clone(), state.last_seq);
            let gap = query.since.is_some();
            drop(state);
            let snapshot = snapshot(at, cancel)?;
            // Without a snapshot there is nothing to start from: the failure is the answer.
            if snapshot.is_error() {
                return Some(snapshot);
            }
            return Some(done(&boot, seq, &[], false, gap, Some(&snapshot), None, started, at));
        }
        let since = query.since.unwrap_or_default();
        loop {
            if cancel.is_cancelled() {
                return None;
            }
            state.last_used = Instant::now();
            let mut found: Vec<&EventLine> = state
                .events
                .iter()
                .filter(|line| line.seq > since && query.filter.matches(&line.event))
                .take(query.max_events + 1)
                .collect();
            let ended = match &state.phase {
                Phase::Ended(reason) => Some(reason.clone()),
                _ => None,
            };
            let now = Instant::now();
            if !found.is_empty() || ended.is_some() || now >= deadline {
                let more = found.len() > query.max_events;
                found.truncate(query.max_events);
                let seq = match (more, found.last()) {
                    (true, Some(last)) => last.seq,
                    _ => state.last_seq.max(since),
                };
                let gap = state.has_gap_after(since);
                let lines: Vec<EventLine> = found.into_iter().cloned().collect();
                let boot = state.boot.clone();
                state.ended_told |= ended.is_some();
                drop(state);
                // (A snapshot the app would not give leaves the events themselves, and `gap`, to tell.)
                let snapshot = if gap { snapshot(at, cancel).filter(|s| !s.is_error()) } else { None };
                return Some(done(&boot, seq, &lines, more, gap, snapshot.as_ref(), ended, started, at));
            }
            let wait = (deadline - now).min(LOOK);
            state = self.shared.changed.wait_timeout(state, wait).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    /// Start the subscription if there is none, and wait for the app to accept it. The error is the call's
    /// whole answer (`None` when it was cancelled).
    fn ensure_live(&self, at: &Endpoint, cancel: &Cancel, fresh: bool) -> Result<(), Option<Box<Outcome>>> {
        let mut state = self.shared.lock();
        state.last_used = Instant::now();
        // A stream that ended is told of once to a caller who has a cursor in it (the cursor is why it
        // matters); a caller with none, or one that was told, gets a new one.
        let restart = match state.phase {
            Phase::Idle | Phase::Failed(_) => true,
            Phase::Ended(_) => fresh || state.ended_told,
            Phase::Starting | Phase::Live => false,
        };
        if restart {
            let generation = state.generation + 1;
            let last_used = state.last_used;
            *state = State::new();
            state.generation = generation;
            state.last_used = last_used;
            state.phase = Phase::Starting;
            let shared = Arc::clone(&self.shared);
            let (caller, spec) = (at.caller.clone(), self.spec());
            let spawned = std::thread::Builder::new()
                .name("wormhole-mcp-events".into())
                .spawn(move || pump(&shared, &caller, spec, generation));
            if spawned.is_err() {
                state.phase = Phase::Idle;
                return Err(Some(Box::new(Outcome::failed(
                    "failed",
                    "couldn't start the thread that listens to Wormhole",
                ))));
            }
        }
        let give_up = Instant::now() + START_WAIT;
        while state.phase == Phase::Starting {
            if cancel.is_cancelled() {
                return Err(None);
            }
            if Instant::now() >= give_up {
                state.phase = Phase::Idle;
                return Err(Some(Box::new(Outcome::failed(
                    "unavailable",
                    "the running Wormhole didn't answer",
                ))));
            }
            state = self.shared.changed.wait_timeout(state, LOOK).unwrap_or_else(PoisonError::into_inner).0;
        }
        if let Phase::Failed(outcome) = state.phase.clone() {
            state.phase = Phase::Idle;
            return Err(Some(Box::new(outcome)));
        }
        Ok(())
    }
}

/// A fresh `list`, to start from (`None` when the call was cancelled; its outcome is an error if the app
/// would not say).
fn snapshot(at: &Endpoint, cancel: &Cancel) -> Option<Outcome> {
    let outcome = exec::run(at, Action::List, cancel);
    (!cancel.is_cancelled()).then_some(outcome)
}

/// The thread that drains one subscription into the buffer.
fn pump(shared: &Shared, caller: &Caller, spec: EventsSpec, generation: u64) {
    let request = asking_if_needed(Request::new(Action::Events(spec), caller.from.clone()));
    let current = |state: &State| state.generation == generation;
    let on_reply = |_: &str, reply: Reply| {
        let mut state = shared.lock();
        if !current(&state) {
            return ControlFlow::Break(0);
        }
        let outcome = Outcome::of_reply(&request.action, &reply, None);
        let subscribed = reply.result.clone().and_then(|r| serde_json::from_value::<Subscribed>(r).ok());
        match (outcome.is_error(), subscribed) {
            (false, Some(subscribed)) => {
                state.boot = subscribed.boot;
                state.first_seq = subscribed.seq;
                state.last_seq = subscribed.seq;
                state.phase = Phase::Live;
                shared.changed.notify_all();
                ControlFlow::Continue(())
            }
            (false, None) => {
                state.phase = Phase::Failed(Outcome::failed(
                    "failed",
                    "Wormhole's answer to the subscription was unreadable",
                ));
                shared.changed.notify_all();
                ControlFlow::Break(1)
            }
            (true, _) => {
                state.phase = Phase::Failed(outcome);
                shared.changed.notify_all();
                ControlFlow::Break(1)
            }
        }
    };
    let on_line = |text: &str| {
        let mut state = shared.lock();
        if !current(&state) {
            return ControlFlow::Break(0);
        }
        if state.last_used.elapsed() > IDLE_END {
            state.phase = Phase::Ended("idle".into());
            shared.changed.notify_all();
            return ControlFlow::Break(0);
        }
        match StreamLine::parse(text) {
            Some(StreamLine::Event(line)) => {
                state.push(line);
                shared.changed.notify_all();
            }
            Some(StreamLine::Notice(Notice::Lagged { .. })) => state.lag_pending = true,
            Some(StreamLine::Notice(Notice::Closed { reason })) => {
                let word = serde_json::to_value(reason)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_else(|| "closed".into());
                state.phase = Phase::Ended(word);
                shared.changed.notify_all();
                return ControlFlow::Break(0);
            }
            Some(StreamLine::Notice(Notice::Heartbeat { .. })) | None => {}
        }
        ControlFlow::Continue(())
    };
    let end = exchange_stream(&caller.socket, &request, on_reply, on_line);
    let mut state = shared.lock();
    if !current(&state) {
        return;
    }
    let phase = state.phase.clone();
    match (phase, end) {
        (Phase::Starting, Err(failure)) => {
            state.phase = Phase::Failed(Outcome::failed("unavailable", failure.words()));
        }
        (Phase::Starting | Phase::Live, end) => {
            let why = match end {
                Err(failure) => failure.words(),
                Ok(_) => "the connection to Wormhole ended".to_string(),
            };
            state.phase = Phase::Ended(why);
        }
        _ => {}
    }
    shared.changed.notify_all();
}

/// The call's answer.
#[allow(clippy::too_many_arguments, reason = "the answer's fields, put together in one place")]
fn done(
    boot: &str,
    seq: u64,
    lines: &[EventLine],
    more: bool,
    gap: bool,
    snapshot: Option<&Outcome>,
    closed: Option<String>,
    started: Instant,
    at: &Endpoint,
) -> Outcome {
    let mut result = json!({
        "boot": boot,
        "seq": seq,
        "events": lines,
        "more": more,
        "gap": gap,
        "waited_ms": started.elapsed().as_millis() as u64,
    });
    if let Some(structured) = snapshot.and_then(|snapshot| snapshot.structured.as_ref()) {
        result["snapshot"] = structured.clone();
    }
    if let Some(reason) = &closed {
        result["closed"] = json!(reason);
    }
    let mut text = format!("{} events, cursor seq {seq}", lines.len());
    if more {
        text.push_str(" (more are waiting: call again at once)");
    }
    if gap {
        text.push_str("; events may have been lost before the first one here, the snapshot is the state now");
    }
    if let Some(reason) = &closed {
        text.push_str(&format!("; the event stream ended ({reason}), the next call starts it again"));
    }
    text.push('\n');
    let style = Style { color: false };
    for line in lines {
        if let Some(shown) = events::line(&StreamLine::Event(line.clone()), at.home.as_deref(), style) {
            text.push_str(&shown);
        }
    }
    if let Some(snapshot) = snapshot {
        text.push_str("\nnow:\n");
        text.push_str(&snapshot.text);
    }
    Outcome::done(text.trim_end(), result)
}
