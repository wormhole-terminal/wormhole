//! Splits: several terminals shown side by side as one. Every terminal is a session,
//! a row of the sidebar; a split is the layout they share (core `panes`, keyed by the sessions'
//! ids) and each member names it in `Session::split`. What is on screen is the selected
//! terminal's split, with the selected terminal's pane focused: selecting a row of a split shows it
//! all, and the keyboard coming into a pane selects its row.
//!
//! The rules kept here: a split has at least two members (one left dissolves it into a plain row),
//! a split's members are next to each other in the window's order, they share the pin, and a new
//! pane is a new session. Drawing a split is `panes.rs`.

use wormhole_core::panes::{
    Axis, DropError, Leave, PANE_MIN_H, PANE_MIN_W, PaneId, Preset, Room, TabPanes, Zone,
};
use wormhole_core::sidebar::SplitId;
use wormhole_core::store::{SessionRecord, SplitRecord};

use super::*;

/// A split: its id (one counter with the sessions') and the layout its members share.
pub(super) struct Split {
    pub id: SplitId,
    pub layout: TabPanes,
}

impl Split {
    /// Whether pane `id` is on screen while the split is: every pane is, or the zoomed one alone.
    pub(super) fn shows(&self, id: PaneId) -> bool {
        self.layout.zoomed().is_none_or(|zoomed| zoomed == id)
    }
}

impl Workspace {
    /// The session whose terminal has this id (what hooks, notifications and the control socket
    /// name: a terminal's id is its session's).
    pub(super) fn find_session(&self, id: i64) -> Option<usize> {
        self.sessions.iter().position(|session| session.id == id)
    }

    /// A notification about terminal `id` was clicked: bring it forward, with its split.
    pub(super) fn reveal_terminal(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.find_session(id) {
            self.activate(ix, window, cx);
        }
    }

    /// The terminal with this id (what agent hooks and the control socket name), in whichever
    /// session of this window.
    pub(crate) fn pane_terminal(&self, id: PaneId) -> Option<Entity<TerminalView>> {
        Some(self.sessions[self.find_session(id)?].terminal().clone())
    }

    /// The split session `ix` is in.
    pub(super) fn split_of(&self, ix: usize) -> Option<&Split> {
        let id = self.sessions.get(ix)?.split?;
        self.splits.iter().find(|split| split.id == id)
    }

    /// The selected terminal's split: what is on screen beside it.
    pub(super) fn active_split(&self) -> Option<&Split> {
        self.split_of(self.active)
    }

    pub(super) fn split_mut(&mut self, id: SplitId) -> Option<&mut Split> {
        self.splits.iter_mut().find(|split| split.id == id)
    }

    /// The selected terminal's split, to change it.
    pub(super) fn active_split_mut(&mut self) -> Option<&mut Split> {
        let id = self.sessions.get(self.active)?.split?;
        self.split_mut(id)
    }

    /// Each session's split, in the window's order, for the sidebar's list.
    pub(super) fn split_ids(&self) -> Vec<Option<SplitId>> {
        self.sessions.iter().map(|session| session.split).collect()
    }

    /// The sessions in split `id`, in the window's order.
    pub(super) fn members(&self, id: SplitId) -> Vec<usize> {
        (0..self.sessions.len()).filter(|&ix| self.sessions[ix].split == Some(id)).collect()
    }

    /// Whether the terminal of session `ix` is in front of the user: the selected one, or another
    /// pane of its split that isn't hidden by a zoom. Those are the ones that count as seen.
    pub(super) fn on_screen(&self, ix: usize) -> bool {
        if ix == self.active {
            return true;
        }
        let (Some(session), Some(split)) = (self.sessions.get(ix), self.active_split()) else { return false };
        session.split == self.sessions[self.active].split && split.shows(session.id)
    }

    /// The sessions on screen, in the window's order.
    pub(super) fn on_screen_sessions(&self) -> Vec<usize> {
        (0..self.sessions.len()).filter(|&ix| self.on_screen(ix)).collect()
    }

    /// The splits as the store keeps them.
    pub(super) fn split_records(&self) -> Vec<SplitRecord> {
        self.splits
            .iter()
            .map(|split| SplitRecord { id: split.id, layout: Some(split.layout.to_json()) })
            .collect()
    }

    /// The splits a window was saved with, once its sessions are all here: the sessions that name
    /// one are its members, laid out as saved and fitted to who is there (core
    /// `TabPanes::restored`). One member alone is no split; a name no record has still groups.
    pub(super) fn restore_splits(&mut self, records: &[SplitRecord]) {
        let mut ids: Vec<SplitId> = Vec::new();
        for session in &self.sessions {
            if let Some(id) = session.split
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
        for id in ids {
            let members = self.members(id);
            let panes: Vec<PaneId> = members.iter().map(|&ix| self.sessions[ix].id).collect();
            let saved =
                records.iter().find(|record| record.id == id).and_then(|record| record.layout.as_deref());
            let layout = TabPanes::restored(saved, &panes).filter(|layout| layout.is_split());
            match layout {
                Some(layout) => self.splits.push(Split { id, layout }),
                None => members.into_iter().for_each(|ix| self.sessions[ix].split = None),
            }
        }
    }

    /// Whether the split `ix` is in, or `ix` itself, is pinned: pinning one member pins them all.
    pub(super) fn split_pinned(&self, ix: usize) -> bool {
        match self.sessions.get(ix).and_then(|session| session.split) {
            Some(id) => self.members(id).into_iter().any(|member| self.sessions[member].pinned),
            None => self.sessions.get(ix).is_some_and(|session| session.pinned),
        }
    }

    // ---------------------------------------------------------------- the room

    /// The area the selected split's panes share, below the title rows, as of the last frame
    /// (`pane_probe`): zero before the first.
    pub(super) fn pane_area_size(&self) -> (f32, f32) {
        (f32::from(self.pane_area.size.width), f32::from(self.pane_area.size.height))
    }

    /// The least height a pane needs: the grid's minimum under its header.
    pub(super) fn pane_min_h(&self) -> f32 {
        // The taller header, so a pane fits whichever way the gaps setting goes.
        PANE_MIN_H + style::pane_header_h(true).max(style::PANE_HEADER_H)
    }

    /// What a drop or a join works with: the panes' area and the least a pane may be in it.
    pub(super) fn pane_room(&self) -> Room {
        let (width, height) = self.pane_area_size();
        Room { width, height, min_w: PANE_MIN_W, min_h: self.pane_min_h() }
    }

    /// Whether the focused pane has the room for a split along `axis`: both halves at least the
    /// minimum (core `PANE_MIN_W` × `PANE_MIN_H`), each under a header. Before the first frame
    /// nothing is known, and the split is allowed.
    pub(super) fn can_split(&self, axis: Axis) -> bool {
        let (w, h) = self.pane_area_size();
        if w <= 0.0 || h <= 0.0 {
            return true;
        }
        // A terminal on its own fills the area.
        let (pane_w, pane_h) = match self.active_split() {
            Some(split) => {
                let focused = split.layout.focused();
                let rect = split.layout.tree().rects(w, h).into_iter().find(|(id, _)| *id == focused);
                match rect {
                    Some((_, rect)) => (rect.w, rect.h),
                    None => return false,
                }
            }
            None => (w, h),
        };
        match axis {
            Axis::Row => pane_w / 2.0 >= PANE_MIN_W,
            Axis::Column => pane_h / 2.0 >= self.pane_min_h(),
        }
    }

    /// Whether the area has room for `preset`'s slots, each pane at least the minimum.
    pub(super) fn can_layout(&self, preset: Preset) -> bool {
        let (w, h) = self.pane_area_size();
        if w <= 0.0 || h <= 0.0 {
            return true;
        }
        let (columns, rows) = match preset {
            Preset::Single => (1.0, 1.0),
            Preset::TwoColumns => (2.0, 1.0),
            Preset::TwoRows => (1.0, 2.0),
            Preset::Grid => (2.0, 2.0),
        };
        w / columns >= PANE_MIN_W && h / rows >= self.pane_min_h()
    }

    // ---------------------------------------------------------------- joining and leaving

    /// Take session `ix` out of its split's layout, without touching what is selected: the panes
    /// beside it take its room. A split left with one member is gone and that terminal stands
    /// alone. `None`, nothing changed, when it is in no split.
    pub(super) fn leave_layout(&mut self, ix: usize) -> Option<Leave> {
        let id = self.sessions.get(ix)?.id;
        let split_id = self.sessions[ix].split?;
        let left = self.split_mut(split_id).and_then(|split| split.layout.leave(id));
        self.sessions[ix].split = None;
        if left != Some(Leave::Shrunk) {
            // One pane is left, which is no split (or the layout had lost track of it): whoever
            // is still named by it stands alone.
            self.splits.retain(|split| split.id != split_id);
            for member in self.members(split_id) {
                self.sessions[member].split = None;
            }
        }
        self.dirty = true;
        Some(left.unwrap_or(Leave::Dissolved))
    }

    /// Session `ix` leaves its split and stands alone (the pane menu's Move Pane Out of Split, a
    /// row dragged out of its bracket). The selection stays where it is. `false` when it is in no
    /// split.
    pub(crate) fn leave_split(&mut self, ix: usize, cx: &mut Context<Self>) -> bool {
        let left = self.leave_layout(ix).is_some();
        if left {
            cx.notify();
        }
        left
    }

    /// Session `ix` joins the split of the terminal with id `target`, on `zone`'s side of its
    /// pane, making the split when the target stands alone; it leaves its own split if it was in
    /// one. It takes the keyboard, and the row is selected. Refused, with nothing changed, when
    /// either pane would end up under the minimum size (`DropError::TooSmall`), or `ix` is in the
    /// split already or `target` isn't a terminal here (`Unknown`).
    pub(crate) fn join_split(
        &mut self,
        ix: usize,
        target: PaneId,
        zone: Zone,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), DropError> {
        let (Some(target_ix), Some(joiner)) =
            (self.find_session(target), self.sessions.get(ix).map(|s| s.id))
        else {
            return Err(DropError::Unknown);
        };
        let room = self.pane_room();
        let mut layout = match self.split_of(target_ix) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(target),
        };
        layout.join(target, zone, joiner, room)?;
        // It fits: leave the old split, then make or update the new one.
        self.leave_layout(ix);
        self.install_layout(target_ix, ix, layout, cx);
        let at = self.find_session(joiner).unwrap_or(ix);
        self.activate(at, window, cx);
        Ok(())
    }

    // ---------------------------------------------------------------- new panes

    /// ⌘D (`Row`: to the right) / ⇧⌘D (`Column`: below): a new terminal beside the selected one, in
    /// its folder and with its shell profile, joined into its split (made if it stood alone), with
    /// the keyboard. Nothing happens without the room.
    pub(crate) fn split_pane(&mut self, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_split(axis) {
            return;
        }
        let target = self.sessions[self.active].id;
        let (id, ix) = self.new_member(window, cx);
        let mut layout = match self.active_split() {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(target),
        };
        if !layout.split(axis, id) {
            self.drop_session(ix, cx);
            return;
        }
        self.install_layout(self.active, ix, layout, cx);
        let at = self.find_session(id).unwrap_or(ix);
        self.activate(at, window, cx);
    }

    /// A new terminal for a split to take: where the selected one is, running its shell, not yet
    /// selected or in any split. Its id and its place in `sessions`.
    fn new_member(&mut self, window: &mut Window, cx: &mut Context<Self>) -> (PaneId, usize) {
        let id = self.alloc_id(cx);
        let mut record = SessionRecord::new(id, None, self.default_cwd(cx));
        record.shell = self.active_session().pane.shell.clone();
        let ix = self.add_session(Some(record), window, cx);
        (id, ix)
    }

    /// Forget a session that was made for a split that did not happen.
    fn drop_session(&mut self, ix: usize, cx: &mut Context<Self>) {
        let session = self.sessions.remove(ix);
        notifications::dismiss(session.id, cx);
    }

    /// The id of session `anchor`'s split, made (with `layout`) when it stands alone.
    fn split_for(&mut self, anchor: usize, layout: &TabPanes, cx: &mut Context<Self>) -> SplitId {
        let current = self.sessions[anchor].split;
        if let Some(id) = current.filter(|id| self.splits.iter().any(|split| split.id == *id)) {
            return id;
        }
        let id = self.alloc_id(cx);
        self.splits.push(Split { id, layout: layout.clone() });
        id
    }

    /// Make `layout` session `anchor`'s split, with `joiner` in it: the split it had, or a new one.
    /// The joiner takes the anchor's pin (the pin is the split's) and its place beside the others.
    fn install_layout(&mut self, anchor: usize, joiner: usize, layout: TabPanes, cx: &mut Context<Self>) {
        let id = self.split_for(anchor, &layout, cx);
        if let Some(split) = self.split_mut(id) {
            split.layout = layout;
        }
        let pinned = self.sessions[anchor].pinned;
        self.sessions[anchor].split = Some(id);
        self.sessions[joiner].split = Some(id);
        self.sessions[joiner].pinned = pinned;
        // The joiner stands in the split's card: its group, right after the other members.
        let ids: Vec<i64> = self.sessions.iter().map(|s| s.id).collect();
        let joiner_id = self.sessions[joiner].id;
        let mates: Vec<i64> = self.members(id).into_iter().map(|m| self.sessions[m].id).collect();
        self.sidebar.view.seat_beside(&ids, joiner_id, &mates);
        self.dirty = true;
    }

    /// View → Layout: the selected terminal's split as `preset` in one step, made if it
    /// stood alone. New panes are new terminals in the focused pane's folder with its shell; panes
    /// over the layout's count leave the split and stand alone, still running. Nothing happens
    /// without the room.
    pub(crate) fn apply_layout(&mut self, preset: Preset, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_layout(preset) {
            return;
        }
        let anchor = self.active;
        let selected = self.sessions[anchor].id;
        let mut layout = match self.active_split() {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(selected),
        };
        let plan = layout.plan(preset);
        let new_ids: Vec<PaneId> = (0..plan.new).map(|_| self.alloc_id(cx)).collect();
        let Some(out) = layout.arrange(preset, &new_ids) else { return };
        let cwd = self.default_cwd(cx);
        let shell = self.sessions[anchor].pane.shell.clone();
        let mut joined = Vec::new();
        for &id in &new_ids {
            let mut record = SessionRecord::new(id, None, cwd.clone());
            record.shell = shell.clone();
            joined.push(self.add_session(Some(record), window, cx));
        }
        for pane in out {
            if let Some(ix) = self.find_session(pane) {
                self.sessions[ix].split = None;
            }
        }
        if layout.is_split() {
            let anchor_id = self.sessions[anchor].id;
            let split_id = self.split_for(anchor, &layout, cx);
            let pinned = self.sessions[anchor].pinned;
            if let Some(split) = self.split_mut(split_id) {
                split.layout = layout;
            }
            self.sessions[anchor].split = Some(split_id);
            for ix in joined {
                self.sessions[ix].split = Some(split_id);
                self.sessions[ix].pinned = pinned;
            }
            let focused = self.split_with(split_id).map_or(anchor_id, |split| split.layout.focused());
            self.dirty = true;
            let at = self.find_session(focused).unwrap_or(self.active);
            self.activate(at, window, cx);
        } else {
            // Down to one pane: no split.
            if let Some(split_id) = self.sessions[anchor].split.take() {
                self.splits.retain(|split| split.id != split_id);
                self.members(split_id).into_iter().for_each(|ix| self.sessions[ix].split = None);
            }
            self.dirty = true;
            let at = self.find_session(layout.focused()).unwrap_or(self.active);
            self.activate(at, window, cx);
        }
    }

    pub(super) fn split_with(&self, id: SplitId) -> Option<&Split> {
        self.splits.iter().find(|split| split.id == id)
    }
}
