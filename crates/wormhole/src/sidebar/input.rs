//! What the window tells the sidebar, and what the sidebar tells the window. The sidebar never reads
//! the `Workspace`: whenever its facts change (and before the window calls it for something that needs
//! the terminals as they are now) the window pushes a [`SidebarInput`], never while it draws, and what
//! the person does in the list comes back as a [`SidebarEvent`] the window handles when the update that
//! raised it ends.

use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::sidebar::SplitId;

use super::facts::RowInfo;
use crate::platform::vibrancy::Glass;
use crate::sidebar::terminal_drag::{DragOrigin, Over};

/// The window's facts the list is drawn and acted on from: its terminals as rows, in the window's
/// (manual) order, and the few things about the window the rows depend on.
pub(crate) struct SidebarInput {
    /// Every terminal's row facts.
    pub(crate) rows: Vec<RowInfo>,
    /// The selected terminal's place in `rows` (`usize::MAX` while the window has none).
    pub(crate) active: usize,
    /// The profile the window is in, whose settings and theme the list is drawn with.
    pub(crate) profile: ProfileId,
    /// The terminals are in tabs across the top, not in the sidebar.
    pub(crate) tabs_on: bool,
    /// The window's glass, while it is on: each card asks for a glass of its own.
    pub(crate) glass: Option<Rc<Glass>>,
    /// A question about closing a terminal or the window is up (a × shows no tooltip then).
    pub(crate) asking: bool,
    /// The terminal being dragged, by its row or its pane's header.
    pub(crate) terminal_drag: Option<(i64, DragOrigin)>,
    /// The group being dragged by its head.
    pub(crate) group_drag: Option<u32>,
    /// The terminals whose welcome program is running: the example band is drawn only for one of them.
    pub(crate) welcomes: Vec<i64>,
}

/// Whether the window says the same twice: the sidebar draws again for a different input only.
/// (Times are instants in the rows, so the same facts at two moments are equal; the glass is the
/// window's own, compared by identity.)
impl PartialEq for SidebarInput {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows
            && self.active == other.active
            && self.profile == other.profile
            && self.tabs_on == other.tabs_on
            && match (&self.glass, &other.glass) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
            && self.asking == other.asking
            && self.terminal_drag == other.terminal_drag
            && self.group_drag == other.group_drag
            && self.welcomes == other.welcomes
    }
}

impl Default for SidebarInput {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            active: usize::MAX,
            profile: ProfileId::default_profile(),
            tabs_on: false,
            glass: None,
            asking: false,
            terminal_drag: None,
            group_drag: None,
            welcomes: Vec::new(),
        }
    }
}

impl SidebarInput {
    /// The terminals' ids, in the window's order.
    pub(super) fn ids(&self) -> Vec<i64> {
        self.rows.iter().map(|row| row.id).collect()
    }

    /// Each terminal's split, in the window's order.
    pub(super) fn split_of(&self) -> Vec<Option<SplitId>> {
        self.rows.iter().map(|row| row.split).collect()
    }

    /// Each terminal's pinned state (a split is pinned as a whole), in the window's order.
    pub(super) fn pinned(&self) -> Vec<bool> {
        self.rows.iter().map(|row| row.pinned).collect()
    }

    /// Where terminal `id` is in `rows`.
    pub(super) fn find(&self, id: i64) -> Option<usize> {
        self.rows.iter().position(|row| row.id == id)
    }

    /// The selected terminal's id.
    pub(super) fn active_id(&self) -> Option<i64> {
        self.rows.get(self.active).map(|row| row.id)
    }

    /// Terminal `id` and the rest of its split: they move, join and leave groups together.
    pub(crate) fn with_split_mates(&self, id: i64) -> Vec<i64> {
        let Some(ix) = self.find(id) else { return Vec::new() };
        match self.rows[ix].split {
            Some(split) => {
                self.rows.iter().filter(|row| row.split == Some(split)).map(|row| row.id).collect()
            }
            None => vec![id],
        }
    }
}

/// How a new terminal is asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NewTerminal {
    /// Where the selected terminal is, in its group.
    Plain,
    /// With the shell of this key (Settings → Shells), as the + menu offers.
    Shell(String),
    /// In this group, in its folder if it has one.
    InGroup(u32),
    /// In a group of its own (the tab bar's "New group").
    InNewGroup,
}

/// What the list asks of the window. Delivered when the update that raised it ends, so a window never
/// runs one while the sidebar is still being updated.
#[derive(Clone, Debug)]
pub(crate) enum SidebarEvent {
    /// The saved view changed (a group made, folded, renamed, coloured, a terminal moved): save the window.
    Changed,
    /// Show terminal `id`; with `then_focus_list` the keyboard stays in the list (Space browses).
    Activate { id: i64, then_focus_list: bool },
    /// The × (or Close in a menu) on terminal `id`: close it, asking first as the rules say.
    Close(i64),
    /// A new terminal.
    NewTerminal(NewTerminal),
    /// "Split right" on terminal `id`.
    SplitRight(i64),
    /// "Remove from split" on terminal `id`.
    LeaveSplit(i64),
    /// Pin terminal `id` (with its split) to the top, or let it go.
    TogglePin(i64),
    /// "Don't keep this terminal's text" on terminal `id`.
    ToggleForgetText(i64),
    /// Terminal `other` shows beside terminal `id`.
    ShowBeside { other: i64, id: i64 },
    /// Terminal `id`'s menu was asked for at `at`: the window answers with the terminals it could show
    /// beside it, which only it knows ([`super::Sidebar::show_row_menu`]).
    RowMenu { id: i64, at: Point<Pixels> },
    /// Terminal `id` was renamed to `typed` (`Some("")`: back to what runs in it); `None`: left as it was.
    Renamed { id: i64, typed: Option<String> },
    /// The keyboard goes back to the terminal on screen.
    FocusTerminal,
    /// "Close group": every terminal in it ends, asking once first.
    CloseGroup(u32),
    /// "Move group to new window".
    MoveGroupToWindow(u32),
    /// A terminal was picked up by its row.
    TerminalDragStarted { id: i64, origin: DragOrigin },
    /// A group was picked up by its head.
    GroupDragStarted(u32),
    /// A drag of a group ended (dropped or not).
    GroupDragEnded,
    /// A terminal was dropped, on `over` if the pointer was over something of the list.
    TerminalDropped { dragged: i64, origin: DragOrigin, over: Option<Over> },
    /// ⇥ / ⇧⇥ in the list: the next or previous stop of the window's areas.
    StepStop(isize),
}
