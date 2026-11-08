//! Core's workspace types as UniFFI sees them. Every type of `wormhole_core::workspace` that crosses
//! the seam is declared here with `uniffi::remote`, so Swift gets core's own records and enums with
//! no copy to keep in step: a field added in core fails to compile here until it is added below.
//! Only what holds a path (`Event`, `TerminalFacts`) has a hand-written twin, in `workspace.rs`.

use wormhole_core::config::FeatureId;
use wormhole_core::config::tab::TabId;
use wormhole_core::git::pr::{Checks, PrState, PullRequest, Review};
use wormhole_core::view::layout::title_parts::TitlePart;
use wormhole_core::view::layout::{Edge, PanelToggle, Stop};
use wormhole_core::view::panes::{Axis, Direction, Preset, Zone};
use wormhole_core::view::sidebar::{Bracket, GroupColor, Item, Landing, Part, RowState, Status, Urgency};
use wormhole_core::workspace::{
    Action, AgentInfo, CardRef, ChromeSnapshot, ColumnLimits, DividerInfo, DropPlan, EdgeDrag, GroupHead,
    LayoutInfo, MemberTile, Over, Overlay, PaneRect, PanelTab, PanesSnapshot, PathFolder, PathStep,
    PresetInfo, Refusal, Revisions, SecondLine, SidebarRow, SidebarSection, SidebarSnapshot, SplitInfo,
    TerminalCounts, TerminalSummary, TerminalsSnapshot, TileGlyph, TitleSlot, TitleSnapshot, TitleTerminal,
    Tone,
};

#[uniffi::remote(Enum)]
pub enum Axis {
    Row,
    Column,
}

#[uniffi::remote(Enum)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
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
pub enum GroupColor {
    Grey,
    Blue,
    Red,
    Yellow,
    Green,
    Pink,
    Purple,
    Cyan,
    Orange,
}

#[uniffi::remote(Enum)]
pub enum Preset {
    Single,
    TwoColumns,
    TwoRows,
    Grid,
}

#[uniffi::remote(Enum)]
pub enum Edge {
    Sidebar,
    Panel,
}

#[uniffi::remote(Enum)]
pub enum Stop {
    Min,
    Max,
}

#[uniffi::remote(Enum)]
pub enum PanelToggle {
    Open,
    Closed { hides_sidebar: bool },
    Waiting,
}

#[uniffi::remote(Enum)]
pub enum FeatureId {
    Changes,
    History,
    Notes,
    Usage,
    Project,
    Worktrees,
    Files,
}

#[uniffi::remote(Enum)]
pub enum TitlePart {
    Folder,
    Branch,
    BranchStatus,
    Terminal,
    TerminalStatus,
    Space,
    AgentGauge,
    Ports,
    PullRequest,
    OpenIn,
    Splits,
}

#[uniffi::remote(Enum)]
pub enum Urgency {
    NeedsYou,
    Failed,
    Working,
    Bell,
    NewOutput,
    Running,
    Done,
    Idle,
}

#[uniffi::remote(Enum)]
pub enum TileGlyph {
    Working,
    NeedsYou,
    Done,
    Failed,
    Idle,
    Terminal { unread: bool },
    Shell,
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
    SetAgentHidden { id: i64, hidden: bool },
    ToggleAgentHidden(i64),
    SetForgetText { id: i64, forget: bool },
    ToggleForgetText(i64),
    NewGroup { members: Vec<i64> },
    RecoverGroup { members: Vec<i64> },
    RenameGroup { group: u32, name: String },
    ToggleGroup(u32),
    Ungroup(u32),
    SetGroupPinned { group: u32, pinned: bool },
    NewTerminalInNewGroup,
    NewTerminalInGroup(u32),
    SetGroupColor { group: u32, color: Option<GroupColor> },
    SetGroupCustomColor { group: u32, r: u8, g: u8, b: u8 },
    SetGroupFolder { group: u32, folder: Option<String> },
    SetOnlyInFolder { group: u32, on: bool },
    DeclineGroupFolder(u32),
    CloseGroup(u32),
    MoveGroupToWindow(u32),
    AddToGroup { id: i64, group: u32 },
    RemoveFromGroup(i64),
    DropTerminal { dragged: i64, over: Over },
    DropGroup { group: u32, over: Over },
    MoveTerminal { id: i64, landing: Landing },
    MoveTerminalAs { id: i64, landing: Landing, leave_split: bool, with_mates: bool },
    MoveGroup { group: u32, landing: Landing },
    SplitBeside { anchor: i64, axis: Axis },
    JoinSplit { id: i64, target: i64, zone: Zone },
    LeaveSplit(i64),
    ShowBeside { id: i64, other: i64 },
    ApplyLayout(Preset),
    DragDivider { path: Vec<u32>, index: u32, position: f32 },
    EqualizeDivider { path: Vec<u32>, index: u32 },
    MovePane { id: i64, target: i64, zone: Zone },
    FocusPane(i64),
    FocusDirection(Direction),
    ToggleZoom,
    ToggleSidebar,
    TogglePanel,
    ShowPanelTab(TabId),
    Search(String),
    ShowOverlay(Option<Overlay>),
    SetPaneArea { width: f32, height: f32 },
    SetWindowWidth(f32),
    DragEdge { edge: Edge, extent: f32 },
    ResetEdge(Edge),
    SetTitleRow(Vec<TitlePart>),
    SetNumbersFollowList(bool),
    SetFeature { feature: FeatureId, on: bool },
}

#[uniffi::remote(Record)]
pub struct Revisions {
    pub sidebar: u64,
    pub layout: u64,
    pub chrome: u64,
    pub terminals: u64,
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
    pub repo: Option<String>,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub repo_files: Option<u32>,
    pub glyph: TileGlyph,
    pub agent_hidden: bool,
    pub forget_text: bool,
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
    pub folder_path: Option<String>,
    pub only_in_folder: bool,
}

#[uniffi::remote(Record)]
pub struct MemberTile {
    pub id: i64,
    pub name: String,
    pub glyph: TileGlyph,
    pub state: RowState,
    pub agent: Option<AgentInfo>,
    pub urgency: Urgency,
}

#[uniffi::remote(Record)]
pub struct SidebarSection {
    pub key: String,
    pub head: Option<GroupHead>,
    pub rows: Vec<SidebarRow>,
    pub terminal_count: u32,
    pub alert_count: u32,
    pub tiles: Vec<MemberTile>,
    pub alert: Option<Urgency>,
    pub working_count: u32,
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
    pub dividers: Vec<DividerInfo>,
    pub unzoomed: Vec<PaneRect>,
}

#[uniffi::remote(Record)]
pub struct DividerInfo {
    pub path: Vec<u32>,
    pub index: u32,
    pub axis: Axis,
    pub position: f32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[uniffi::remote(Record)]
pub struct PanelTab {
    pub tab: TabId,
    pub key: String,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
}

#[uniffi::remote(Record)]
pub struct PresetInfo {
    pub preset: Preset,
    pub title: String,
    pub slots: u32,
    pub current: bool,
    pub enabled: bool,
}

#[uniffi::remote(Record)]
pub struct LayoutInfo {
    pub presets: Vec<PresetInfo>,
    pub can_split_row: bool,
    pub can_split_column: bool,
    pub pane_count: u32,
    pub zoomed: bool,
}

#[uniffi::remote(Record)]
pub struct EdgeDrag {
    pub open: bool,
    pub width: f32,
    pub stop: Option<Stop>,
    pub readout: String,
}

#[uniffi::remote(Record)]
pub struct ColumnLimits {
    pub sidebar_min: f32,
    pub sidebar_max: f32,
    pub panel_min: f32,
    pub panel_max: f32,
    pub terminal_min: f32,
    pub window_min_width: f32,
    pub window_min_height: f32,
}

#[uniffi::remote(Record)]
pub struct ChromeSnapshot {
    pub revision: u64,
    pub sidebar_visible: bool,
    pub panel_visible: bool,
    pub panel_tab: TabId,
    pub search: String,
    pub overlay: Option<Overlay>,
    pub panel_wanted: bool,
    pub panel_toggle: PanelToggle,
    pub window_width: f32,
    pub sidebar_width: f32,
    pub terminal_width: f32,
    pub panel_width: f32,
    pub sidebar_chosen_width: f32,
    pub panel_chosen_width: f32,
    pub sidebar_max_now: f32,
    pub panel_max_now: f32,
    pub limits: ColumnLimits,
    pub tabs: Vec<PanelTab>,
    pub layout: LayoutInfo,
    pub selected: Option<i64>,
    pub zoomed: Option<i64>,
    pub numbers_follow_list: bool,
}

#[uniffi::remote(Record)]
pub struct TerminalSummary {
    pub id: i64,
    pub name: String,
    pub number: Option<u32>,
    pub selected: bool,
    pub pinned: bool,
    pub state: RowState,
    pub since: u64,
    pub glyph: TileGlyph,
    pub agent: Option<AgentInfo>,
    pub message: Option<String>,
    pub branch: Option<String>,
    pub folder: Option<String>,
    pub cwd: Option<String>,
    pub command: Option<String>,
    pub group: Option<u32>,
    pub group_name: Option<String>,
    pub split: Option<i64>,
    pub folded: bool,
    pub agent_hidden: bool,
    pub forget_text: bool,
    pub unread: bool,
    pub bell: bool,
}

#[uniffi::remote(Record)]
pub struct TerminalCounts {
    pub needs_you: u32,
    pub working: u32,
    pub finished: u32,
    pub failed: u32,
    pub quiet: u32,
}

#[uniffi::remote(Record)]
pub struct TerminalsSnapshot {
    pub revision: u64,
    pub selected: Option<i64>,
    pub terminals: Vec<TerminalSummary>,
    pub counts: TerminalCounts,
}

#[uniffi::remote(Record)]
pub struct TitleSlot {
    pub part: TitlePart,
    pub present: bool,
    pub chevron_before: bool,
}

#[uniffi::remote(Record)]
pub struct TitleTerminal {
    pub id: i64,
    pub name: String,
    pub agent: Option<AgentInfo>,
    pub glyph: TileGlyph,
    pub state: RowState,
    pub folder: Option<String>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub message: Option<String>,
}

#[uniffi::remote(Record)]
pub struct PathStep {
    pub parents: String,
    pub last: String,
}

#[uniffi::remote(Record)]
pub struct PathFolder {
    pub path: String,
    pub name: String,
}

#[uniffi::remote(Record)]
pub struct TitleSnapshot {
    pub revision: u64,
    pub terminal: Option<TitleTerminal>,
    pub left: Vec<TitleSlot>,
    pub right: Vec<TitleSlot>,
    pub hidden: Vec<TitlePart>,
    pub path_steps: Vec<PathStep>,
    pub path_menu: Vec<PathFolder>,
    pub ports: Vec<u16>,
    pub pull_request: Option<PullRequest>,
    pub pull_request_words: Option<String>,
}

#[uniffi::remote(Enum)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

#[uniffi::remote(Enum)]
pub enum Checks {
    Pending,
    Failing,
    Passing,
}

#[uniffi::remote(Enum)]
pub enum Review {
    Approved,
    ChangesRequested,
    Required,
}

#[uniffi::remote(Record)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
    pub state: PrState,
    pub draft: bool,
    pub checks: Option<Checks>,
    pub review: Option<Review>,
}
