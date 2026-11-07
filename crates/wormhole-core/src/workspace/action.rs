//! Everything a person can do to the window: what a shell sends, and the model decides what it means.

use crate::config::tab::TabId;
use crate::view::panes::{Axis, Zone};
use crate::view::sidebar::{Landing, Part};

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
}
