//! The person's own groups in the list: making and taking them apart, folding, naming, pinning and
//! putting terminals in. Core keeps the groups and their order (`WindowSidebar`); a terminal moves
//! with the rest of its split.

use std::path::PathBuf;

use crate::view::sidebar::{GroupColor, Item, Landing, group_key};

use super::event::{Event, Refusal};
use super::model::WorkspaceModel;

impl WorkspaceModel {
    /// A new group of `members` (and the rest of their splits), standing where the first stood.
    /// Core makes a group of terminals, so a list with none that exist refuses.
    pub(super) fn new_group(&mut self, members: &[i64]) {
        let mut all: Vec<i64> = Vec::new();
        for &id in members {
            for mate in self.with_split_mates(id) {
                if !all.contains(&mate) {
                    all.push(mate);
                }
            }
        }
        if all.is_empty() {
            return self.emit(Event::Refused(Refusal::Unknown));
        }
        let ids = self.ids();
        self.sidebar.new_group(&ids, &all);
        self.rearranged();
    }

    /// Put `members` (terminals adopted from orphaned session hosts) in the Recovered group.
    pub(super) fn recover_group(&mut self, members: &[i64]) {
        let alive = self.ids();
        let members: Vec<i64> = members.iter().copied().filter(|id| alive.contains(id)).collect();
        if self.sidebar.groups.recover(&members).is_none() {
            return self.emit(Event::Refused(Refusal::Unknown));
        }
        self.rearranged();
    }

    pub(super) fn rename_group(&mut self, group: u32, name: &str) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        // A blank name keeps the old one (`Groups::rename`).
        self.sidebar.groups.rename(group, name);
        self.emit(Event::Listing);
    }

    pub(super) fn toggle_group(&mut self, group: u32) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.toggle_group_fold(group);
        // A folded group's terminals have no number: they are dealt again.
        self.rearranged();
    }

    pub(super) fn ungroup(&mut self, group: u32) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        let ids = self.ids();
        self.sidebar.ungroup(&ids, group);
        self.rearranged();
    }

    pub(super) fn set_group_pinned(&mut self, group: u32, pinned: bool) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.set_pinned(group, pinned);
        self.emit(Event::Listing);
    }

    /// Put terminal `id` (with the rest of its split) in group `group`, last in it.
    pub(super) fn add_to_group(&mut self, id: i64, group: u32) {
        if self.index_of(id).is_none() || self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        // A group that keeps only its folder's terminals takes none from outside it.
        if !self.admits(group, id) {
            return self.emit(Event::Refused(Refusal::Unchanged));
        }
        self.land(id, Landing::Group { group, before: None }, true);
    }

    /// Take a terminal (with the rest of its split) out of its group: it stands right after it.
    pub(super) fn remove_from_group(&mut self, id: i64) {
        let Some(group) = self.sidebar.groups.group_of(id) else { return self.refuse() };
        let ids = self.ids();
        let top = self.sidebar.arranged(&ids);
        let after =
            top.iter().position(|&item| item == Item::Group(group)).and_then(|at| top.get(at + 1)).copied();
        for member in self.with_split_mates(id) {
            self.sidebar.place_at_top(&ids, Item::Terminal(member), after);
        }
        self.rearranged();
    }

    /// A new terminal that starts in a new group of its own.
    pub(super) fn new_terminal_in_new_group(&mut self) {
        let cwd = self.start_folder();
        let id = self.open_terminal(cwd);
        let ids = self.ids();
        self.sidebar.new_group(&ids, &[id]);
        self.rearranged();
        self.select_new(id);
    }

    /// Where a new terminal in `group` starts: the group's folder when it has one that is still a
    /// folder, else where it was last used, else where the selected terminal is.
    pub(super) fn group_start_folder(&self, group: u32) -> Option<PathBuf> {
        let selected = self.selected;
        let last = self.last_selected.get(&Some(group)).copied();
        self.sidebar
            .groups
            .start_place(group, selected, last, |id| self.place_at(id))
            .or_else(|| self.start_folder())
    }

    /// A new terminal last in `group`, in its folder, selected.
    pub(super) fn new_terminal_in_group(&mut self, group: u32) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        let cwd = self.group_start_folder(group);
        let id = self.open_terminal(cwd);
        let ids = self.ids();
        self.sidebar.land(&ids, id, Landing::Group { group, before: None });
        self.rearranged();
        self.select_new(id);
    }

    /// The folder to ask the person about before a terminal opens in `group`, `None` when there is
    /// nothing to ask: it has a folder, was asked and said no, is the Recovered group, or nothing in
    /// it points at a folder worth offering. Answer with `SetGroupFolder` (and `SetOnlyInFolder`) or
    /// `DeclineGroupFolder`, then `NewTerminalInGroup`.
    pub fn group_folder_offer(&self, group: u32) -> Option<PathBuf> {
        let ground = self.cfg.ground.as_deref()?;
        let home = self.cfg.home.as_deref().and_then(|home| ground.real_folder(home))?;
        let last = self.last_selected.get(&Some(group)).copied();
        self.sidebar.groups.folder_to_ask_about(group, self.selected, last, |id| self.place_at(id), &home)
    }

    pub(super) fn set_group_color(&mut self, group: u32, color: Option<GroupColor>) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.set_color(group, color);
        self.emit(Event::Listing);
    }

    pub(super) fn set_group_custom_color(&mut self, group: u32, rgb: [u8; 3]) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.set_custom_color(group, rgb);
        self.emit(Event::Listing);
    }

    /// Where new terminals in `group` start, or none. A group with a folder is pinned, as a project
    /// is; the terminals outside a fenced group's new folder leave it.
    pub(super) fn set_group_folder(&mut self, group: u32, folder: Option<&str>) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.set_folder(group, folder);
        self.forget_places();
        self.emit(Event::Listing);
        self.keep_to_folder(group);
    }

    pub(super) fn set_only_in_folder(&mut self, group: u32, on: bool) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.set_only_in_folder(group, on);
        self.forget_places();
        self.emit(Event::Listing);
        if on {
            self.keep_to_folder(group);
        }
    }

    pub(super) fn decline_group_folder(&mut self, group: u32) {
        if self.sidebar.groups.get(group).is_none() {
            return self.refuse();
        }
        self.sidebar.groups.decline_folder(group);
        self.emit(Event::Listing);
    }

    /// Close every terminal of `group`, and the group unless it is pinned. Closing the window's last
    /// terminal starts a new one, as closing it by itself does.
    pub(super) fn close_group(&mut self, group: u32) {
        let Some(found) = self.sidebar.groups.get(group) else { return self.refuse() };
        let members = found.members.clone();
        for id in members {
            self.close_terminal(id);
        }
        let ends =
            self.sidebar.groups.get(group).is_some_and(|found| !found.pinned && found.members.is_empty());
        if ends {
            self.sidebar.groups.remove(group);
            self.sidebar.folded.remove(&group_key(group));
            self.rearranged();
        }
    }

    /// The group is to move to a window of its own: tell the shell which terminals go with it.
    pub(super) fn move_group_to_window(&mut self, group: u32) {
        let Some(found) = self.sidebar.groups.get(group) else { return self.refuse() };
        let mut terminals: Vec<i64> = Vec::new();
        for &id in &found.members.clone() {
            for mate in self.with_split_mates(id) {
                if !terminals.contains(&mate) {
                    terminals.push(mate);
                }
            }
        }
        if terminals.is_empty() {
            return self.emit(Event::Refused(Refusal::Unchanged));
        }
        self.emit(Event::GroupToWindow { group, terminals });
    }
}
