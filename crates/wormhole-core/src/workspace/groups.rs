//! The person's own groups in the list: making and taking them apart, folding, naming, pinning and
//! putting terminals in. Core keeps the groups and their order (`WindowSidebar`); a terminal moves
//! with the rest of its split.

use crate::view::sidebar::{Item, Landing};

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
}
