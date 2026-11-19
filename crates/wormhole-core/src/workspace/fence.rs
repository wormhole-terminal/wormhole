//! Groups that keep only the terminals in their folder (`Group::only_in_folder`): which terminals
//! such a group admits, and taking out the ones that are no longer inside. The rules are core's
//! (`view::sidebar::group_folder`: `Fence`, `keeps`, `members_outside`); this gathers each
//! terminal's place through the model's `Ground` and acts on the answer. What the disk said is kept
//! until a terminal's folder or a group's folder changes, so a drag that asks again at every
//! pointer move reads memory.

use std::path::Path;

use crate::view::sidebar::group_folder::{Fence, Place, keeps};
use crate::view::sidebar::{Drop, Item, Landing};

use super::ground::place_of;
use super::model::WorkspaceModel;

impl WorkspaceModel {
    /// Forget what the disk said of places and folders: a terminal changed folder, or a group's
    /// folder or fence did.
    pub(super) fn forget_places(&self) {
        self.places.borrow_mut().clear();
        self.fences.borrow_mut().clear();
    }

    /// Where terminal `id` is, made real; `None` when it has no folder yet or there is no disk to ask.
    pub(super) fn place_at(&self, id: i64) -> Option<Place> {
        let ground = self.cfg.ground.as_deref()?;
        let cwd = self.sessions[self.index_of(id)?].facts.cwd.clone()?;
        if let Some(known) = self.places.borrow().get(&cwd) {
            return known.clone();
        }
        let place = place_of(ground, &cwd);
        self.places.borrow_mut().insert(cwd, place.clone());
        place
    }

    /// What group `group` holds its terminals to, when it keeps only its folder's.
    fn fence_of(&self, group: u32) -> Option<Fence> {
        let ground = self.cfg.ground.as_deref()?;
        let folder = self.sidebar.groups.fenced_folder(group)?.to_string();
        if let Some(known) = self.fences.borrow().get(&folder) {
            return known.clone();
        }
        let fence = ground
            .real_folder(Path::new(&folder))
            .map(|real| Fence::new(real, |folder| ground.repo_of(folder)));
        self.fences.borrow_mut().insert(folder, fence.clone());
        fence
    }

    /// Whether terminal `id` and the rest of its split are inside group `group`'s fence: any pane is,
    /// or one's place isn't known. A group with no fence holds everything.
    pub(super) fn inside(&self, group: u32, id: i64) -> bool {
        let Some(fence) = self.fence_of(group) else { return true };
        keeps(&fence, self.with_split_mates(id).into_iter().map(|mate| self.place_at(mate)))
    }

    /// Whether terminal `id` alone is inside the fence: for one that leaves its split on the way in.
    fn inside_alone(&self, group: u32, id: i64) -> bool {
        let Some(fence) = self.fence_of(group) else { return true };
        keeps(&fence, [self.place_at(id)])
    }

    /// Whether terminal `id` (with its split) may be put in group `group`: one already in it may move
    /// within it, anything else must be inside the fence, when it has one.
    pub fn admits(&self, group: u32, id: i64) -> bool {
        self.sidebar.groups.group_of(id) == Some(group) || self.inside(group, id)
    }

    /// Whether the fences let a drop of `dragged` happen: into a group by landing in it or by joining
    /// the split of one of its terminals, which is allowed when that split is inside the folder.
    pub(super) fn fences_allow(&self, drop: Drop, dragged: i64) -> bool {
        match drop {
            // Leaving its split on the way in it lands alone, and is judged so even when it is in the group.
            Drop::Move { landing: Landing::Group { group, .. }, leave_split: true, .. } => {
                self.inside_alone(group, dragged)
            }
            Drop::Move { landing: Landing::Group { group, .. }, .. } => self.admits(group, dragged),
            Drop::Join { target } => self.sessions.get(target).is_none_or(|row| {
                self.sidebar
                    .groups
                    .group_of(row.id)
                    .is_none_or(|group| self.admits(group, dragged) || self.inside(group, row.id))
            }),
            Drop::Move { .. } => true,
        }
    }

    /// How many of `group`'s terminals a group keeping only the terminals in `folder` would let go (a split's panes each
    /// count): the folder question says it before the box is ticked. 0 when there is no disk to ask.
    pub fn members_leaving(&self, group: u32, folder: &Path) -> usize {
        let Some(ground) = self.cfg.ground.as_deref() else { return 0 };
        let Some(real) = ground.real_folder(folder) else { return 0 };
        let fence = Fence::new(real, |folder| ground.repo_of(folder));
        self.sidebar
            .groups
            .members_outside(
                group,
                &fence,
                |id| self.place_at(id),
                |id| self.index_of(id).and_then(|ix| self.sessions[ix].split),
            )
            .len()
    }

    /// Take the terminals out of `group` that it, keeping only its folder's, lets go: the ones in
    /// another folder (a split goes whole when none of its panes is in it). How many left.
    pub(super) fn keep_to_folder(&mut self, group: u32) -> usize {
        let Some(fence) = self.fence_of(group) else { return 0 };
        let leaving = self.sidebar.groups.members_outside(
            group,
            &fence,
            |id| self.place_at(id),
            |id| self.index_of(id).and_then(|ix| self.sessions[ix].split),
        );
        if leaving.is_empty() {
            return 0;
        }
        let ids = self.ids();
        let top = self.sidebar.arranged(&ids);
        let after =
            top.iter().position(|&item| item == Item::Group(group)).and_then(|at| top.get(at + 1)).copied();
        for &id in &leaving {
            self.sidebar.place_at_top(&ids, Item::Terminal(id), after);
        }
        self.rearranged();
        leaving.len()
    }

    /// A terminal's folder changed: it leaves its group when that keeps only its folder's terminals.
    pub(super) fn folder_changed(&mut self, id: i64) {
        self.forget_places();
        if let Some(group) = self.sidebar.groups.group_of(id) {
            self.keep_to_folder(group);
        }
    }
}
