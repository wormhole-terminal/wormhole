//! One event over every Probe's hooks.
//!
//! The dispatcher decides and the app carries out: an [`Effect`] says what to show, never how, and nothing
//! here spawns, writes or draws. A hook never affects the terminal it listens to: nothing it plans can
//! block, rewrite or answer, a plan that fails is a hook that did nothing, and what it may do is bounded
//! by [`Limits`], counted per hook, so a program that rings the bell in a loop costs the budget, not the app.
//!
//! In this build the hooks' `notify`, `status` and `highlight` act; the other actions (`fold`, `open-path`,
//! `run`) are planned by `probe test` and are not carried out yet.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::load::ActiveProbe;
use crate::probe::events::{self, EventData};
use crate::probe::hooks::{Plan, Planned};
use crate::probe::manifest::HookEvent;
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
}

impl Default for Limits {
    fn default() -> Self {
        Self { cooldown: Duration::from_millis(100), per_minute: 60, notifications_per_minute: 6 }
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
        Self { slots, limits }
    }

    /// The events some hook listens to, each once.
    pub fn events(&self) -> Vec<HookEvent> {
        let mut events: Vec<HookEvent> =
            self.slots.iter().flat_map(|slot| slot.probe.hooks.iter().map(|hook| hook.hook.on)).collect();
        events.sort_by_key(|event| HookEvent::ALL.iter().position(|e| e == event));
        events.dedup();
        events
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
        let limits = self.limits;
        let mut effects = Vec::new();
        for slot in &mut self.slots {
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
                if !quiet || budget.runs.within(now) >= limits.per_minute {
                    budget.refused += 1;
                    continue;
                }
                budget.last = Some(now);
                budget.runs.0.push_back(now);
                for step in planned {
                    if let Some(effect) = effect_of(&slot.probe, step, &data, &mut slot.notices, now, limits)
                    {
                        effects.push(effect);
                    }
                }
            }
        }
        effects
    }
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
        Planned::Fold { .. } | Planned::OpenPath { .. } | Planned::Run { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::active::{Grants, load::ActiveProbe};
    use crate::probe::events::Value;
    use crate::probe::hooks::CompiledHooks;
    use crate::probe::manifest::Manifest;
    use crate::probe::rules::CompiledRules;

    const HOOKS: &str = r#"
[permissions]
notify = true

[[contributes.status]]
id = "build"

[[contributes.hooks]]
id = "failed"
on = "command.finished"
match = { exit = "!=0" }
do = [
  { type = "notify", title = "Build failed", body = "{took_s} s in {folder_name}" },
  { type = "status", item = "build", text = "failed", tone = "danger" },
  { type = "highlight", tone = "danger" },
]

[[contributes.hooks]]
id = "bell"
on = "bell"
do = [{ type = "status", item = "build", text = "rang" }]

[[contributes.hooks]]
id = "strangers"
on = "bell"
do = [{ type = "status", item = "other", text = "x" }]
"#;

    fn probe(extra: &str, grants: Grants) -> ActiveProbe {
        let text = format!(
            "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
             version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n{extra}"
        );
        let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}"));
        let manifest = parsed.manifest;
        ActiveProbe {
            id: "tiny".into(),
            name: "Tiny".into(),
            rules: CompiledRules::compile(&manifest).expect("rules"),
            hooks: CompiledHooks::compile(&manifest).expect("hooks"),
            grants,
            status_items: manifest.contributes.status.iter().map(|s| s.id.clone()).collect(),
        }
    }

    fn finished(exit: i64, took_ms: i64) -> EventData {
        EventData::new([
            ("terminal".to_string(), Value::Number(3)),
            ("block".to_string(), Value::Number(9)),
            ("exit".to_string(), Value::Number(exit)),
            ("took_ms".to_string(), Value::Number(took_ms)),
            ("folder_name".to_string(), Value::Text("acme".into())),
        ])
    }

    fn notify() -> Grants {
        Grants { notify: true, ..Grants::default() }
    }

    #[test]
    fn a_failed_command_notifies_sets_the_item_and_highlights_its_block() {
        let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, notify())]);
        let effects = dispatcher.deliver(HookEvent::CommandFinished, &finished(2, 61_000), Instant::now());
        assert_eq!(effects.len(), 3, "{effects:?}");
        assert!(
            matches!(&effects[0], Effect::Notify { probe, probe_name, title, body, terminal: Some(3) }
            if probe == "tiny" && probe_name == "Tiny" && title == "Build failed" && body == "61 s in acme"),
            "{:?}",
            effects[0]
        );
        assert!(matches!(&effects[1], Effect::Status { item, text, tone: Some(Tone::Danger), .. }
            if item == "build" && text == "failed"));
        assert_eq!(
            effects[2],
            Effect::Highlight { probe: "tiny".into(), tone: Tone::Danger, terminal: 3, block: 9 }
        );
        assert!(
            dispatcher.deliver(HookEvent::CommandFinished, &finished(0, 5), Instant::now()).is_empty(),
            "exit 0 does not match"
        );
    }

    #[test]
    fn a_notification_needs_the_grant_and_the_rest_still_acts() {
        let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, Grants::default())]);
        let effects = dispatcher.deliver(HookEvent::CommandFinished, &finished(1, 1), Instant::now());
        assert!(effects.iter().all(|e| !matches!(e, Effect::Notify { .. })), "{effects:?}");
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn a_status_item_the_probe_did_not_declare_is_not_set() {
        let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, notify())]);
        let effects = dispatcher.deliver(
            HookEvent::Bell,
            &EventData::new([("terminal".to_string(), Value::Number(1))]),
            Instant::now(),
        );
        assert_eq!(effects.len(), 1, "`rang` only: `other` is nobody's item");
        assert!(matches!(&effects[0], Effect::Status { text, .. } if text == "rang"));
    }

    #[test]
    fn a_burst_is_one_run_within_the_cooldown_and_a_minute_has_a_cap() {
        let limits =
            Limits { cooldown: Duration::from_millis(100), per_minute: 3, notifications_per_minute: 2 };
        let mut dispatcher = Dispatcher::with_limits(vec![probe(HOOKS, notify())], limits);
        let start = Instant::now();
        let bell = EventData::new([("terminal".to_string(), Value::Number(1))]);
        assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, start).len(), 1);
        assert!(dispatcher.deliver(HookEvent::Bell, &bell, start + Duration::from_millis(10)).is_empty());
        assert_eq!(dispatcher.refused(), 2, "counted for each of the two hooks on the bell");
        let later = |ms: u64| start + Duration::from_millis(ms);
        assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(200)).len(), 1);
        assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(400)).len(), 1);
        assert!(dispatcher.deliver(HookEvent::Bell, &bell, later(600)).is_empty(), "the fourth in a minute");
        assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(61_000)).len(), 1, "a new minute");
        // Notifications have a cap of their own.
        let mut dispatcher = Dispatcher::with_limits(vec![probe(HOOKS, notify())], limits);
        let notices: usize = (0..3u64)
            .map(|n| {
                dispatcher
                    .deliver(
                        HookEvent::CommandFinished,
                        &finished(1, 1),
                        start + Duration::from_millis(200 * n),
                    )
                    .iter()
                    .filter(|e| matches!(e, Effect::Notify { .. }))
                    .count()
            })
            .sum();
        assert_eq!(notices, 2);
    }

    #[test]
    fn text_fields_reach_a_hook_only_with_the_read_grant() {
        let text_hook = r#"
[[contributes.hooks]]
id = "wherever"
on = "folder.changed"
match = { folder = "/work/*" }
do = [{ type = "highlight", tone = "link" }]
"#;
        let folder = EventData::new([
            ("terminal".to_string(), Value::Number(1)),
            ("block".to_string(), Value::Number(1)),
            ("folder".to_string(), Value::Text("/work/a".into())),
        ]);
        let mut without = Dispatcher::new(vec![probe(text_hook, Grants::default())]);
        assert!(without.deliver(HookEvent::FolderChanged, &folder, Instant::now()).is_empty());
        let read = Grants { terminal_read: true, ..Grants::default() };
        let mut with = Dispatcher::new(vec![probe(text_hook, read)]);
        // The highlight is refused by `validate` on this event, but the dispatcher only acts on what matched.
        assert_eq!(with.deliver(HookEvent::FolderChanged, &folder, Instant::now()).len(), 1);
    }
}
