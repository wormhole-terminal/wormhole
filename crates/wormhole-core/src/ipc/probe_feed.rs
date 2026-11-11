//! What the Probes' hooks hear: a subscription of its own on the event bus, for exactly the topics some hook
//! listens to, read on a thread (`wormhole-probe-hooks`) that hands each event over a bounded channel to the
//! shell, and the step that turns what arrives into [`Effect`]s (`active::Dispatcher`: pure, microseconds,
//! under each hook's budget). The shell reads the channel where it may carry the effects out (an executor task,
//! a thread of its own) and does so; nothing runs in a frame, no hook can block, rewrite or answer anything a
//! terminal does, and a hook that fails is a hook that did nothing.
//!
//! What a terminal prints reaches `output.match` hooks the way it reaches `wh events --output`: the bus's own
//! worker reads the screen off the PTY's thread (the shell registers each terminal's reader with
//! `Bus::watch_output` when [`Listener::output`] says a hook wants lines) and the finished lines are told to a
//! subscriber that may read the terminals. Only a Probe the user let read them (`terminal = ["read"]`) makes the
//! feed one, and the dispatcher still hands each Probe only what its own grant allows.

use std::sync::Arc;
use std::time::Instant;

use smol::channel::{Receiver, Sender, bounded};

use super::bus::{Bus, Grant, Next, Subscription};
use super::control::event::{
    Body, CloseReason, EventFilter, HEARTBEAT, Printed, Topic, hook_event, hook_topics,
};
use crate::probe::active::{self, Dispatcher, Effect, RunEnd, RunJob, RunTicket};
use crate::probe::events::{EventData, Value};
use crate::probe::manifest::HookEvent;
use crate::storage::paths::DataRoot;

/// Events handed from the bus's thread to the shell that are waiting at most. A flood is dropped here, as the
/// bus's own queue drops it, never waited for.
const PENDING: usize = 256;
/// The caller of this subscription on the bus: no terminal has this id, so it takes no share of the streams a
/// shell outside the app (`None`) may open.
const CALLER: i64 = i64::MIN;
/// The approval the feed is under on the bus: the Probes' own, which the command line's setting does not end
/// (`Subscription`'s `holder`). No other approval has this number.
const HOLDER: u64 = u64::MAX;

/// What the shell's side of the feed hears: an event (several lines at once, for printed output) and the end of a
/// program a hook started.
pub enum Feed {
    Event(HookEvent, Vec<EventData>),
    Ended(RunTicket, RunEnd),
}

/// What the app knows of a terminal that a hook's `match` may test (`front`, `kind`) or a template name
/// (`folder_name`): not payload, so output cannot forge it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabFacts {
    /// The terminal is the key one of a window in front.
    pub front: bool,
    /// A coding agent holds it (`kind = "agent"`), else a shell.
    pub agent: bool,
    pub name: String,
}

/// The hook feed: dropping it ends the subscription, so the thread's next wait says so and it ends, and closes the
/// channel, so whoever reads it (which may hold a sender for the programs it starts) stops waiting.
pub struct Listener {
    subscription: Arc<Subscription>,
    feed: Receiver<Feed>,
    /// Where a program a hook started reports its end ([`start_run`]).
    pub sender: Sender<Feed>,
    /// Some hook listens to what terminals print: register each terminal's reader with `Bus::watch_output`.
    pub output: bool,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.subscription.close(CloseReason::Quitting);
        self.feed.close();
    }
}

/// Subscribe to the topics `dispatcher`'s hooks listen to, and start the thread that fills the channel.
/// `None` when no hook listens, or the bus or a thread cannot be had (logged).
pub fn listen(bus: &Arc<Bus>, dispatcher: &Dispatcher) -> Option<(Listener, Receiver<Feed>)> {
    let topics = hook_topics(dispatcher.events().iter().copied());
    if topics.is_empty() {
        return None;
    }
    // Text reaches the feed only when some Probe may be told it, and then each Probe is handed only what its
    // own grant allows (`Dispatcher::deliver`).
    let grant = Grant { reads_all: dispatcher.reads_text(), holder: Some(HOLDER), ..Grant::default() };
    let filter = EventFilter { topics, ..EventFilter::default() };
    let subscription = match bus.subscribe(filter, grant, Some(CALLER)) {
        Ok((subscription, _)) => Arc::new(subscription),
        Err(refused) => {
            tracing::warn!("the Probes' hooks hear nothing: {refused}");
            return None;
        }
    };
    let (tx, rx) = bounded::<Feed>(PENDING);
    let (waiting, sender) = (subscription.clone(), tx.clone());
    let spawned = std::thread::Builder::new().name("wormhole-probe-hooks".into()).spawn(move || {
        let mut printed = Printed::default();
        loop {
            match waiting.next(HEARTBEAT) {
                Next::Event(published) => {
                    let item = match hook_event(&published.event) {
                        Some((event, data)) => Feed::Event(event, vec![data]),
                        None if matches!(published.event.body, Body::Output { .. }) => {
                            let lines = printed.lines(&published.event);
                            if lines.is_empty() {
                                continue;
                            }
                            Feed::Event(HookEvent::OutputMatch, lines)
                        }
                        None => continue,
                    };
                    if sender.is_closed() {
                        break;
                    }
                    // Full: a flood of events. The hooks' budgets would refuse most of them anyway.
                    let _ = sender.try_send(item);
                }
                Next::Lagged(dropped) => tracing::debug!(dropped, "the Probes' hooks fell behind"),
                Next::Closed(_) => break,
                Next::Idle | Next::Changed => {}
            }
        }
    });
    if let Err(err) = spawned {
        tracing::warn!("the Probes' hooks hear nothing: {err}");
        return None;
    }
    let output = topics.contains(Topic::Output);
    Some((Listener { subscription, feed: rx.clone(), sender: tx, output }, rx))
}

impl Feed {
    /// What the hooks decide about what the feed brought, as effects to carry out. `facts` says what the app
    /// knows of a terminal by its id.
    pub fn plan(
        self,
        dispatcher: &mut Dispatcher,
        facts: &dyn Fn(i64) -> Option<TabFacts>,
        now: Instant,
    ) -> Vec<Effect> {
        match self {
            Feed::Event(event, batch) if dispatcher.listens(event) => {
                let mut effects = Vec::new();
                for data in batch {
                    let tab = data.number("terminal").and_then(facts);
                    let data = match tab {
                        Some(tab) => data
                            .with("front", Value::Number(i64::from(tab.front)))
                            .with("kind", Value::Text(if tab.agent { "agent" } else { "shell" }.into()))
                            .with("folder_name", Value::Text(tab.name)),
                        None => data,
                    };
                    effects.extend(dispatcher.deliver(event, &data, now));
                }
                effects
            }
            Feed::Event(..) => Vec::new(),
            Feed::Ended(ticket, end) => dispatcher.finished(ticket, &end, now),
        }
    }
}

/// Run `job` on a thread of its own (`active::execute`: from the approved copy, no shell, scrubbed) and tell
/// `feed` how it ended, which is where its output becomes effects of its own. A thread that cannot be had is an
/// end too, or the hook would wait for it for ever.
pub fn start_run(root: DataRoot, ticket: RunTicket, job: RunJob, feed: &Sender<Feed>) {
    let report = feed.clone();
    let spawned = std::thread::Builder::new().name("wormhole-probe-run".into()).spawn(move || {
        let end = active::execute(&root, &job);
        // Blocking is fine here, and a closed feed (the Probes were loaded again) just means nobody waits.
        let _ = report.send_blocking(Feed::Ended(ticket, end));
    });
    if let Err(err) = spawned {
        let _ = feed.try_send(Feed::Ended(ticket, RunEnd::NotRun(err.to_string())));
    }
}
