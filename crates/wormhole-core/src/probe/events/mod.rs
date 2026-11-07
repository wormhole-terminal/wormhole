//! What a hook can listen to: the events, the fields each carries and the matcher fields each takes
//! (`repo-docs/docs/research/probes-hooks-and-fold.md`, sections 3.1 and 3.2).
//!
//! This is the vocabulary as data, one table, so `validate`, the fixtures' runner and (later) the
//! delivery read the same one. Each event is mapped onto what exists: the `wh events` topic and body
//! it comes from ([`Source::Bus`]), or the one that has none yet ([`Source::GitWatch`]). Nothing here
//! delivers an event or knows the bus: the names are text, and `tests/probes_events.rs` checks them
//! against `ipc`'s own.
//!
//! The policy lives in [`FieldKind`]: events carry ids and numbers freely, a few names the sidebar shows
//! anyway, and text only as the table names it. A [`FieldKind::Text`] field is hostile input (a command
//! line, a branch, a line of output) and a Probe may send it anywhere only with `terminal = ["read"]`.
//!
//! Must not use anything above `manifest`.

mod data;
mod template;
mod value;

use crate::probe::manifest::HookEvent;

pub use data::{EventData, Value};
pub use template::{Piece, parse_template};
pub use value::{Comparison, Glob, MAX_LIST, MAX_TEXT_BYTES, list};

/// What a field's value is, which decides what it may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// A terminal, block or window number.
    Id,
    Number,
    /// One of [`STATES`].
    State,
    /// A short label the app shows anyway (an agent's id, a terminal's row name): cleaned, and free to use.
    Name,
    /// Text a program or a repository chose: cleaned like a program's, and needs `terminal:read` to leave
    /// the matcher.
    Text,
}

/// One field an event carries.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
}

/// What a matcher field's value looks like, and so what it tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherKind {
    /// `[op]N` with `op` one of `=`, `!=`, `<`, `<=`, `>`, `>=`, against the number field of the same name.
    Number,
    /// A comma-separated list of [`STATES`], against the state field of the same name.
    States,
    /// A comma-separated list of ids, against the name field of the same name.
    Names,
    /// A comma-separated list of anchored globs (`*` is the only wildcard), against the text field of
    /// the same name.
    Globs,
    /// A regular expression (`probes.md` 3.1's limits) against the text field `on`; its groups are the
    /// hook's `{1}`, `{2}`.
    Pattern { on: &'static str },
    /// `front`: only while the terminal's window is in front.
    Front,
    /// `agent` or `shell`: what the terminal is running.
    TerminalKind,
}

/// One field a hook's `match` may hold.
#[derive(Debug, Clone, Copy)]
pub struct MatchField {
    pub name: &'static str,
    pub kind: MatcherKind,
}

/// Where an event comes from today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The `wh events` bus (`ipc::control::event`): a `topic` (`blocks`), the `body` event that carries it
    /// (`block`) and, for `block`, the `phase`.
    Bus { topic: &'static str, body: &'static str, phase: Option<&'static str> },
    /// Core's git watch (`git/watch`), which has no bus topic yet.
    GitWatch,
}

/// One event.
#[derive(Debug, Clone, Copy)]
pub struct EventSpec {
    pub event: HookEvent,
    pub source: Source,
    /// What the event carries.
    pub fields: &'static [Field],
    /// What the app works out from them, usable in a template like a field.
    pub derived: &'static [Field],
    /// Matcher fields of this event alone; [`UNIVERSAL`] come on top.
    pub matchers: &'static [MatchField],
}

/// The states a terminal reports, as the bus spells them.
pub const STATES: [&str; 6] = ["idle", "working", "needs_you", "finished", "failed", "exited"];

/// Hooks a Probe may have, and actions in one of them.
pub const MAX_HOOKS: usize = 32;
pub const MAX_ACTIONS: usize = 4;

/// What an action's expanded text is cut to, in characters (with a `…`): what the sheets and the status
/// bar can show, not what a program printed.
pub const MAX_TITLE_CHARS: usize = 80;
pub const MAX_BODY_CHARS: usize = 240;
pub const MAX_STATUS_CHARS: usize = 80;
pub const MAX_TOOLTIP_CHARS: usize = 120;
/// A file's path, in bytes: longer is not offered.
pub const MAX_PATH_BYTES: usize = 1024;
/// A fold marker's words, in characters.
pub const MAX_LABEL_CHARS: usize = 40;
/// Rows a fold keeps at each end, at most.
pub const MAX_KEEP: u32 = 20;
/// Fewest rows a fold hides, and most.
pub const MIN_HIDDEN_ROWS: u32 = 3;
pub const MAX_HIDDEN_ROWS: u32 = 100_000;
/// A `run`'s seconds before it is killed: the default and the most.
pub const RUN_TIMEOUT_DEFAULT: u64 = 10;
pub const RUN_TIMEOUT_MAX: u64 = 60;
/// What a `run` may print that is read, in bytes.
pub const RUN_OUTPUT_MAX: usize = 4096;

/// What every event has, whatever it is: the terminal's row name, usable in a template.
pub const CONTEXT: &[Field] = &[Field { name: "folder_name", kind: FieldKind::Name }];

/// Matcher fields every event takes.
pub const UNIVERSAL: &[MatchField] = &[
    MatchField { name: "when", kind: MatcherKind::Front },
    MatchField { name: "kind", kind: MatcherKind::TerminalKind },
];

const fn id(name: &'static str) -> Field {
    Field { name, kind: FieldKind::Id }
}
const fn number(name: &'static str) -> Field {
    Field { name, kind: FieldKind::Number }
}
const fn text(name: &'static str) -> Field {
    Field { name, kind: FieldKind::Text }
}
const fn label(name: &'static str) -> Field {
    Field { name, kind: FieldKind::Name }
}
const fn numeric(name: &'static str) -> MatchField {
    MatchField { name, kind: MatcherKind::Number }
}

const fn bus(topic: &'static str, body: &'static str, phase: Option<&'static str>) -> Source {
    Source::Bus { topic, body, phase }
}

const SPECS: [EventSpec; 10] = [
    EventSpec {
        event: HookEvent::CommandStarted,
        source: bus("blocks", "block", Some("started")),
        fields: &[id("terminal"), id("block"), text("command")],
        derived: &[],
        matchers: &[],
    },
    EventSpec {
        event: HookEvent::CommandFinished,
        source: bus("blocks", "block", Some("finished")),
        fields: &[
            id("terminal"),
            id("block"),
            number("exit"),
            number("took_ms"),
            // The finished block's rows, counted by the app when it tells the event: the bus does not
            // carry it today.
            number("output_lines"),
            text("command"),
        ],
        derived: &[Field { name: "took_s", kind: FieldKind::Number }],
        matchers: &[numeric("exit"), numeric("took_ms"), numeric("output_lines")],
    },
    EventSpec {
        event: HookEvent::AgentStatus,
        source: bus("status", "status", None),
        fields: &[
            id("terminal"),
            Field { name: "status", kind: FieldKind::State },
            Field { name: "was", kind: FieldKind::State },
            label("agent"),
            text("message"),
        ],
        derived: &[],
        matchers: &[
            MatchField { name: "status", kind: MatcherKind::States },
            MatchField { name: "was", kind: MatcherKind::States },
            MatchField { name: "agent", kind: MatcherKind::Names },
        ],
    },
    EventSpec {
        event: HookEvent::OutputMatch,
        source: bus("output", "output", None),
        // `line` is the finished logical line the pattern ran over; the captures are its groups.
        fields: &[id("terminal"), id("block"), number("line_no"), text("line")],
        derived: &[],
        matchers: &[MatchField { name: "pattern", kind: MatcherKind::Pattern { on: "line" } }],
    },
    EventSpec {
        event: HookEvent::GitChanged,
        source: Source::GitWatch,
        fields: &[
            id("terminal"),
            text("branch"),
            number("ahead"),
            number("behind"),
            number("dirty"),
            number("staged"),
            number("untracked"),
        ],
        derived: &[],
        matchers: &[
            MatchField { name: "branch", kind: MatcherKind::Globs },
            numeric("ahead"),
            numeric("behind"),
            numeric("dirty"),
            numeric("staged"),
            numeric("untracked"),
        ],
    },
    EventSpec {
        event: HookEvent::FolderChanged,
        source: bus("folder", "folder", None),
        fields: &[id("terminal"), text("folder")],
        derived: &[],
        matchers: &[MatchField { name: "folder", kind: MatcherKind::Globs }],
    },
    EventSpec {
        event: HookEvent::Bell,
        source: bus("bell", "bell", None),
        fields: &[id("terminal")],
        derived: &[],
        matchers: &[],
    },
    EventSpec {
        event: HookEvent::Title,
        source: bus("title", "title", None),
        fields: &[id("terminal"), label("name"), text("title")],
        derived: &[],
        matchers: &[MatchField { name: "pattern", kind: MatcherKind::Pattern { on: "title" } }],
    },
    EventSpec {
        event: HookEvent::TerminalOpened,
        source: bus("terminals", "terminal_opened", None),
        fields: &[id("terminal"), text("folder")],
        derived: &[],
        matchers: &[],
    },
    EventSpec {
        event: HookEvent::TerminalClosed,
        source: bus("terminals", "terminal_closed", None),
        fields: &[id("terminal")],
        derived: &[],
        matchers: &[],
    },
];

/// Every event, in [`HookEvent::ALL`]'s order.
pub fn all() -> &'static [EventSpec] {
    &SPECS
}

/// The spec of `event`.
pub fn spec(event: HookEvent) -> &'static EventSpec {
    SPECS.iter().find(|spec| spec.event == event).unwrap_or(&SPECS[0])
}

impl EventSpec {
    /// The field `name` of the payload.
    pub fn field(&self, name: &str) -> Option<&'static Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// What a template may name here: the payload, what is worked out from it, and the context.
    pub fn template_field(&self, name: &str) -> Option<&'static Field> {
        self.fields.iter().chain(self.derived).chain(CONTEXT).find(|f| f.name == name)
    }

    /// The matcher field `name` of this event.
    pub fn matcher(&self, name: &str) -> Option<&'static MatchField> {
        self.matchers.iter().chain(UNIVERSAL).find(|f| f.name == name)
    }

    /// The matcher fields, as a sentence's list: `exit, took_ms, when, kind`.
    pub fn matcher_names(&self) -> String {
        self.matchers.iter().chain(UNIVERSAL).map(|f| f.name).collect::<Vec<_>>().join(", ")
    }

    /// Whether the hook's `pattern` produces captures here (`{1}` and the like).
    pub fn has_pattern(&self) -> bool {
        self.matcher("pattern").is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_has_one_spec_in_order() {
        let order: Vec<_> = all().iter().map(|s| s.event).collect();
        assert_eq!(order, HookEvent::ALL);
        for event in HookEvent::ALL {
            assert_eq!(spec(event).event, event);
        }
    }

    #[test]
    fn every_event_names_its_terminal() {
        for spec in all() {
            assert_eq!(spec.field("terminal").map(|f| f.kind), Some(FieldKind::Id), "{:?}", spec.event);
        }
    }

    #[test]
    fn a_pattern_runs_over_a_text_field_of_the_event() {
        for spec in all() {
            if let Some(MatcherKind::Pattern { on }) = spec.matcher("pattern").map(|m| m.kind) {
                assert_eq!(spec.field(on).map(|f| f.kind), Some(FieldKind::Text), "{:?}", spec.event);
            }
        }
    }

    #[test]
    fn a_matcher_that_names_a_field_tests_a_field_of_that_kind() {
        for spec in all() {
            for m in spec.matchers {
                let want = match m.kind {
                    MatcherKind::Number => Some(FieldKind::Number),
                    MatcherKind::States => Some(FieldKind::State),
                    MatcherKind::Names => Some(FieldKind::Name),
                    MatcherKind::Globs => Some(FieldKind::Text),
                    _ => None,
                };
                if let Some(want) = want {
                    assert_eq!(spec.field(m.name).map(|f| f.kind), Some(want), "{:?} {}", spec.event, m.name);
                }
            }
        }
    }

    #[test]
    fn events_carry_no_text_but_what_the_table_names() {
        let text: Vec<_> = all()
            .iter()
            .flat_map(|s| {
                s.fields.iter().filter(|f| f.kind == FieldKind::Text).map(move |f| (s.event.name(), f.name))
            })
            .collect();
        assert_eq!(
            text,
            [
                ("command.started", "command"),
                ("command.finished", "command"),
                ("agent.status", "message"),
                ("output.match", "line"),
                ("git.changed", "branch"),
                ("folder.changed", "folder"),
                ("title", "title"),
                ("terminal.opened", "folder"),
            ]
        );
    }
}
