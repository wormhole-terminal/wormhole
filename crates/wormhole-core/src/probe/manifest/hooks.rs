//! `[[contributes.hooks]]`: an event, a matcher and the actions to take
//! (`repo-docs/docs/research/probes-hooks-and-fold.md`, section 3).
//!
//! The spelling only. Which fields an event has, which matcher fields it takes and what an action may
//! name is `events`' table and `validate`'s; here the closed sets (the events, the action types, a
//! fold's target) are enums, so a typo is a parse error that lists the choices.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::contrib::Unknown;

/// What a hook listens to. The names are the `wh events` vocabulary where one exists
/// (`events::spec` says which topic each reads) and the ones `$.on` takes in code (E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum HookEvent {
    #[serde(rename = "command.started")]
    CommandStarted,
    #[serde(rename = "command.finished")]
    CommandFinished,
    #[serde(rename = "agent.status")]
    AgentStatus,
    /// A finished logical line matched the hook's `pattern`.
    #[serde(rename = "output.match")]
    OutputMatch,
    #[serde(rename = "git.changed")]
    GitChanged,
    #[serde(rename = "folder.changed")]
    FolderChanged,
    #[serde(rename = "bell")]
    Bell,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "terminal.opened")]
    TerminalOpened,
    #[serde(rename = "terminal.closed")]
    TerminalClosed,
}

impl HookEvent {
    pub const ALL: [HookEvent; 10] = [
        Self::CommandStarted,
        Self::CommandFinished,
        Self::AgentStatus,
        Self::OutputMatch,
        Self::GitChanged,
        Self::FolderChanged,
        Self::Bell,
        Self::Title,
        Self::TerminalOpened,
        Self::TerminalClosed,
    ];

    /// The name a manifest and `$.on` spell it with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::CommandStarted => "command.started",
            Self::CommandFinished => "command.finished",
            Self::AgentStatus => "agent.status",
            Self::OutputMatch => "output.match",
            Self::GitChanged => "git.changed",
            Self::FolderChanged => "folder.changed",
            Self::Bell => "bell",
            Self::Title => "title",
            Self::TerminalOpened => "terminal.opened",
            Self::TerminalClosed => "terminal.closed",
        }
    }
}

/// What a hook does. Observing and contributing only: nothing here blocks, rewrites or answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookActionKind {
    /// A notification through the app's rules.
    Notify,
    /// A tone on the matched span or block; display only.
    Highlight,
    /// Hide finished rows behind a marker that says how many; display only.
    Fold,
    /// Offer a link to a file; it never opens by itself.
    OpenPath,
    /// Run one of the Probe's background commands.
    Run,
    /// Set a declared status item.
    Status,
}

impl HookActionKind {
    /// The name a manifest spells it with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Notify => "notify",
            Self::Highlight => "highlight",
            Self::Fold => "fold",
            Self::OpenPath => "open-path",
            Self::Run => "run",
            Self::Status => "status",
        }
    }
}

/// The rows a fold hides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FoldTarget {
    /// The output of the command that finished.
    BlockOutput,
    /// The run of consecutive lines the pattern matched.
    MatchRun,
}

impl FoldTarget {
    pub const fn name(self) -> &'static str {
        match self {
            Self::BlockOutput => "block-output",
            Self::MatchRun => "match-run",
        }
    }
}

/// One action of a hook as one table: the fields each type reads are told apart by `validate`, which
/// can then name a field a type does not use (as for a rule's action).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HookAction {
    #[serde(rename = "type")]
    pub kind: HookActionKind,
    /// `notify`: a template of fixed words and `{field}`s.
    pub title: Option<String>,
    /// `notify`: a template.
    pub body: Option<String>,
    /// `highlight`, `status`: a design token's name.
    pub tone: Option<String>,
    /// `fold`.
    pub target: Option<FoldTarget>,
    /// `fold`: fewer finished lines than this are left alone.
    pub min_lines: Option<u32>,
    /// `fold`: rows kept visible at the start.
    pub keep_head: Option<u32>,
    /// `fold`: rows kept visible at the end.
    pub keep_tail: Option<u32>,
    /// `fold`: the marker's words. Fixed text: no field, no capture.
    pub label: Option<String>,
    /// `open-path`: a template for the file.
    pub path: Option<String>,
    /// `open-path`: a template for the line in it.
    pub line: Option<String>,
    /// `run`: the local id of one of this Probe's commands.
    pub command: Option<String>,
    /// `status`: the local id of one of this Probe's status items. `run`: the one its output feeds.
    pub item: Option<String>,
    /// `status`: a template.
    pub text: Option<String>,
    /// `status`: a template.
    pub tooltip: Option<String>,
    /// `status`: an icon's name.
    pub icon: Option<String>,
    /// `run`: seconds before the process is killed.
    pub timeout: Option<u64>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// `[[contributes.hooks]]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Hook {
    pub id: String,
    pub on: HookEvent,
    /// Fields ANDed. Every value is text (`exit = "!=0"`); which fields the event takes, and what each
    /// value may be, is `events`' table, so a field it does not know is a `validate` error naming the
    /// ones it does.
    #[serde(rename = "match", default)]
    pub matcher: BTreeMap<String, String>,
    #[serde(rename = "do")]
    pub actions: Vec<HookAction>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}
