//! One event over every Probe's hooks.
//!
//! The dispatcher decides and the app carries out: an [`Effect`] says what to show, never how, and nothing
//! here spawns, writes or draws. A hook never affects the terminal it listens to: nothing it plans can
//! block, rewrite or answer, a plan that fails is a hook that did nothing, and what it may do is bounded
//! by [`Limits`], counted per hook, so a program that rings the bell in a loop costs the budget, not the app.
//!
//! In this build the hooks' `notify`, `status`, `highlight`, `open-path` and `run` act; `fold` is planned by
//! `probe test` and is not carried out yet. A `run` is the one effect that has a life after it: the dispatcher
//! says what to start ([`Effect::Run`], under the `process` grant, one at a time per hook and a few at once)
//! and is told how it ended ([`Dispatcher::finished`]), which is when its output becomes effects of its own
//! and when a hook that keeps failing is paused.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::load::ActiveProbe;
use super::run::RunEnd;
use crate::probe::events::{self, EventData};
use crate::probe::hooks::{Plan, Planned};
use crate::probe::manifest::{Cwd, HookActionKind, HookEvent};
use crate::probe::tone::Tone;

/// What a hook and a Probe may do in a minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Least time between two runs of one hook: a burst of the same event is one.
    pub cooldown: Duration,
    /// Runs of one hook in the last minute.
    pub per_minute: usize,
    /// Notifications of one Probe in the last minute, whatever its hooks.
    pub notifications_per_minute: usize,
    /// Programs started and not yet ended, over every Probe.
    pub runs_at_once: usize,
    /// Failed runs of one hook in a minute that pause it.
    pub failures_to_pause: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            cooldown: Duration::from_millis(100),
            per_minute: 60,
            notifications_per_minute: 6,
            runs_at_once: 4,
            failures_to_pause: 3,
        }
    }
}

const MINUTE: Duration = Duration::from_secs(60);

/// What the app is to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// A notification, which the app words as the Probe's and rate-limits again.
    Notify { probe: String, probe_name: String, title: String, body: String, terminal: Option<i64> },
    /// The text, tone and tooltip of a status item the Probe declared.
    Status {
        probe: String,
        probe_name: String,
        item: String,
        text: String,
        tone: Option<Tone>,
        icon: Option<String>,
        tooltip: Option<String>,
    },
    /// A tone on the rows of the block that finished.
    Highlight { probe: String, tone: Tone, terminal: i64, block: i64 },
    /// A file the Probe offers to open, as written (`path[:line]` is for the app to find and open when
    /// the user says so, never by itself).
    OpenPath { probe: String, probe_name: String, terminal: i64, path: String, line: Option<u32> },
    /// A program to start; when it ends the app calls [`Dispatcher::finished`] with `ticket`.
    Run {
        ticket: RunTicket,
        probe: String,
        argv: Vec<String>,
        cwd: Cwd,
        timeout: Duration,
        /// Variables the user let the Probe pass on, by name.
        env: Vec<String>,
        terminal: Option<i64>,
    },
    /// A hook failed too often in a minute and was paused until the Probes are loaded again.
    Paused { probe_name: String, hook: String },
}

/// Which run ended: the hook that started it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunTicket {
    slot: usize,
    hook: usize,
}

/// Times something happened in the last minute.
#[derive(Debug, Default)]
struct Recent(VecDeque<Instant>);

impl Recent {
    fn within(&mut self, now: Instant) -> usize {
        while self.0.front().is_some_and(|at| now.duration_since(*at) >= MINUTE) {
            self.0.pop_front();
        }
        self.0.len()
    }
}

#[derive(Debug, Default)]
struct HookBudget {
    last: Option<Instant>,
    runs: Recent,
    /// Runs refused, for the log.
    refused: u64,
    /// A program this hook started has not ended.
    running: bool,
    /// Programs of this hook that failed in the last minute.
    failures: Recent,
    /// Too many failed: it does nothing until the Probes are loaded again.
    paused: bool,
}

#[derive(Debug)]
struct Slot {
    probe: ActiveProbe,
    hooks: Vec<HookBudget>,
    notices: Recent,
}

/// Every Probe that is on, with what each hook has used.
#[derive(Debug)]
pub struct Dispatcher {
    slots: Vec<Slot>,
    limits: Limits,
    /// Programs started and not yet ended.
    in_flight: usize,
}

impl Dispatcher {
    pub fn new(probes: Vec<ActiveProbe>) -> Self {
        Self::with_limits(probes, Limits::default())
    }

    pub fn with_limits(probes: Vec<ActiveProbe>, limits: Limits) -> Self {
        let slots = probes
            .into_iter()
            .map(|probe| Slot {
                hooks: probe.hooks.iter().map(|_| HookBudget::default()).collect(),
                probe,
                notices: Recent::default(),
            })
            .collect();
        Self { slots, limits, in_flight: 0 }
    }

    /// The events some hook listens to, each once. `output.match` only counts for a Probe that may read the
    /// terminals: what it matches is their text, which it would never be told.
    pub fn events(&self) -> Vec<HookEvent> {
        let mut events: Vec<HookEvent> = self
            .slots
            .iter()
            .flat_map(|slot| {
                let reads = slot.probe.grants.terminal_read;
                slot.probe
                    .hooks
                    .iter()
                    .map(|hook| hook.hook.on)
                    .filter(move |on| reads || *on != HookEvent::OutputMatch)
            })
            .collect();
        events.sort_by_key(|event| HookEvent::ALL.iter().position(|e| e == event));
        events.dedup();
        events
    }

    /// Whether some Probe may be told what is on the terminals: the feed then hears their text too, and each
    /// Probe's own grant decides what reaches it.
    pub fn reads_text(&self) -> bool {
        self.slots.iter().any(|slot| slot.probe.grants.terminal_read)
    }

    /// Whether some hook listens to `event`.
    pub fn listens(&self, event: HookEvent) -> bool {
        self.slots.iter().any(|slot| slot.probe.hooks.iter().any(|hook| hook.hook.on == event))
    }

    /// Runs refused for want of budget, over every hook: for the log and the tests.
    pub fn refused(&self) -> u64 {
        self.slots.iter().flat_map(|slot| &slot.hooks).map(|budget| budget.refused).sum()
    }

    /// What `event`, whose fields are `data`, makes the Probes' hooks do. Cheap when nothing listens: a
    /// hook that does not match costs a comparison of its fields and no budget.
    pub fn deliver(&mut self, event: HookEvent, data: &EventData, now: Instant) -> Vec<Effect> {
        let Self { slots, limits, in_flight } = self;
        let limits = *limits;
        let mut effects = Vec::new();
        for (slot_ix, slot) in slots.iter_mut().enumerate() {
            if !slot.probe.hooks.iter().any(|hook| hook.hook.on == event) {
                continue;
            }
            // Text a terminal holds reaches a hook only when the user let this Probe read the terminals.
            let data: Cow<'_, EventData> = if slot.probe.grants.terminal_read {
                Cow::Borrowed(data)
            } else {
                Cow::Owned(data.without_text(events::spec(event)))
            };
            for (ix, hook) in slot.probe.hooks.iter().enumerate().filter(|(_, hook)| hook.hook.on == event) {
                let Plan::Actions(planned) = hook.plan(&data) else { continue };
                let budget = &mut slot.hooks[ix];
                let quiet = budget.last.is_none_or(|at| now.duration_since(at) >= limits.cooldown);
                if budget.paused || !quiet || budget.runs.within(now) >= limits.per_minute {
                    budget.refused += 1;
                    continue;
                }
                budget.last = Some(now);
                budget.runs.0.push_back(now);
                for step in planned {
                    let effect = match step {
                        Planned::Run { argv, cwd, timeout_s, .. } => {
                            let ticket = RunTicket { slot: slot_ix, hook: ix };
                            let launch =
                                Launch { ticket, argv, cwd, timeout: Duration::from_secs(timeout_s) };
                            start(&slot.probe, budget, in_flight, launch, &data, limits)
                        }
                        step => effect_of(&slot.probe, step, &data, &mut slot.notices, now, limits),
                    };
                    effects.extend(effect);
                }
            }
        }
        effects
    }

    /// The program `ticket` started ended as `end`. What it printed becomes effects, a failure counts against
    /// its hook, and a hook that failed too often in a minute is paused.
    pub fn finished(&mut self, ticket: RunTicket, end: &RunEnd, now: Instant) -> Vec<Effect> {
        let limits = self.limits;
        let Some(slot) = self.slots.get_mut(ticket.slot) else { return Vec::new() };
        let Some(hook) = slot.probe.hooks.iter().nth(ticket.hook) else { return Vec::new() };
        self.in_flight = self.in_flight.saturating_sub(1);
        let budget = &mut slot.hooks[ticket.hook];
        budget.running = false;
        // What the run feeds: the first `run` action of the hook, the one that can have started.
        let item =
            hook.hook.actions.iter().find_map(|a| a.item.clone().filter(|_| a.kind == HookActionKind::Run));
        let id = hook.id().to_string();
        let (planned, log) = match end {
            RunEnd::Exited { code, stdout } => slot.probe.hooks.run_effects(item.as_deref(), *code, stdout),
            RunEnd::TimedOut => (muted(item.as_deref(), "timed out"), vec!["timed out".to_string()]),
            RunEnd::TooMuchOutput => (muted(item.as_deref(), "failed"), vec!["printed too much".to_string()]),
            RunEnd::NotRun(why) => (muted(item.as_deref(), "failed"), vec![format!("not run: {why}")]),
        };
        for line in log {
            tracing::debug!(probe = %slot.probe.id, hook = %id, "{line}");
        }
        let mut effects: Vec<Effect> = planned
            .into_iter()
            .filter_map(|step| {
                effect_of(&slot.probe, step, &EventData::default(), &mut slot.notices, now, limits)
            })
            .collect();
        // A program that was never started is not the program's failure: nothing there to pause.
        let failed = match end {
            RunEnd::Exited { code, .. } => *code != 0,
            RunEnd::TimedOut | RunEnd::TooMuchOutput => true,
            RunEnd::NotRun(_) => false,
        };
        if failed {
            budget.failures.0.push_back(now);
            if budget.failures.within(now) >= limits.failures_to_pause && !budget.paused {
                budget.paused = true;
                effects.push(Effect::Paused { probe_name: slot.probe.name.clone(), hook: id });
            }
        }
        effects
    }
}

/// A program to start, before the dispatcher has checked what may run.
struct Launch {
    ticket: RunTicket,
    argv: Vec<String>,
    cwd: Cwd,
    timeout: Duration,
}

/// Start `launch` if the Probe may and the budgets allow: the program was granted by name in `process`, the
/// hook has none running, and only so many run at once.
fn start(
    probe: &ActiveProbe,
    budget: &mut HookBudget,
    in_flight: &mut usize,
    launch: Launch,
    data: &EventData,
    limits: Limits,
) -> Option<Effect> {
    let Launch { ticket, argv, cwd, timeout } = launch;
    let refuse = |why: &str| tracing::debug!(probe = %probe.id, why, "a hook's run was refused");
    let Some(program) = argv.first().filter(|program| probe.grants.process.contains(program)) else {
        refuse("the program is not granted");
        return None;
    };
    if budget.running || *in_flight >= limits.runs_at_once {
        budget.refused += 1;
        refuse(&format!("`{program}` is busy"));
        return None;
    }
    budget.running = true;
    *in_flight += 1;
    Some(Effect::Run {
        ticket,
        probe: probe.id.clone(),
        argv,
        cwd,
        timeout,
        env: probe.grants.env.clone(),
        terminal: data.number("terminal"),
    })
}

/// A status item's dim words, when the action feeds one.
fn muted(item: Option<&str>, text: &str) -> Vec<Planned> {
    item.map(|item| Planned::Status {
        item: item.to_string(),
        text: text.to_string(),
        tone: Some("muted".into()),
        icon: None,
        tooltip: None,
    })
    .into_iter()
    .collect()
}

fn effect_of(
    probe: &ActiveProbe,
    planned: Planned,
    data: &EventData,
    notices: &mut Recent,
    now: Instant,
    limits: Limits,
) -> Option<Effect> {
    match planned {
        Planned::Notify { title, body } => {
            if !probe.grants.notify || notices.within(now) >= limits.notifications_per_minute {
                return None;
            }
            notices.0.push_back(now);
            Some(Effect::Notify {
                probe: probe.id.clone(),
                probe_name: probe.name.clone(),
                title,
                body,
                terminal: data.number("terminal"),
            })
        }
        Planned::Status { item, text, tone, icon, tooltip } => {
            probe.status_items.contains(&item).then(|| Effect::Status {
                probe: probe.id.clone(),
                probe_name: probe.name.clone(),
                item,
                text,
                tone: tone.as_deref().and_then(Tone::from_name),
                icon,
                tooltip,
            })
        }
        Planned::Highlight { tone } => Some(Effect::Highlight {
            probe: probe.id.clone(),
            tone: Tone::from_name(&tone)?,
            terminal: data.number("terminal")?,
            block: data.number("block")?,
        }),
        Planned::Dropped { action, why } => {
            tracing::debug!(probe = %probe.id, action, why, "a hook's action was refused");
            None
        }
        Planned::OpenPath { path, line } => Some(Effect::OpenPath {
            probe: probe.id.clone(),
            probe_name: probe.name.clone(),
            terminal: data.number("terminal")?,
            path,
            line,
        }),
        Planned::Fold { .. } | Planned::Run { .. } => None,
    }
}

#[cfg(test)]
mod tests;
