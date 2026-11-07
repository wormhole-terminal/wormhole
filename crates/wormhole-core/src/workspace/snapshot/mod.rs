//! What a shell pulls to draw: plain records with ids, numbers, strings and core's small enums,
//! built on demand from the model and never mirrored by the shell. Each part carries the revision of
//! its section (`Revisions`), so a view compares numbers, never values. The words are finished
//! (second lines, VoiceOver labels) in the one wording core's sidebar already has.

mod panes;
mod sidebar;

use crate::config::tab::TabId;
use crate::view::sidebar::{Bracket, RowState};

use super::action::Overlay;
use super::words::SecondLine;

/// A coding agent as a row names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentInfo {
    /// The agent's command ("claude"), the key a shell knows it by.
    pub key: String,
    pub name: String,
    pub short_name: String,
}

/// How a row belongs to its split, if it is in one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitInfo {
    pub id: Option<i64>,
    /// Its place in the bracket along the rows of a split whose members are next to each other.
    pub bracket: Option<Bracket>,
    /// A member of a split that isn't together in this list: a chain mark stands in for the bracket.
    pub chain: bool,
    /// Another member of the selected terminal's split: it is on screen beside it.
    pub on_screen: bool,
    /// How many terminals the split has, this one included; 0 outside a split.
    pub members: u32,
}

/// One terminal's row.
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarRow {
    pub id: i64,
    pub name: String,
    /// ⌘1 to ⌘9, when the row has one, and the keycap that shows it ("⌘1").
    pub number: Option<u32>,
    pub keycap: Option<String>,
    pub selected: bool,
    pub pinned: bool,
    pub state: RowState,
    pub agent: Option<AgentInfo>,
    /// What the agent or program says it is doing or asking.
    pub message: Option<String>,
    pub branch: Option<String>,
    /// The terminal's folder as the list shows it (`~/Projects/app`).
    pub folder: Option<String>,
    pub unread: bool,
    pub bell: bool,
    pub failure_code: Option<i32>,
    /// Files changed, when known; notes and tasks still open.
    pub changes: Option<u32>,
    pub notes: u32,
    pub second_line: SecondLine,
    pub split: SplitInfo,
    /// Its place among the rows of its group (or of the top level, in no group), from 1.
    pub position: u32,
    pub of: u32,
    /// What VoiceOver reads for the row.
    pub spoken: String,
}

/// A group's head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupHead {
    pub id: u32,
    pub name: String,
    /// The colour's key ("blue", or a custom colour's own text); empty for none.
    pub color: String,
    pub pinned: bool,
    pub folded: bool,
    pub folder: Option<String>,
    /// Its place among the groups and terminals in no group, from 1.
    pub position: u32,
    pub of: u32,
    pub spoken: String,
}

/// A run of rows: a group's, or the terminals in no group between two groups.
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSection {
    /// What the section is known by across snapshots.
    pub key: String,
    /// `None` for a run of terminals in no group.
    pub head: Option<GroupHead>,
    /// Empty while the group is folded; `terminal_count` still says what is folded away.
    pub rows: Vec<SidebarRow>,
    /// Terminals in the section that pass the search, folded away or not.
    pub terminal_count: u32,
    /// Of those, the ones that need input or failed: a folded head shows them as its alert dot.
    pub alert_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarSnapshot {
    pub revision: u64,
    pub selected: Option<i64>,
    pub sections: Vec<SidebarSection>,
    /// Every terminal, whatever the search shows.
    pub terminal_count: u32,
    /// How long until words that move with time ("Working · 3m") read differently: arm one timer
    /// for it. `None` when no row's words move.
    pub refresh_in_ms: Option<u64>,
}

/// A pane's place in the area the shown split fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneRect {
    pub id: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub focused: bool,
}

/// The panes of the selected terminal's split (just the terminal, when it stands alone).
#[derive(Clone, Debug, PartialEq)]
pub struct PanesSnapshot {
    pub revision: u64,
    /// The split's id; `None` for a terminal on its own.
    pub split: Option<i64>,
    pub panes: Vec<PaneRect>,
    pub focused: Option<i64>,
    pub zoomed: Option<i64>,
}

/// The window's chrome the list and the panel depend on.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeSnapshot {
    pub revision: u64,
    pub sidebar_visible: bool,
    pub panel_visible: bool,
    pub panel_tab: TabId,
    pub search: String,
    pub overlay: Option<Overlay>,
}
