//! The tools: what an agent is offered, how each one's arguments become a control request, and what
//! each says about itself (a description written for the model that reads it, JSON Schemas with bounds,
//! annotations that are true).
//!
//! There is no policy here. A tool builds the same [`Action`] the command line builds and the app decides
//! as it does for `wh`: the switches (`control_write`, `control_read`), the budgets, the refusals and the
//! automation flag are all the app's. A tool's arguments are checked for their shape only (serde, with
//! unknown fields refused so a misspelt one is told, not ignored); the bounds the schemas state are the
//! app's own caps, which it enforces.

mod input;
mod schema;
#[cfg(test)]
pub(super) mod schema_check;
mod structure;
mod terminals;
#[cfg(test)]
mod tests;
mod waiting;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::exec::Endpoint;
use super::feed::EventsQuery;
use super::version::Version;
use crate::ipc::control::Action;

/// What a tool call comes to, before it is carried out.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Call {
    /// A request to the app, the one the command line would send.
    Control(Action),
    /// The buffered stream of events (the one tool that is not a single request).
    Events(EventsQuery),
}

/// The annotations of a tool. All four are stated, so a client need not know the spec's defaults.
#[derive(Debug, Clone, Copy)]
pub(super) struct Hints {
    /// Only looks: changes nothing in the app or in any terminal.
    pub(super) read_only: bool,
    /// May do something that cannot be taken back (typing into a program, running a command).
    pub(super) destructive: bool,
    /// The same call twice leaves the same state as once.
    pub(super) idempotent: bool,
    /// Reaches beyond Wormhole: what runs in a terminal can do anything the user can.
    pub(super) open_world: bool,
}

impl Hints {
    /// Looks at Wormhole and changes nothing.
    pub(super) const LOOKS: Hints =
        Hints { read_only: true, destructive: false, idempotent: true, open_world: false };
    /// Changes Wormhole only, harmlessly and to the same end if repeated.
    pub(super) const ARRANGES: Hints =
        Hints { read_only: false, destructive: false, idempotent: true, open_world: false };
    /// Changes Wormhole only, and again each time.
    pub(super) const ADDS: Hints =
        Hints { read_only: false, destructive: false, idempotent: false, open_world: false };
    /// Starts or drives programs: whatever they do.
    pub(super) const DRIVES: Hints =
        Hints { read_only: false, destructive: true, idempotent: false, open_world: true };
}

/// One tool.
pub(super) struct Tool {
    pub(super) name: &'static str,
    pub(super) title: &'static str,
    pub(super) description: &'static str,
    pub(super) hints: Hints,
    /// Can take a long while: a call tells its progress while it waits, if the client asks for it.
    pub(super) long: bool,
    /// Needs the Automation feature flag: the five tools the command line always had (`list`, `status`,
    /// `notify`, `send-text`, `new-terminal`) do not, the rest answer `unsupported` until it is on.
    pub(super) flag: bool,
    pub(super) input: fn() -> Value,
    pub(super) output: fn() -> Value,
    pub(super) build: fn(Value, &Endpoint) -> Result<Call, String>,
}

impl Tool {
    /// The tool as `tools/list` shows it to a client of `version`: later revisions add fields, and an earlier
    /// one is not sent what it does not have.
    pub(super) fn definition(&self, version: Version) -> Value {
        let mut tool =
            json!({"name": self.name, "description": self.description, "inputSchema": (self.input)()});
        if version.has_structured_output() {
            tool["title"] = json!(self.title);
            tool["outputSchema"] = (self.output)();
        }
        if version.has_annotations() {
            tool["annotations"] = json!({
                "title": self.title,
                "readOnlyHint": self.hints.read_only,
                "destructiveHint": self.hints.destructive,
                "idempotentHint": self.hints.idempotent,
                "openWorldHint": self.hints.open_world,
            });
        }
        tool
    }
}

/// Every tool, in the order `tools/list` gives them (always the same one, which lets a client cache them).
pub(super) fn all() -> impl Iterator<Item = &'static Tool> {
    terminals::TOOLS.iter().chain(input::TOOLS).chain(structure::TOOLS).chain(waiting::TOOLS)
}

/// The tools a client is offered: all of them while the Automation flag is on, else those that work without it.
pub(super) fn offered(flag_on: bool) -> impl Iterator<Item = &'static Tool> {
    all().filter(move |tool| flag_on || !tool.flag)
}

pub(super) fn find(name: &str) -> Option<&'static Tool> {
    all().find(|tool| tool.name == name)
}

/// The arguments of a call as the type a tool reads them into. `arguments` may be missing for a tool that
/// takes none. The error is the words the model reads to fix its call.
fn read<T: DeserializeOwned>(tool: &str, arguments: Value) -> Result<T, String> {
    serde_json::from_value(arguments).map_err(|err| format!("the arguments of {tool} are wrong: {err}"))
}
