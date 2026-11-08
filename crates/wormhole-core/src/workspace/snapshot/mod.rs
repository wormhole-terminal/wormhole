//! What a shell pulls to draw: plain records with ids, numbers, strings and core's small enums,
//! built on demand from the model and never mirrored by the shell. Each part carries the revision of
//! its section (`Revisions`), so a view compares numbers, never values. The words are finished
//! (second lines, VoiceOver labels) in the one wording core's sidebar already has.

mod chrome;
mod panes;
mod sidebar;
mod terminals;
mod title;

use crate::view::panes::Axis;
use crate::view::sidebar::{Bracket, RowState, Urgency};

use super::tile::TileGlyph;
use super::words::SecondLine;

pub use chrome::{ChromeSnapshot, ColumnLimits, EdgeDrag, LayoutInfo, PanelTab, PresetInfo};
pub use terminals::{TerminalCounts, TerminalSummary, TerminalsSnapshot};
pub use title::{PathFolder, PathStep, TitleSlot, TitleSnapshot, TitleTerminal};

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
    /// The repository the terminal is in (its root): rows with the same one share it.
    pub repo: Option<String>,
    /// The working tree has uncommitted changes.
    pub dirty: bool,
    /// Commits the branch is ahead of and behind its upstream.
    pub ahead: u32,
    pub behind: u32,
    /// Files changed in the repository, whoever changed them (`changes` is this terminal's own).
    pub repo_files: Option<u32>,
    /// The mark on its tile.
    pub glyph: TileGlyph,
    /// "Hide from Agents" and "Forget text" are on (the row menu's checks).
    pub agent_hidden: bool,
    pub forget_text: bool,
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
    /// The folder as the person set it (a full path), for the folder picker to start from.
    pub folder_path: Option<String>,
    /// The group keeps only the terminals in its folder.
    pub only_in_folder: bool,
}

/// One terminal of a group as its folded head draws it: a tile.
#[derive(Clone, Debug, PartialEq)]
pub struct MemberTile {
    pub id: i64,
    pub name: String,
    pub glyph: TileGlyph,
    pub state: RowState,
    pub agent: Option<AgentInfo>,
    pub urgency: Urgency,
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
    /// What a folded head draws in the place of its rows: every terminal of the group that passes
    /// the search, the loudest first (equally loud ones in the list's order). Empty while open.
    pub tiles: Vec<MemberTile>,
    /// The loudest alert among them (`NeedsYou` or `Failed`), for the head's dot; `None` for no alert.
    pub alert: Option<Urgency>,
    /// Terminals of the group working now: closing the group asks first when it is not 0.
    pub working_count: u32,
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
    /// What is shown: every pane, or the zoomed one alone over the whole area.
    pub panes: Vec<PaneRect>,
    pub focused: Option<i64>,
    pub zoomed: Option<i64>,
    /// The lines between the panes (none while one is zoomed).
    pub dividers: Vec<DividerInfo>,
    /// Every pane at its place in the unzoomed layout, in reading order: what a zoom hides, which a shell
    /// keeps alive under the zoomed one, and the order the headers count in.
    pub unzoomed: Vec<PaneRect>,
}

/// A line between two panes, as `DragDivider` and `EqualizeDivider` name it.
#[derive(Clone, Debug, PartialEq)]
pub struct DividerInfo {
    pub path: Vec<u32>,
    pub index: u32,
    pub axis: Axis,
    /// Its place along its split's axis: x for a vertical line, y for a horizontal one.
    pub position: f32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
