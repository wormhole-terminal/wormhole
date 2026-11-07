//! Everything a person can do to the window: what a shell sends, and the model decides what it means.

use crate::config::FeatureId;
use crate::config::tab::TabId;
use crate::view::layout::Edge;
use crate::view::layout::title_parts::TitlePart;
use crate::view::panes::{Axis, Preset, Zone};
use crate::view::sidebar::{GroupColor, Landing, Part};

/// What is laid over the window, at most one at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Palette,
    Shortcuts,
}

/// The card a gap in the list sits under, by id (the model finds its place in the listing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardRef {
    /// A group's head and rows.
    Group(u32),
    /// A terminal in no group.
    Loose(i64),
    /// The terminals of a split in no group, named by the first of them.
    Split(i64),
}

/// What the pointer is over while a row or a head is dragged, by id: the shell hit-tests, the model
/// decides (`view::sidebar::Target` is the same by place in the listing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Over {
    /// A terminal's row: its top or bottom quarter reorders, its middle joins a split.
    Row { id: i64, part: Part },
    /// A group's head: the top quarter stands before the group, the rest joins it.
    Head { group: u32, part: Part },
    /// The gap under a card.
    After(CardRef),
    /// The space under the last line.
    Tail,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    // Selection.
    Select(i64),
    /// The next (positive) or previous terminal as listed, wrapping round; folded rows are skipped.
    SelectStep(i32),
    /// ⌘1 to ⌘9: the terminal that holds the number, the last numbered one for a number past them.
    SelectNumber(u32),
    // Terminals.
    /// A new terminal in the selected one's folder, selected.
    NewTerminal,
    Close(i64),
    /// A blank name goes back to what runs in the terminal.
    Rename {
        id: i64,
        name: String,
    },
    SetPinned {
        id: i64,
        pinned: bool,
    },
    TogglePin(i64),
    /// "Hide from Agents": a program that controls Wormhole cannot list, read or type into it.
    SetAgentHidden {
        id: i64,
        hidden: bool,
    },
    ToggleAgentHidden(i64),
    /// "Forget text": the terminal's screen is not saved with the window; setting it asks the shell
    /// to erase what was saved (`Event::SavedTextForgotten`).
    SetForgetText {
        id: i64,
        forget: bool,
    },
    ToggleForgetText(i64),
    // Groups.
    NewGroup {
        members: Vec<i64>,
    },
    RenameGroup {
        group: u32,
        name: String,
    },
    ToggleGroup(u32),
    /// Take the group apart: its terminals stand where it stood.
    Ungroup(u32),
    SetGroupPinned {
        group: u32,
        pinned: bool,
    },
    /// A new terminal that starts in a new group of its own, which names itself.
    NewTerminalInNewGroup,
    /// A new terminal in the group, in its folder (else where it was last used), selected. Whether a
    /// group without a folder is asked for one first is the shell's (`group_folder_offer`).
    NewTerminalInGroup(u32),
    /// A group's colour: one of the nine names, or none.
    SetGroupColor {
        group: u32,
        color: Option<GroupColor>,
    },
    /// A colour of the person's own (sRGB), in the place of a name.
    SetGroupCustomColor {
        group: u32,
        r: u8,
        g: u8,
        b: u8,
    },
    /// Where new terminals in the group start, or none. A group with a folder is pinned; clearing it
    /// counts as saying no to having one.
    SetGroupFolder {
        group: u32,
        folder: Option<String>,
    },
    /// The group keeps only the terminals in its folder; the ones outside leave it now.
    SetOnlyInFolder {
        group: u32,
        on: bool,
    },
    /// The person said no to a folder for the group: it is not asked again.
    DeclineGroupFolder(u32),
    /// Close every terminal of the group (and the group, unless it is pinned). The shell asks first
    /// when a row of the group `needs_confirmation`.
    CloseGroup(u32),
    /// Take the group's terminals (and their splits) to a window of their own: the model tells the
    /// shell (`Event::GroupToWindow`) and leaves them here until `take_group` hands them over.
    MoveGroupToWindow(u32),
    AddToGroup {
        id: i64,
        group: u32,
    },
    /// Take the terminal (with its split) out of its group: it stands right after it.
    RemoveFromGroup(i64),
    // Moves.
    DropTerminal {
        dragged: i64,
        over: Over,
    },
    DropGroup {
        group: u32,
        over: Over,
    },
    MoveTerminal {
        id: i64,
        landing: Landing,
    },
    /// A drop's plan applied as `drop_plan` gave it: the terminal lands at `landing`, leaving its
    /// split first when `leave_split`, with the rest of its split when `with_mates`.
    MoveTerminalAs {
        id: i64,
        landing: Landing,
        leave_split: bool,
        with_mates: bool,
    },
    MoveGroup {
        group: u32,
        landing: Landing,
    },
    // Splits.
    /// A new terminal beside `anchor` (along `axis`), in its folder, joined into its split (made if
    /// it stood alone); it takes the selection. Nothing happens without the room.
    SplitBeside {
        anchor: i64,
        axis: Axis,
    },
    /// The terminal `id` joins the split of `target`, on `zone`'s side of its pane.
    JoinSplit {
        id: i64,
        target: i64,
        zone: Zone,
    },
    LeaveSplit(i64),
    /// "Show beside": terminal `other` joins the split of `id`, to the right of its pane.
    ShowBeside {
        id: i64,
        other: i64,
    },
    /// View → Layout: the selected terminal's split arranged as the preset, new terminals filling
    /// the slots it adds, the panes over its count leaving the split (they keep running).
    ApplyLayout(Preset),
    /// A divider dragged to `position` (x in a row's, y in a column's) in the shown area.
    DragDivider {
        path: Vec<u32>,
        index: u32,
        position: f32,
    },
    /// A double-click on a divider: equal shares for the split it belongs to.
    EqualizeDivider {
        path: Vec<u32>,
        index: u32,
    },
    /// Select a pane of the shown split.
    FocusPane(i64),
    ToggleZoom,
    // The window's chrome.
    ToggleSidebar,
    TogglePanel,
    /// Asking for a tab is asking to see it.
    ShowPanelTab(TabId),
    Search(String),
    ShowOverlay(Option<Overlay>),
    /// The area the panes share, in points; a zero is "not laid out yet", where nothing is refused
    /// for want of room.
    SetPaneArea {
        width: f32,
        height: f32,
    },
    /// The window's width: whether the panel has room, and how wide the columns are, follow it.
    SetWindowWidth(f32),
    /// A drag on a column's edge with the pointer `extent` from the window edge it hangs on.
    DragEdge {
        edge: Edge,
        extent: f32,
    },
    /// A double-click on a column's edge: back to its default width.
    ResetEdge(Edge),
    /// The title row's parts, shown and in order (`title_row` in `config.json`); a space is added
    /// where the default has it when the list has none.
    SetTitleRow(Vec<TitlePart>),
    /// ⌘1 to ⌘9 count along the list as shown (`true`) or stay with their terminal (`false`).
    SetNumbersFollowList(bool),
    /// A feature was switched on or off in Settings: its panel tab comes or goes.
    SetFeature {
        feature: FeatureId,
        on: bool,
    },
}
