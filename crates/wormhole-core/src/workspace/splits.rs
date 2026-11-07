//! Splits: several terminals shown side by side as one. Every terminal is a row of the list; a split
//! is the layout they share (core `view::panes`, keyed by the terminals' ids) and each member names
//! it. What is on screen is the selected terminal's split, with the selected terminal's pane focused.
//!
//! The rules kept here, as in the app: a split has at least two members (one left dissolves it into a
//! plain row), its members stand next to each other in the list, they share the pin, and a new pane
//! is a new terminal.

use crate::view::panes::{Axis, Leave, PANE_MIN_W, TabPanes, Zone};
use crate::view::sidebar::SplitId;

use super::event::{Event, Refusal};
use super::model::{Split, WorkspaceModel};

impl WorkspaceModel {
    /// Whether the focused pane of `anchor`'s split has the room for a split along `axis`: both halves
    /// at least the minimum. Before the area is laid out nothing is known, and the split is allowed.
    fn can_split(&self, anchor: usize, axis: Axis) -> bool {
        let (w, h) = self.pane_area;
        if w <= 0.0 || h <= 0.0 {
            return true;
        }
        let (pane_w, pane_h) = match self.sessions[anchor].split.and_then(|id| self.split_of(id)) {
            Some(split) => {
                let focused = split.layout.focused();
                match split.layout.tree().rects(w, h).into_iter().find(|(id, _)| *id == focused) {
                    Some((_, rect)) => (rect.w, rect.h),
                    None => return false,
                }
            }
            None => (w, h),
        };
        match axis {
            Axis::Row => pane_w / 2.0 >= PANE_MIN_W,
            Axis::Column => pane_h / 2.0 >= self.room().min_h,
        }
    }

    /// A new terminal beside `anchor`, in its folder, joined into its split (made if it stood
    /// alone); it takes the selection. Nothing happens without the room.
    pub(super) fn split_beside(&mut self, anchor: i64, axis: Axis) {
        let Some(at) = self.index_of(anchor) else { return self.refuse() };
        if !self.can_split(at, axis) {
            return self.emit(Event::Refused(Refusal::TooSmall));
        }
        let mut layout = match self.sessions[at].split.and_then(|id| self.split_of(id)) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(anchor),
        };
        // The pane that splits is the anchor's, not whichever had the keyboard.
        layout.focus(anchor);
        let id = self.alloc_id();
        if !layout.split(axis, id) {
            return self.emit(Event::Refused(Refusal::Unchanged));
        }
        let cwd = self.sessions[at].facts.cwd.clone();
        let number = self.next_number();
        let mut session = super::session::Session::new(id, number);
        session.facts.shell = self.sessions[at].facts.shell.clone();
        self.sessions.push(session);
        self.emit(Event::Opened { id, cwd });
        let joiner = self.sessions.len() - 1;
        self.install_layout(at, joiner, layout);
        self.select_id(id);
    }

    /// Terminal `id` joins the split of `target`, on `zone`'s side of its pane, making the split when
    /// the target stands alone; it leaves its own split if it was in one. It is selected. Refused,
    /// with nothing changed, when either pane would end up under the minimum size, or `id` is in the
    /// split already.
    pub(super) fn join_split(&mut self, id: i64, target: i64, zone: Zone) {
        let (Some(at), Some(joiner)) = (self.index_of(target), self.index_of(id)) else {
            return self.emit(Event::Refused(Refusal::Unknown));
        };
        let mut layout = match self.sessions[at].split.and_then(|split| self.split_of(split)) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(target),
        };
        if let Err(error) = layout.join(target, zone, id, self.room()) {
            return self.emit(Event::Refused(error.into()));
        }
        // It fits: leave the old split, then make or update the new one.
        self.leave_layout(joiner);
        let (at, joiner) = (self.index_of(target).unwrap_or(at), self.index_of(id).unwrap_or(joiner));
        self.install_layout(at, joiner, layout);
        self.emit(Event::Moved { id });
        self.select_id(id);
    }

    /// Terminal `id` leaves its split and stands alone; the selection stays where it is.
    pub(super) fn leave_split(&mut self, id: i64) {
        match self.index_of(id) {
            Some(ix) if self.leave_layout(ix).is_some() => self.rearranged(),
            Some(_) => self.emit(Event::Refused(Refusal::Unchanged)),
            None => self.refuse(),
        }
    }

    pub(super) fn toggle_zoom(&mut self) {
        let Some(split) = self.selected_index().and_then(|ix| self.sessions[ix].split) else {
            return self.emit(Event::Refused(Refusal::Unchanged));
        };
        if self.split_mut(split).is_some_and(|split| split.layout.toggle_zoom()) {
            self.emit(Event::Layout);
        }
    }

    /// Take session `ix` out of its split's layout, without touching what is selected: the panes
    /// beside it take its room. A split left with one member is gone and that terminal stands alone.
    /// `None`, nothing changed, when it is in no split.
    pub(super) fn leave_layout(&mut self, ix: usize) -> Option<Leave> {
        let id = self.sessions.get(ix)?.id;
        let split_id = self.sessions[ix].split?;
        let left = self.split_mut(split_id).and_then(|split| split.layout.leave(id));
        self.sessions[ix].split = None;
        if left != Some(Leave::Shrunk) {
            // One pane is left, which is no split: whoever is still named by it stands alone.
            self.splits.retain(|split| split.id != split_id);
            for member in self.members(split_id) {
                self.sessions[member].split = None;
            }
        }
        self.emit(Event::Layout);
        self.emit(Event::Listing);
        Some(left.unwrap_or(Leave::Dissolved))
    }

    /// The id of session `anchor`'s split, made (with `layout`) when it stands alone.
    fn split_for(&mut self, anchor: usize, layout: &TabPanes) -> SplitId {
        let current = self.sessions[anchor].split;
        if let Some(id) = current.filter(|id| self.splits.iter().any(|split| split.id == *id)) {
            return id;
        }
        let id = self.alloc_id();
        self.splits.push(Split { id, layout: layout.clone() });
        id
    }

    /// Make `layout` session `anchor`'s split, with `joiner` in it: the split it had, or a new one.
    /// The joiner takes the anchor's pin (the pin is the split's) and stands in the split's card: its
    /// group, right after the other members.
    fn install_layout(&mut self, anchor: usize, joiner: usize, layout: TabPanes) {
        let id = self.split_for(anchor, &layout);
        if let Some(split) = self.split_mut(id) {
            split.layout = layout;
        }
        let pinned = self.sessions[anchor].pinned;
        self.sessions[anchor].split = Some(id);
        self.sessions[joiner].split = Some(id);
        self.sessions[joiner].pinned = pinned;
        let ids = self.ids();
        let joiner_id = self.sessions[joiner].id;
        let mates: Vec<i64> = self.members(id).into_iter().map(|m| self.sessions[m].id).collect();
        self.sidebar.seat_beside(&ids, joiner_id, &mates);
        self.rearranged();
        self.emit(Event::Layout);
    }
}
