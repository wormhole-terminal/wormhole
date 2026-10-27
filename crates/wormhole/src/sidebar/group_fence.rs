//! What the list remembers about its groups' places: the terminal each group was last on (where a new
//! terminal in a group without a folder starts), and which terminals a group that keeps only the ones in its
//! folder will take. The first is noted where the window tells the list a fact; the second is worked out
//! when something is put in a group, and while a drag is on the folders and fences are found once, at its
//! start (`set_input`), so the hover that asks again each frame reads what was found.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wormhole_core::view::sidebar::group_folder::{Fence, Place, keeps, real_folder};

use super::{Sidebar, SidebarInput};

/// The repository `folder` is in, for a group's fence: its root, or, for a linked worktree, the main working
/// tree it belongs to (read from its `.git` file), so the repository's other worktrees count as inside.
pub(crate) fn repo_of_folder(folder: &Path) -> Option<PathBuf> {
    let root = wormhole_core::git::find_root(folder)?;
    Some(wormhole_core::git::worktree_of(&root).unwrap_or(root))
}

/// The list's memory of its groups' places.
#[derive(Default)]
pub(super) struct Memory {
    /// The terminal last selected in each group (`None`: in no group), by the id the window told the list
    /// was the active one. A selection by any road gets here, closing a terminal or moving one included,
    /// because the list is told each time.
    last_selected: HashMap<Option<u32>, i64>,
    /// Folders as they really are, and the fences of the groups that keep only their folder's terminals,
    /// found while a drag is on. Only then: at any other time they are found when asked, so a folder that
    /// changed is never read from here.
    real: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
    fences: RefCell<HashMap<u32, Option<Fence>>>,
}

impl Memory {
    /// Forget what a drag found.
    pub(super) fn forget_folders(&self) {
        self.real.borrow_mut().clear();
        self.fences.borrow_mut().clear();
    }
}

impl Sidebar {
    /// The active terminal is the one its group was last on.
    pub(super) fn note_selected(&mut self) {
        if let Some(id) = self.input.active_id() {
            self.memory.last_selected.insert(self.view.groups.group_of(id), id);
        }
    }

    /// The terminal last selected in `group` (`None`: among the terminals in no group).
    pub(crate) fn last_selected(&self, group: Option<u32>) -> Option<i64> {
        self.memory.last_selected.get(&group).copied()
    }

    /// Whether a drag is on, for which the folders found are kept.
    pub(super) fn dragging(&self) -> bool {
        self.input.terminal_drag.is_some() || self.input.group_drag.is_some()
    }

    /// `path` as it really is, kept while a drag is on.
    fn real_of(&self, path: &Path) -> Option<PathBuf> {
        if !self.dragging() {
            return real_folder(path);
        }
        let mut real = self.memory.real.borrow_mut();
        real.entry(path.to_path_buf()).or_insert_with(|| real_folder(path)).clone()
    }

    /// What group `group` holds its terminals to, when it keeps only its folder's: kept while a drag is on.
    fn fence_of(&self, group: u32) -> Option<Fence> {
        let find = || {
            let folder = self.real_of(Path::new(self.view.groups.fenced_folder(group)?))?;
            Some(Fence::new(folder, repo_of_folder))
        };
        if !self.dragging() {
            return find();
        }
        if let Some(known) = self.memory.fences.borrow().get(&group) {
            return known.clone();
        }
        let fence = find();
        self.memory.fences.borrow_mut().insert(group, fence.clone());
        fence
    }

    /// A drag has just begun: find what its hover will ask for, so no frame is the one that looks at the disk
    /// (each fence, the dragged terminals and the members of the groups that keep a fence, which are what
    /// a drop joins, are found here once).
    pub(super) fn warm_drag(&self) {
        let fenced: Vec<u32> = self.view.groups.iter().filter(|g| g.only_in_folder).map(|g| g.id).collect();
        let dragged = self.input.terminal_drag.map(|(id, _)| id);
        let mut places: Vec<i64> =
            dragged.into_iter().flat_map(|id| self.input.with_split_mates(id)).collect();
        for &group in &fenced {
            self.fence_of(group);
            places.extend(self.view.groups.get(group).into_iter().flat_map(|g| g.members.iter().copied()));
        }
        for id in places {
            self.place_of(&self.input, id);
        }
    }

    /// Where terminal `id` is, as its row says: its folder made real and the repository it is a worktree of.
    fn place_of(&self, input: &SidebarInput, id: i64) -> Option<Place> {
        let row = &input.rows[input.find(id)?];
        Some(Place {
            folder: self.real_of(row.cwd.as_deref()?)?,
            repo: None,
            // As the `.git` file names it, not looked up: it can lead anywhere, a network volume included.
            linked_to: row.worktree.clone(),
        })
    }

    /// Whether terminal `id` and the rest of its split are inside the fence of group `group`: any pane
    /// is, or one isn't known. A group that keeps no fence holds everything.
    pub(crate) fn inside(&self, input: &SidebarInput, group: u32, id: i64) -> bool {
        let Some(fence) = self.fence_of(group) else { return true };
        keeps(&fence, input.with_split_mates(id).into_iter().map(|mate| self.place_of(input, mate)))
    }

    /// Whether terminal `id` alone is inside the fence of group `group`: for a terminal that leaves its
    /// split on the way in, which brings none of its mates and has no place in the group already.
    pub(crate) fn inside_alone(&self, input: &SidebarInput, group: u32, id: i64) -> bool {
        let Some(fence) = self.fence_of(group) else { return true };
        keeps(&fence, [self.place_of(input, id)])
    }

    /// Whether terminal `id` (with the rest of its split) may be put in group `group`, which, when it keeps
    /// only the terminals in its folder, takes none from outside. One already in the group may move in it.
    pub(crate) fn admits(&self, input: &SidebarInput, group: u32, id: i64) -> bool {
        self.view.groups.group_of(id) == Some(group) || self.inside(input, group, id)
    }
}
