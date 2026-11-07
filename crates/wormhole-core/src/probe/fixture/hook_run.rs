//! Running a hook case: the actions planned for the event against the actions expected.

use super::{ExpectAction, HookCase};
use crate::probe::events;
use crate::probe::hooks::{CompiledHooks, Plan, Planned};

/// A planned action as the case would spell it.
struct Found {
    kind: &'static str,
    fields: Vec<(&'static str, String)>,
}

impl Found {
    fn of(planned: &Planned) -> Self {
        let text = |value: &str| value.to_string();
        let opt = |name, value: &Option<String>| value.as_ref().map(|v| (name, v.clone()));
        let fields: Vec<Option<(&str, String)>> = match planned {
            Planned::Notify { title, body } => vec![Some(("title", text(title))), Some(("body", text(body)))],
            Planned::Highlight { tone } => vec![Some(("tone", text(tone)))],
            Planned::Fold { target, hidden_rows, label } => vec![
                Some(("target", text(target.name()))),
                hidden_rows.map(|n| ("hidden_rows", n.to_string())),
                Some(("label", text(label))),
            ],
            Planned::OpenPath { path, line } => {
                vec![Some(("path", text(path))), line.map(|n| ("line", n.to_string()))]
            }
            Planned::Run { command, argv, timeout_s, item, .. } => vec![
                Some(("command", text(command))),
                Some(("argv", format!("{argv:?}"))),
                Some(("timeout", timeout_s.to_string())),
                opt("item", item),
            ],
            Planned::Status { item, text: words, tone, icon, tooltip } => vec![
                Some(("item", text(item))),
                Some(("text", text(words))),
                opt("tone", tone),
                opt("icon", icon),
                opt("tooltip", tooltip),
            ],
            Planned::Dropped { action, why } => {
                vec![Some(("action", text(action))), Some(("why", text(why)))]
            }
        };
        Self { kind: planned_kind(planned), fields: fields.into_iter().flatten().collect() }
    }

    fn describe(&self) -> String {
        let fields: Vec<String> =
            self.fields.iter().map(|(name, value)| format!("{name} {value:?}")).collect();
        format!("{} {}", self.kind, fields.join(" "))
    }
}

/// The action type's name; `Found::of` builds the fields, this is the name so the two cannot drift.
fn planned_kind(planned: &Planned) -> &'static str {
    match planned {
        Planned::Notify { .. } => "notify",
        Planned::Highlight { .. } => "highlight",
        Planned::Fold { .. } => "fold",
        Planned::OpenPath { .. } => "open-path",
        Planned::Run { .. } => "run",
        Planned::Status { .. } => "status",
        Planned::Dropped { .. } => "dropped",
    }
}

const KINDS: [&str; 7] = ["notify", "highlight", "fold", "open-path", "run", "status", "dropped"];

/// What is wrong with `case`; nothing when it passes.
pub(super) fn run_case(case: &HookCase, hooks: &CompiledHooks) -> Vec<String> {
    let Some(hook) = hooks.get(&case.hook) else {
        let known: Vec<_> = hooks.iter().map(|h| h.id()).collect();
        return vec![format!("no hook `{}` (the Probe has: {})", case.hook, known.join(", "))];
    };
    let problems = case.event.problems(events::spec(hook.hook.on));
    if !problems.is_empty() {
        return problems.into_iter().map(|p| format!("event: {p}")).collect();
    }
    if let Some(bad) = case.expect.iter().find(|e| !KINDS.contains(&e.kind.as_str())) {
        return vec![format!("`type = {:?}` is not one of {}", bad.kind, KINDS.join(", "))];
    }
    let mut planned = match hook.plan(&case.event) {
        Plan::Actions(planned) => planned,
        Plan::Skipped(why) => {
            return if case.expect.is_empty() {
                Vec::new()
            } else {
                vec![format!("expected {} action(s), but the hook did not fire: {why}", case.expect.len())]
            };
        }
    };
    if let Some(result) = &case.result {
        let Some(at) = planned.iter().position(|p| matches!(p, Planned::Run { .. })) else {
            return vec![
                "`result` is what a `run` printed, and this hook plans no `run` for this event".into(),
            ];
        };
        let item = match &planned[at] {
            Planned::Run { item, .. } => item.clone(),
            _ => None,
        };
        let (effects, _log) = hooks.run_effects(item.as_deref(), result.exit, &result.stdout);
        planned.splice(at + 1..at + 1, effects);
    }
    let found: Vec<Found> = planned.iter().map(Found::of).collect();
    let mut problems = Vec::new();
    for ix in 0..found.len().max(case.expect.len()) {
        match (case.expect.get(ix), found.get(ix)) {
            (Some(want), Some(got)) => compare(want, got, ix, &mut problems),
            (Some(want), None) => {
                problems.push(format!(
                    "action {}: expected {}, but nothing more was planned",
                    ix + 1,
                    want.kind
                ));
            }
            (None, Some(got)) => {
                problems.push(format!("action {}: not expected, but {} was planned", ix + 1, got.describe()))
            }
            (None, None) => {}
        }
    }
    if !problems.is_empty() {
        if found.is_empty() {
            problems.push("planned no actions".into());
        }
        problems.extend(
            found.iter().enumerate().map(|(ix, got)| format!("planned {}: {}", ix + 1, got.describe())),
        );
    }
    problems
}

fn compare(want: &ExpectAction, got: &Found, ix: usize, problems: &mut Vec<String>) {
    let at = format!("action {}", ix + 1);
    if want.kind != got.kind {
        problems.push(format!("{at}: expected a `{}`, planned a `{}`", want.kind, got.kind));
        return;
    }
    for (name, value) in want.fields() {
        match got.fields.iter().find(|(n, _)| *n == name) {
            Some((_, found)) if *found == value => {}
            Some((_, found)) => problems.push(format!("{at}: expected {name} {value:?}, found {found:?}")),
            None => {
                problems.push(format!("{at}: expected {name} {value:?}, but this {} has no {name}", got.kind))
            }
        }
    }
}
