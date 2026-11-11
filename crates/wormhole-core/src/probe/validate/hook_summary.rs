//! A Probe's hooks in plain words, for `probe validate` and the approval sheet: when each fires and
//! what it does, so someone can read that before enabling the Probe.

use crate::probe::events::{self, Comparison, MatcherKind};
use crate::probe::manifest::{Hook, HookAction, HookActionKind};

/// One hook, as a sentence's parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookNote {
    pub id: String,
    /// The event, as `command.finished`.
    pub on: &'static str,
    /// What must hold, as `exit != 0, took_ms >= 60000`; empty for every such event.
    pub when: String,
    /// What it does, one per action.
    pub actions: Vec<String>,
}

pub(super) fn notes(hooks: &[Hook]) -> Vec<HookNote> {
    hooks
        .iter()
        .map(|hook| HookNote {
            id: hook.id.clone(),
            on: hook.on.name(),
            when: when(hook),
            actions: hook.actions.iter().map(action).collect(),
        })
        .collect()
}

fn when(hook: &Hook) -> String {
    let spec = events::spec(hook.on);
    hook.matcher
        .iter()
        .map(|(field, value)| match spec.matcher(field).map(|m| m.kind) {
            Some(MatcherKind::Number) => match Comparison::parse(value) {
                Ok(test) => format!("{field} {test}"),
                Err(_) => format!("{field} {value}"),
            },
            Some(MatcherKind::Pattern { .. }) => format!("{field} /{value}/"),
            _ => format!("{field} {value}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn action(action: &HookAction) -> String {
    let some = |text: &Option<String>| text.as_deref().unwrap_or("?").to_string();
    match action.kind {
        HookActionKind::Notify => format!("notify \"{}\"", some(&action.title)),
        HookActionKind::Highlight => format!("highlight in {}", some(&action.tone)),
        HookActionKind::Fold => format!(
            "fold {} \"{}\", keeping {} and {} rows (hides output on screen; copy still gives it)",
            action.target.map_or("?", |t| t.name()),
            some(&action.label),
            action.keep_head.unwrap_or(0),
            action.keep_tail.unwrap_or(0),
        ),
        HookActionKind::OpenPath => format!("offer to open {}", some(&action.path)),
        HookActionKind::Run => format!("run the command `{}` in the background", some(&action.command)),
        HookActionKind::Status => {
            format!("set the status item `{}` to \"{}\"", some(&action.item), some(&action.text))
        }
    }
}
