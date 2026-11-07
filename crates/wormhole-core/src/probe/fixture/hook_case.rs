//! `[[hook_case]]`: one event and the actions its hook must plan.

use serde::Deserialize;

use crate::probe::events::EventData;

/// A case for a hook.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookCase {
    pub name: Option<String>,
    /// The hook's local id.
    pub hook: String,
    /// The fields of the event (`events::spec` lists them), plus `front` (0 or 1) and `kind` (`agent` or
    /// `shell`) for a `match` that asks about the terminal.
    pub event: EventData,
    /// What the first `run` action's process did.
    pub result: Option<RunResult>,
    #[serde(default)]
    pub expect: Vec<ExpectAction>,
}

/// How a `run` ended.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub exit: i32,
    #[serde(default)]
    pub stdout: String,
}

/// One action a case expects: the type, and whichever of its fields the case checks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectAction {
    /// `notify`, `highlight`, `fold`, `open-path`, `run`, `status`, or `dropped`.
    #[serde(rename = "type")]
    pub kind: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub tone: Option<String>,
    pub target: Option<String>,
    pub hidden_rows: Option<u64>,
    pub label: Option<String>,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub command: Option<String>,
    /// A `run`'s final argument vector, `git`'s end-of-options marker and all.
    pub argv: Option<Vec<String>>,
    pub timeout: Option<u64>,
    pub item: Option<String>,
    pub text: Option<String>,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    /// `dropped`: the type of the action a fixed rule refused.
    pub action: Option<String>,
}

impl ExpectAction {
    /// The fields the case checks, as `(name, value)` with the value as the report prints it.
    pub(super) fn fields(&self) -> Vec<(&'static str, String)> {
        let text = |name, value: &Option<String>| value.as_ref().map(|v| (name, v.clone()));
        let number = |name, value: Option<u64>| value.map(|v| (name, v.to_string()));
        [
            text("title", &self.title),
            text("body", &self.body),
            text("tone", &self.tone),
            text("target", &self.target),
            number("hidden_rows", self.hidden_rows),
            text("label", &self.label),
            text("path", &self.path),
            number("line", self.line.map(u64::from)),
            text("command", &self.command),
            self.argv.as_ref().map(|argv| ("argv", format!("{argv:?}"))),
            number("timeout", self.timeout),
            text("item", &self.item),
            text("text", &self.text),
            text("icon", &self.icon),
            text("tooltip", &self.tooltip),
            text("action", &self.action),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
