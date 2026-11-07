//! Core's workspace types as UniFFI sees them. Every type of `wormhole_core::workspace` that crosses
//! the seam is declared here with `uniffi::remote`, so Swift gets core's own records and enums with
//! no copy to keep in step: a field added in core fails to compile here until it is added below.
//! Only what holds a path (`Event`, `TerminalFacts`) has a hand-written twin, in `workspace.rs`.

use wormhole_core::config::tab::TabId;
use wormhole_core::view::panes::{Axis, Zone};
use wormhole_core::view::sidebar::{Bracket, Item, Landing, Part, RowState, Status};
use wormhole_core::workspace::{
    Action, AgentInfo, CardRef, ChromeSnapshot, DropPlan, GroupHead, Over, Overlay, PaneRect, PanesSnapshot,
    Refusal, Revisions, SecondLine, SidebarRow, SidebarSection, SidebarSnapshot, SplitInfo, Tone,
};

#[uniffi::remote(Enum)]
pub enum Axis {
    Row,
    Column,
}

#[uniffi::remote(Enum)]
pub enum Zone {
    Left,
    Right,
    Top,
    Bottom,
}

#[uniffi::remote(Enum)]
pub enum Part {
    Top,
    Middle,
    Bottom,
}

#[uniffi::remote(Enum)]
pub enum Item {
    Terminal(i64),
    Group(u32),
}

#[uniffi::remote(Enum)]
pub enum Landing {
    Top { before: Option<Item> },
    Group { group: u32, before: Option<i64> },
}

#[uniffi::remote(Enum)]
pub enum TabId {
    History,
    Notes,
    Usage,
    Project,
    Files,
    Changes,
}

#[uniffi::remote(Enum)]
pub enum Overlay {
    Palette,
    Shortcuts,
}

#[uniffi::remote(Enum)]
pub enum RowState {
    NeedsYou,
    Working,
    Finished,
    Failed,
    Idle,
    Exited,
}

#[uniffi::remote(Enum)]
pub enum Status {
    Idle,
    Busy,
    Done,
    Failed,
    Attention,
    Exited,
}

#[uniffi::remote(Enum)]
pub enum Bracket {
    Start,
    Mid,
    End,
}

#[uniffi::remote(Enum)]
pub enum Tone {
    Plain,
    Attention,
    Done,
    Failed,
}

#[uniffi::remote(Enum)]
pub enum Refusal {
    Unknown,
    TooSmall,
    Unchanged,
}

#[uniffi::remote(Enum)]
pub enum CardRef {
    Group(u32),
    Loose(i64),
    Split(i64),
}

#[uniffi::remote(Enum)]
pub enum Over {
    Row { id: i64, part: Part },
    Head { group: u32, part: Part },
    After(CardRef),
    Tail,
}

#[uniffi::remote(Enum)]
pub enum DropPlan {
    Move { landing: Landing, leave_split: bool, with_mates: bool },
    Join { target: i64 },
}

#[uniffi::remote(Enum)]
pub enum Action {
    Select(i64),
    SelectStep(i32),
    SelectNumber(u32),
    NewTerminal,
    Close(i64),
    Rename { id: i64, name: String },
    SetPinned { id: i64, pinned: bool },
    TogglePin(i64),
    NewGroup { members: Vec<i64> },
    RenameGroup { group: u32, name: String },
    ToggleGroup(u32),
    Ungroup(u32),
    SetGroupPinned { group: u32, pinned: bool },
    AddToGroup { id: i64, group: u32 },
    RemoveFromGroup(i64),
    DropTerminal { dragged: i64, over: Over },
    DropGroup { group: u32, over: Over },
    MoveTerminal { id: i64, landing: Landing },
    MoveGroup { group: u32, landing: Landing },
    SplitBeside { anchor: i64, axis: Axis },
    JoinSplit { id: i64, target: i64, zone: Zone },
    LeaveSplit(i64),
    FocusPane(i64),
    ToggleZoom,
    ToggleSidebar,
    TogglePanel,
    ShowPanelTab(TabId),
    Search(String),
    ShowOverlay(Option<Overlay>),
    SetPaneArea { width: f32, height: f32 },
}

#[uniffi::remote(Record)]
pub struct Revisions {
    pub sidebar: u64,
    pub layout: u64,
    pub chrome: u64,
}

#[uniffi::remote(Record)]
pub struct SecondLine {
    pub text: Option<String>,
    pub tone: Tone,
    pub mono: bool,
    pub trail: Option<String>,
}

#[uniffi::remote(Record)]
pub struct AgentInfo {
    pub key: String,
    pub name: String,
    pub short_name: String,
}

#[uniffi::remote(Record)]
pub struct SplitInfo {
    pub id: Option<i64>,
    pub bracket: Option<Bracket>,
    pub chain: bool,
    pub on_screen: bool,
    pub members: u32,
}

#[uniffi::remote(Record)]
pub struct SidebarRow {
    pub id: i64,
    pub name: String,
    pub number: Option<u32>,
    pub keycap: Option<String>,
    pub selected: bool,
    pub pinned: bool,
    pub state: RowState,
    pub agent: Option<AgentInfo>,
    pub message: Option<String>,
    pub branch: Option<String>,
    pub folder: Option<String>,
    pub unread: bool,
    pub bell: bool,
    pub failure_code: Option<i32>,
    pub changes: Option<u32>,
    pub notes: u32,
    pub second_line: SecondLine,
    pub split: SplitInfo,
    pub position: u32,
    pub of: u32,
    pub spoken: String,
}

#[uniffi::remote(Record)]
pub struct GroupHead {
    pub id: u32,
    pub name: String,
    pub color: String,
    pub pinned: bool,
    pub folded: bool,
    pub folder: Option<String>,
    pub position: u32,
    pub of: u32,
    pub spoken: String,
}

#[uniffi::remote(Record)]
pub struct SidebarSection {
    pub key: String,
    pub head: Option<GroupHead>,
    pub rows: Vec<SidebarRow>,
    pub terminal_count: u32,
    pub alert_count: u32,
}

#[uniffi::remote(Record)]
pub struct SidebarSnapshot {
    pub revision: u64,
    pub selected: Option<i64>,
    pub sections: Vec<SidebarSection>,
    pub terminal_count: u32,
    pub refresh_in_ms: Option<u64>,
}

#[uniffi::remote(Record)]
pub struct PaneRect {
    pub id: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub focused: bool,
}

#[uniffi::remote(Record)]
pub struct PanesSnapshot {
    pub revision: u64,
    pub split: Option<i64>,
    pub panes: Vec<PaneRect>,
    pub focused: Option<i64>,
    pub zoomed: Option<i64>,
}

#[uniffi::remote(Record)]
pub struct ChromeSnapshot {
    pub revision: u64,
    pub sidebar_visible: bool,
    pub panel_visible: bool,
    pub panel_tab: TabId,
    pub search: String,
    pub overlay: Option<Overlay>,
}
