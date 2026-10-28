//! Files: the folder a terminal is in, as a tree in the right panel. Always read-only: it lists, opens
//! folders and hands a file to the app Open in… uses; nothing writes to a file (a viewer and Rename are
//! the next phase).
//!
//! One [`Files`] entity per window holds, for each terminal of the window, the tree, the folder it is of
//! and the row that is selected ([`FilesOf`]). The tree follows the terminal's folder (its repository's
//! root, else the folder itself) until a folder is chosen from the path menu; Follow Terminal goes
//! back. What needs no GPUI is core's (`view::file_tree`: the tree, the order, the path menu's list, the
//! cap; `storage::files::listing`: reading a folder without leaving it by a link). Here are the reads off
//! the main thread (`read`), the rows' drawing (`view`, `row`), the menus (`menus`), the pins (`pins`),
//! what a row hands to other apps (`open`), the keys over the rows (`keys`) and the table entry (`def`).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use gpui_kit::*;
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
mod row;
#[cfg(test)]
mod tests;
mod view;

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
    /// The window as the feature last heard it, for the settings the rows are drawn with.
    profile: ProfileId,
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
    /// Folders being read now, so a slow one is not asked for again every tick.
    reading: HashSet<PathBuf>,
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
            reading: HashSet::new(),
            generation: 0,
            head_at: Anchor::default(),
        }
    }
}

impl Files {
    pub(crate) fn new() -> Self {
        Self { of: HashMap::new(), profile: ProfileId::default_profile() }
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
    pub(super) fn forget(&mut self, session: i64) {
        self.of.remove(&session);
    }
}
