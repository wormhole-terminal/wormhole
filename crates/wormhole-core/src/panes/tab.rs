//! A split's panes as the app keeps and saves them: the tree, the pane that has the keyboard (the
//! selected terminal) and the one zoomed to fill the area, if any. Every change of the tree
//! goes through here so the focus never names a pane that is gone.

use serde::{Deserialize, Deserializer, Serialize};

use super::drop::Zone;
use super::geometry::{Divider, DividerId, Rect};
use super::preset::{Plan, Preset, plan};
use super::{Axis, Direction, PANE_MIN_H, PANE_MIN_W, PaneId, Tree};

/// A pane may be this much short of the minimum and still fit: rounding, not room.
const FIT_EPS: f32 = 0.01;

/// What a drop works with: the area the panes share and the least a pane may be in it (the
/// arguments of [`TabPanes::drag`]). A width or height of 0 is an area that hasn't been laid out yet:
/// nothing is known, and a drop is allowed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    pub width: f32,
    pub height: f32,
    pub min_w: f32,
    pub min_h: f32,
}

impl Room {
    /// An area with core's minimum pane, [`PANE_MIN_W`] × [`PANE_MIN_H`]; the app adds a pane's
    /// header to `min_h` when it draws one.
    pub fn new(width: f32, height: f32) -> Room {
        Room { width, height, min_w: PANE_MIN_W, min_h: PANE_MIN_H }
    }
}

/// Why a drop doesn't change the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropError {
    /// A pane the drop names isn't where it has to be: the target isn't in the layout, or the
    /// terminal being joined already is (or the one being moved isn't).
    Unknown,
    /// The target or the dragged pane would end up under the minimum size.
    TooSmall,
    /// The panes would stay arranged as they are.
    Unchanged,
}

/// What a pane leaving did to its layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    /// Two or more panes are left: the layout stays a split.
    Shrunk,
    /// One pane is left, which is no split: the layout is gone and that terminal stands alone.
    Dissolved,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TabPanes {
    tree: Tree,
    focused: PaneId,
    zoomed: Option<PaneId>,
}

impl TabPanes {
    /// A layout with one pane.
    pub fn single(id: PaneId) -> TabPanes {
        TabPanes { tree: Tree::single(id), focused: id, zoomed: None }
    }

    /// A split loaded from the store: its saved layout (`json`, repaired) fitted to the panes the
    /// store has rows for (`ids`, in the order they were saved): a pane without a row is dropped,
    /// one without a place in the layout goes to the right of everything. No layout, or none worth
    /// keeping: the panes side by side in their order. `None` without any pane.
    pub fn restored(json: Option<&str>, ids: &[PaneId]) -> Option<TabPanes> {
        let (&first, _) = ids.split_first()?;
        let mut panes = json
            .and_then(TabPanes::from_json)
            .filter(|panes| panes.panes().iter().any(|id| ids.contains(id)))
            .unwrap_or_else(|| TabPanes::single(first));
        for id in panes.panes() {
            if !ids.contains(&id) {
                panes.tree.remove(id);
            }
        }
        for &id in ids {
            panes.tree.append(id);
        }
        panes.settle();
        Some(panes)
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The pane that has the keyboard: the selected terminal's.
    pub fn focused(&self) -> PaneId {
        self.focused
    }

    /// The pane that fills the area on its own (⇧⌘↩), while one does.
    pub fn zoomed(&self) -> Option<PaneId> {
        self.zoomed
    }

    /// The panes in reading order (see [`Tree::panes`]).
    pub fn panes(&self) -> Vec<PaneId> {
        self.tree.panes()
    }

    /// How many panes the layout has (never 0).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Whether there is more than one pane: headers and dividers show, ⌘W closes a pane.
    pub fn is_split(&self) -> bool {
        self.tree.len() > 1
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.tree.contains(id)
    }

    /// Give `id` the keyboard. Moving to another pane while one is zoomed shows them all again,
    /// so the person sees where they landed. `false` for a pane that isn't here.
    pub fn focus(&mut self, id: PaneId) -> bool {
        if !self.tree.contains(id) {
            return false;
        }
        self.focused = id;
        if self.zoomed.is_some_and(|zoomed| zoomed != id) {
            self.zoomed = None;
        }
        true
    }

    /// Split the focused pane along `axis` (⌘D right, ⇧⌘D down); the new pane `new_id` takes the
    /// keyboard, and a zoomed pane is shown among the others again. `false` when `new_id` is here
    /// already.
    pub fn split(&mut self, axis: Axis, new_id: PaneId) -> bool {
        if !self.tree.split(self.focused, axis, new_id) {
            return false;
        }
        self.focused = new_id;
        self.zoomed = None;
        true
    }

    /// Close pane `id` (its shell ended, or Close pane). The keyboard, if it had it, goes to the
    /// pane after it in reading order, or the one before when it was last. The last pane stays
    /// (`false`): the tab closes instead.
    pub fn close(&mut self, id: PaneId) -> bool {
        let order = self.tree.panes();
        let at = order.iter().position(|pane| *pane == id);
        if !self.tree.remove(id) {
            return false;
        }
        if self.focused == id
            && let Some(at) = at
        {
            self.focused = order.get(at + 1).or_else(|| order.get(at.wrapping_sub(1))).copied().unwrap_or(id);
        }
        if self.zoomed == Some(id) {
            self.zoomed = None;
        }
        self.settle();
        true
    }

    /// A terminal that isn't in this layout joins it on `zone`'s side of `target` (a sidebar row
    /// dragged onto a pane's edge). It takes half of the target's room and the keyboard, and a zoomed
    /// pane is shown among the others again, as for [`TabPanes::split`]. Refused, with nothing
    /// changed, when `target` isn't here, `id` is, or either would be left under `room`'s minimum.
    pub fn join(&mut self, target: PaneId, zone: Zone, id: PaneId, room: Room) -> Result<(), DropError> {
        let mut tree = self.tree.clone();
        if !tree.insert_beside(target, zone, id) {
            return Err(DropError::Unknown);
        }
        if !fits(&tree, [target, id], room) {
            return Err(DropError::TooSmall);
        }
        self.tree = tree;
        self.focused = id;
        self.zoomed = None;
        Ok(())
    }

    /// Pane `id` moves to `zone`'s side of `target` (a pane's header dragged onto another pane's
    /// edge). It takes the keyboard and a zoomed pane is shown among the others again. Refused,
    /// with nothing changed, when either isn't here, they are the same pane, the arrangement would
    /// stay ([`DropError::Unchanged`]), or one of the two would be left under `room`'s minimum.
    pub fn move_to(&mut self, id: PaneId, target: PaneId, zone: Zone, room: Room) -> Result<(), DropError> {
        if !self.tree.contains(id) || !self.tree.contains(target) {
            return Err(DropError::Unknown);
        }
        let mut tree = self.tree.clone();
        if !tree.move_pane(id, target, zone) {
            return Err(DropError::Unchanged);
        }
        if !fits(&tree, [target, id], room) {
            return Err(DropError::TooSmall);
        }
        self.tree = tree;
        self.focused = id;
        self.zoomed = None;
        Ok(())
    }

    /// Whether [`TabPanes::join`] would go through, for a drop target that shows it won't.
    pub fn can_join(&self, target: PaneId, zone: Zone, id: PaneId, room: Room) -> Result<(), DropError> {
        self.clone().join(target, zone, id, room)
    }

    /// Whether [`TabPanes::move_to`] would go through.
    pub fn can_move_to(&self, id: PaneId, target: PaneId, zone: Zone, room: Room) -> Result<(), DropError> {
        self.clone().move_to(id, target, zone, room)
    }

    /// Pane `id` leaves the layout (a pane's header dragged to the sidebar, or a row dragged out of
    /// its bracket): the panes beside it take its room, and the keyboard moves on as when a pane
    /// closes. `None`, nothing changed, when it isn't here or is the last one, which has nothing to
    /// leave. A layout left with one pane is [`Leave::Dissolved`].
    pub fn leave(&mut self, id: PaneId) -> Option<Leave> {
        if !self.close(id) {
            return None;
        }
        Some(if self.is_split() { Leave::Shrunk } else { Leave::Dissolved })
    }

    /// ⇧⌘↩: the focused pane fills the tab, or the panes come back. Nothing to zoom in a tab
    /// with one pane (`false`).
    pub fn toggle_zoom(&mut self) -> bool {
        if !self.is_split() {
            return false;
        }
        self.zoomed = if self.zoomed == Some(self.focused) { None } else { Some(self.focused) };
        true
    }

    /// Where the panes are in a tab `width` × `height` points: every pane in reading order, or
    /// the zoomed one alone over the whole tab.
    pub fn shown(&self, width: f32, height: f32) -> Vec<(PaneId, Rect)> {
        match self.zoomed {
            Some(id) => vec![(id, Rect::new(0.0, 0.0, width, height))],
            None => self.tree.rects(width, height),
        }
    }

    /// The dividers to draw (none while a pane is zoomed).
    pub fn dividers(&self, width: f32, height: f32) -> Vec<Divider> {
        match self.zoomed {
            Some(_) => Vec::new(),
            None => self.tree.dividers(width, height),
        }
    }

    /// See [`Tree::drag`].
    pub fn drag(
        &mut self,
        divider: &DividerId,
        position: f32,
        width: f32,
        height: f32,
        min_w: f32,
        min_h: f32,
    ) -> Option<f32> {
        self.tree.drag(divider, position, width, height, min_w, min_h)
    }

    /// See [`Tree::equalize`] (a double-click on a divider).
    pub fn equalize(&mut self, divider: &DividerId) -> bool {
        self.tree.equalize(divider)
    }

    /// The pane ⌥⌘arrow moves the keyboard to from the focused one (see [`Tree::neighbour`]).
    pub fn neighbour(&self, direction: Direction, width: f32, height: f32) -> Option<PaneId> {
        self.tree.neighbour(self.focused, direction, width, height)
    }

    /// What View → Layout `preset` would do to this tab (see [`plan`]).
    pub fn plan(&self, preset: Preset) -> Plan {
        plan(&self.tree, self.focused, preset)
    }

    /// Arrange the tab as `preset` (View → Layout), `new_ids` being the ids for the new
    /// panes its plan asks for (at least `plan.new`; the first of them are used). The focused pane
    /// keeps the keyboard when it stays, else the first slot has it; a zoomed pane is shown among
    /// the others. Returns the panes that left for tabs of their own, or `None` (nothing changed)
    /// with too few ids.
    pub fn arrange(&mut self, preset: Preset, new_ids: &[PaneId]) -> Option<Vec<PaneId>> {
        let plan = self.plan(preset);
        if new_ids.len() < plan.new {
            return None;
        }
        let ids: Vec<PaneId> = plan.keep.iter().chain(&new_ids[..plan.new]).copied().collect();
        if !self.tree.arrange(preset, &ids) {
            return None;
        }
        self.zoomed = None;
        self.settle();
        Some(plan.out)
    }

    /// The preset the tab has the shape of, for the menu's check mark.
    pub fn preset(&self) -> Option<Preset> {
        self.tree.preset()
    }

    /// The tab's layout for the store: the tree ([`Tree::to_json`]), `"focused"` and `"zoomed"`.
    pub fn to_json(&self) -> String {
        // Only non-string map keys or a failing `Serialize` make `to_string` fail; neither is here.
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A saved layout, repaired: the tree as [`Tree::from_json`] does, a focused pane that is gone
    /// replaced by the first, a zoom on a pane that is gone (or a tab that isn't split) dropped.
    pub fn from_json(json: &str) -> Option<TabPanes> {
        serde_json::from_str(json).ok()
    }

    /// The focus and the zoom name panes that are here, and nothing is zoomed in an unsplit tab.
    fn settle(&mut self) {
        if !self.tree.contains(self.focused) {
            self.focused = self.tree.panes()[0];
        }
        if self.zoomed.is_some_and(|id| !self.tree.contains(id)) || !self.is_split() {
            self.zoomed = None;
        }
    }
}

/// Whether both panes have their minimum in `room`: after a drop only the target and the pane that
/// came beside it can have shrunk, so a pane that was small already doesn't refuse a drop elsewhere.
fn fits(tree: &Tree, panes: [PaneId; 2], room: Room) -> bool {
    if room.width <= 0.0 || room.height <= 0.0 {
        return true;
    }
    tree.rects(room.width, room.height)
        .iter()
        .filter(|(id, _)| panes.contains(id))
        .all(|(_, rect)| rect.w + FIT_EPS >= room.min_w && rect.h + FIT_EPS >= room.min_h)
}

impl<'de> Deserialize<'de> for TabPanes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            tree: Tree,
            #[serde(default)]
            focused: Option<PaneId>,
            #[serde(default)]
            zoomed: Option<PaneId>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let first = raw.tree.panes()[0];
        let mut panes =
            TabPanes { tree: raw.tree, focused: raw.focused.unwrap_or(first), zoomed: raw.zoomed };
        panes.settle();
        Ok(panes)
    }
}
