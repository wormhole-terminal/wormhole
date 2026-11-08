//! View → Layout and the dividers: arranging the selected terminal's split as a preset (2 columns,
//! 2 rows, a 2x2 grid, a single pane) and moving the lines between panes. The rules are core's
//! (`view::panes`: `Preset`, `TabPanes::plan`/`arrange`/`drag`); this makes the terminals the
//! slots need and lets the ones over leave the split (they keep running).

use crate::view::panes::{Direction, DividerId, PANE_MIN_W, Preset, TabPanes, Zone};

use super::event::{Event, Refusal};
use super::model::WorkspaceModel;
use super::session::Session;

impl WorkspaceModel {
    /// Whether the area has room for `preset`'s slots, each pane at least the minimum. Before the
    /// area is laid out nothing is known and every preset is allowed.
    pub fn can_layout(&self, preset: Preset) -> bool {
        let (w, h) = self.pane_area;
        if w <= 0.0 || h <= 0.0 {
            return true;
        }
        let (columns, rows) = match preset {
            Preset::Single => (1.0, 1.0),
            Preset::TwoColumns => (2.0, 1.0),
            Preset::TwoRows => (1.0, 2.0),
            Preset::Grid => (2.0, 2.0),
        };
        w / columns >= PANE_MIN_W && h / rows >= self.room().min_h
    }

    /// The preset the selected terminal's split has the shape of (a terminal on its own is Single).
    pub(super) fn current_preset(&self) -> Option<Preset> {
        let ix = self.selected_index()?;
        match self.sessions[ix].split.and_then(|id| self.split_of(id)) {
            Some(split) => split.layout.preset(),
            None => Some(Preset::Single),
        }
    }

    /// Arrange the selected terminal's split as `preset`: the first panes in reading order fill the
    /// slots (the focused one always kept), new terminals fill what is left, in the anchor's
    /// folder and shell, and panes over the count leave the split and stand alone.
    pub(super) fn apply_layout(&mut self, preset: Preset) {
        let Some(anchor) = self.selected_index() else { return self.refuse() };
        if !self.can_layout(preset) {
            return self.emit(Event::Refused(Refusal::TooSmall));
        }
        let anchor_id = self.sessions[anchor].id;
        let mut layout = match self.sessions[anchor].split.and_then(|id| self.split_of(id)) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(anchor_id),
        };
        let plan = layout.plan(preset);
        let new_ids: Vec<i64> = (0..plan.new).map(|_| self.alloc_id()).collect();
        let Some(out) = layout.arrange(preset, &new_ids) else {
            return self.emit(Event::Refused(Refusal::Unchanged));
        };
        let (cwd, shell) = (self.start_folder(), self.sessions[anchor].facts.shell.clone());
        for &id in &new_ids {
            let number = self.next_number();
            let mut session = Session::new(id, number);
            session.facts.shell = shell.clone();
            self.sessions.push(session);
            self.emit(Event::Opened { id, cwd: cwd.clone() });
        }
        for pane in out {
            if let Some(ix) = self.index_of(pane) {
                self.sessions[ix].split = None;
            }
        }
        if layout.is_split() {
            for id in new_ids {
                let (anchor, joiner) = (self.index_of(anchor_id), self.index_of(id));
                if let (Some(anchor), Some(joiner)) = (anchor, joiner) {
                    self.install_layout(anchor, joiner, layout.clone());
                }
            }
        } else if let Some(split) = self.index_of(anchor_id).and_then(|ix| self.sessions[ix].split.take()) {
            // Down to one pane: no split.
            self.splits.retain(|known| known.id != split);
            for member in self.members(split) {
                self.sessions[member].split = None;
            }
        }
        self.rearranged();
        self.emit(Event::Layout);
        self.select_id(layout.focused());
    }

    /// Drag the divider to `position` in the shown area; held where a pane would drop under the
    /// minimum size.
    pub(super) fn drag_divider(&mut self, path: &[u32], index: u32, position: f32) {
        let (w, h) = self.pane_area;
        let room = self.room();
        let divider = DividerId { path: path.iter().map(|&at| at as usize).collect(), index: index as usize };
        let Some(split) = self.selected_split_mut() else { return self.refuse() };
        if split.layout.drag(&divider, position, w, h, room.min_w, room.min_h).is_some() {
            self.emit(Event::Layout);
        } else {
            self.emit(Event::Refused(Refusal::Unchanged));
        }
    }

    pub(super) fn equalize_divider(&mut self, path: &[u32], index: u32) {
        let divider = DividerId { path: path.iter().map(|&at| at as usize).collect(), index: index as usize };
        let Some(split) = self.selected_split_mut() else { return self.refuse() };
        if split.layout.equalize(&divider) {
            self.emit(Event::Layout);
        } else {
            self.emit(Event::Refused(Refusal::Unchanged));
        }
    }

    fn selected_split_mut(&mut self) -> Option<&mut super::model::Split> {
        let id = self.selected_index().and_then(|ix| self.sessions[ix].split)?;
        self.split_mut(id)
    }

    /// A pane's header dragged onto another pane's edge: it moves there inside its split and takes the keyboard.
    pub(super) fn move_pane(&mut self, id: i64, target: i64, zone: Zone) {
        let room = self.room();
        let Some(split) = self.index_of(id).and_then(|ix| self.sessions[ix].split) else {
            return self.refuse();
        };
        let Some(split) = self.split_mut(split) else { return self.refuse() };
        match split.layout.move_to(id, target, zone, room) {
            Ok(()) => {
                self.emit(Event::Layout);
                self.emit(Event::Moved { id });
                self.select_id(id);
            }
            Err(error) => self.emit(Event::Refused(error.into())),
        }
    }

    /// ⌥⌘← → ↑ ↓: the neighbouring pane of the shown split takes the keyboard. Before the area is laid
    /// out the panes' shares alone decide, in a unit square.
    pub(super) fn focus_direction(&mut self, direction: Direction) {
        let (w, h) =
            if self.pane_area.0 > 0.0 && self.pane_area.1 > 0.0 { self.pane_area } else { (1.0, 1.0) };
        let next = self
            .selected_index()
            .and_then(|ix| self.sessions[ix].split)
            .and_then(|id| self.split_of(id))
            .and_then(|split| split.layout.neighbour(direction, w, h));
        match next {
            Some(id) => self.select_id(id),
            None => self.emit(Event::Refused(Refusal::Unchanged)),
        }
    }

    /// Why a pane's header (or a row of the list) dropped on `target`'s `zone` would be refused, or
    /// nothing when it would work: a member of the split moves, any other terminal joins. Pure.
    pub fn pane_drop_refusal(&self, dragged: i64, target: i64, zone: Zone) -> Option<Refusal> {
        let layout = match self
            .index_of(target)
            .and_then(|ix| self.sessions[ix].split)
            .and_then(|id| self.split_of(id))
        {
            Some(split) => split.layout.clone(),
            None if self.index_of(target).is_some() => TabPanes::single(target),
            None => return Some(Refusal::Unknown),
        };
        let room = self.room();
        let verdict = if layout.contains(dragged) {
            layout.can_move_to(dragged, target, zone, room)
        } else {
            layout.can_join(target, zone, dragged, room)
        };
        verdict.err().map(Refusal::from)
    }
}
