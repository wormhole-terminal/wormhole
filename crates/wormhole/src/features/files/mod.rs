//! Files: the folder a terminal is in, as a tree in the right panel. A file opens in the viewer, a card laid
//! over the terminal as a diff is (`viewer`); Open in… hands it to an editor. The one write is Rename
//! (`rename`); nothing else changes a file or a folder.
//!
//! One [`Files`] entity per window holds, for each terminal of the window, the tree, the folder it is of
//! and the row that is selected ([`FilesOf`]). The tree follows the terminal's folder (its repository's
//! root, else the folder itself) until a folder is chosen from the path menu; Follow Terminal goes
//! back. What needs no GPUI is core's (`view::file_tree`: the tree, the order, the path menu's list, the
//! cap; `storage::files::listing`: reading a folder without leaving it by a link). Here are the reads off
//! the main thread (`read`), the rows' drawing (`view`, `row`), the menus (`menus`), the pins (`pins`), the viewer (`viewer`), Rename
//! (`rename`),
//! what a row hands to other apps (`open`), the keys over the rows (`keys`) and the table entry (`def`).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::storage::files::FileStamp;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::file_tree::{Row, Tree};

use crate::platform::Anchor;
use crate::style::panel_list::{CardPart, Rows};
use crate::window::facts::WindowFacts;
use crate::window::feature::FeatureEvent;

mod def;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod head;
mod keys;
mod menus;
mod open;
mod pins;
mod read;
mod rename;
#[cfg(test)]
mod rename_tests;
mod row;
#[cfg(test)]
mod tests;
mod view;
mod viewer;

pub(crate) use def::DEF;

/// How many ticks (400 ms each) pass between two looks at the folders on show: about two seconds.
const REFRESH_EVERY: u64 = 5;

/// A line of the list: the path button heading the card, a row of the tree, and the room under the last.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Line {
    Head,
    Row(Row),
    Spacer,
}

impl Line {
    /// The card is the head and the rows under it; the spacer is outside it.
    pub(super) fn part(&self) -> CardPart {
        match self {
            Self::Head => CardPart::Head,
            Self::Row(_) => CardPart::Body,
            Self::Spacer => CardPart::Outside,
        }
    }
}

/// The window's Files: every terminal's tree.
pub(crate) struct Files {
    of: HashMap<i64, FilesOf>,
    /// The window's profile as the feature last heard it: whose pinned folders the menus show and change.
    profile: ProfileId,
    /// The folders being read now, by their full path and whichever root they were asked for: a read
    /// cannot be cancelled, so one that hangs (a dead share) must not be started again each time the tree
    /// goes back to that root. Kept across roots and terminals.
    in_flight: HashSet<PathBuf>,
    /// The file shown in the terminal's place, if any: one for the window.
    viewer: Option<viewer::Viewer>,
    /// Counts the files shown, so a read for an earlier one is dropped.
    viewers: u64,
    /// The row whose name is being edited, if any: one for the window.
    renaming: Option<rename::Renaming>,
    /// A read of the open file is on its way: one at a time for the window, the latest request wins.
    fetching: bool,
    /// The syntax colors of the shown text are being made: likewise.
    colouring: bool,
    /// The window these are shown in, to drop a picture from its atlas.
    window: AnyWindowHandle,
    /// How many reads and syntax-color jobs were started, for the tests of one at a time.
    #[cfg(test)]
    started: (u64, u64),
}

impl EventEmitter<FeatureEvent> for Files {}

/// One terminal's tree.
pub(super) struct FilesOf {
    /// A folder chosen by hand in the path menu; `None` follows the terminal.
    chosen: Option<PathBuf>,
    /// The folder the tree is of: `chosen`, else the terminal's repository root or folder. `None`
    /// while the terminal has not said where it is.
    root: Option<PathBuf>,
    tree: Tree,
    /// The list as drawn, rebuilt from the tree when it changed (`stale`).
    list: Rows<Line>,
    stale: bool,
    /// The folder the terminal is in as its band names it (its repository's root, else the folder), as
    /// last heard; what the tree is of when nothing was chosen.
    followed: Option<PathBuf>,
    /// The selected row's path below the root.
    selected: Option<PathBuf>,
    /// What each folder read looked like from outside when it was read (`listing::stamp_below`), by its
    /// path below the root: a folder that still looks so is not listed again.
    stamps: HashMap<PathBuf, FileStamp>,
    /// Counts the roots the tree has been of: an answer for an earlier one is dropped.
    generation: u64,
    /// Where the path button is, for the menu its keys open.
    head_at: Anchor,
}

impl FilesOf {
    fn new() -> Self {
        Self {
            chosen: None,
            root: None,
            tree: Tree::new(),
            list: Rows::default(),
            stale: true,
            followed: None,
            selected: None,
            stamps: HashMap::new(),
            generation: 0,
            head_at: Anchor::default(),
        }
    }
}

impl Files {
    pub(crate) fn new(window: AnyWindowHandle) -> Self {
        Self {
            of: HashMap::new(),
            profile: ProfileId::default_profile(),
            in_flight: HashSet::new(),
            viewer: None,
            viewers: 0,
            renaming: None,
            fetching: false,
            colouring: false,
            window,
            #[cfg(test)]
            started: (0, 0),
        }
    }

    pub(super) fn of_mut(&mut self, session: i64) -> &mut FilesOf {
        self.of.entry(session).or_insert_with(FilesOf::new)
    }

    /// Takes in what the window says of itself: whose settings it draws with.
    pub(super) fn follow(&mut self, facts: &WindowFacts<'_>) {
        if self.profile != *facts.profile {
            self.profile = facts.profile.clone();
        }
    }

    /// A terminal is gone from this window (closed, or moved to another).
    pub(super) fn forget(&mut self, session: i64, cx: &mut Context<Self>) {
        self.of.remove(&session);
        self.forget_viewer(session, cx);
        if self.renaming.as_ref().is_some_and(|r| r.session() == session) {
            self.cancel_rename(cx);
        }
    }

    /// Another terminal was selected: its viewer and its rename belonged to the one before.
    pub(super) fn selected(&mut self, cx: &mut Context<Self>) {
        self.dismiss_viewer(cx);
        self.cancel_rename(cx);
    }
}
