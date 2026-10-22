//! A split's panes, as data: every terminal is a row of the
//! sidebar, and a split shows several of them side by side. The tree of panes, what the menu and
//! the keys do to it (split right ⌘D, split down ⇧⌘D, zoom ⇧⌘↩, drag a divider, double-click it
//! to equalize, ⌥⌘arrows between panes), its geometry for an area of a given size, the grid
//! presets of View → Layout and a JSON form for the store. The app draws the panes and owns their
//! PTYs; nothing here needs the UI.
//!
//! A split holds a [`Tree`]: one pane, or a split along an [`Axis`] whose children each take a
//! share (`ratio`) of its extent and are panes or splits of the other axis. Splitting a pane along
//! its parent's axis adds a sibling, so the tree never nests splits of one axis and three panes
//! side by side are one node. Closing a pane hands its share to its siblings in proportion, and a
//! split left with one child gives way to it. Siblings' ratios always sum to 1. Dividers have no
//! thickness in this model: the app draws them over the panes' shared edges. A pane's id is the id
//! of its terminal's session, allocated by the app (it is what the agent hooks name,
//! `WORMHOLE_TAB_ID`), so every operation that adds a pane takes the new id. [`TabPanes`] is the
//! tree with what the split remembers around it: the focused pane and the zoomed one. A pane can
//! also be put beside another by a drop on one of its edges (`drop` finds which,
//! [`Tree::insert_beside`] and [`Tree::move_pane`] do it, [`TabPanes`] adds the minimum sizes and
//! the keyboard), and taken out again ([`TabPanes::leave`]), when its terminal is dragged to or
//! from the sidebar.

mod drop;
mod geometry;
mod preset;
#[cfg(test)]
mod rearrange_tests;
mod repair;
mod tab;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Deserializer, Serialize};

pub use drop::{
    EDGE_BAND, EDGE_BAND_MAX, Zone, band_rect, drop_target, edge_band, hint_rect, preview_rect, zone_at,
};
pub use geometry::{Divider, DividerId, Rect};
pub use preset::{Plan, Preset, plan};
pub use tab::{DropError, Leave, Room, TabPanes};

/// A pane's id, allocated by the app like a tab's.
pub type PaneId = i64;

/// The least a pane may be, in points: a divider's drag stops here, and the app refuses a split
/// that would leave either half under it. 160 pt is about 18 columns at the default font, a prompt
/// with a short command and the narrowest the agents' TUIs redraw without wrapping every line;
/// 96 pt is about five rows, a prompt, a line or two of output and a TUI's status line. Smaller
/// panes would also resize their PTYs to grids programs can't use.
pub const PANE_MIN_W: f32 = 160.0;
pub const PANE_MIN_H: f32 = 96.0;

/// How a split lays its children out: `Row` side by side (what ⌘D, split right, adds to),
/// `Column` stacked (⇧⌘D, split down).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    Row,
    Column,
}

/// Where ⌥⌘← → ↑ ↓ moves the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// One node of the tree: a pane, or a split whose children (never fewer than two in a [`Tree`])
/// share its extent along `axis`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Node {
    Leaf(PaneId),
    Split { axis: Axis, children: Vec<Child> },
}

/// A split's child with its share of the split's extent (siblings' ratios sum to 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Child {
    /// A record without one loads with equal shares (see `repair`).
    #[serde(default)]
    pub ratio: f32,
    pub node: Node,
}

impl Child {
    fn new(ratio: f32, node: Node) -> Child {
        Child { ratio, node }
    }
}

/// A split's panes. Always holds at least one pane; every pane id appears once; no split has fewer
/// than two children or a child split of its own axis; siblings' ratios are positive and sum to 1.
/// Deserializing repairs whatever breaks these, so a saved record never panics the app.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Tree {
    root: Node,
}

impl Tree {
    /// A tree with one pane.
    pub fn single(id: PaneId) -> Tree {
        Tree { root: Node::Leaf(id) }
    }

    /// A tree from a hand-built or loaded root, with the invariants restored (`repair`): later
    /// duplicates of an id and empty splits are dropped, lone children replace their split,
    /// same-axis nesting is merged, bad ratios become equal shares. `None` when no pane is left.
    pub fn from_root(root: Node) -> Option<Tree> {
        repair::repaired(root).map(|root| Tree { root })
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Split `target` along `axis`, the new pane `new_id` to its right (`Row`) or below it
    /// (`Column`), each taking half of the target's room. Inside a split of the same axis the new
    /// pane becomes a sibling rather than nesting. `false` when `target` isn't a pane here or
    /// `new_id` already is.
    pub fn split(&mut self, target: PaneId, axis: Axis, new_id: PaneId) -> bool {
        if self.contains(new_id) {
            return false;
        }
        split_in(&mut self.root, target, axis, false, new_id)
    }

    /// Put the new pane `id` on `zone`'s side of `target`: the same as [`Tree::split`] along the
    /// zone's axis, except that the left and top sides put it before the target. `false` when
    /// `target` isn't a pane here or `id` already is.
    pub fn insert_beside(&mut self, target: PaneId, zone: Zone, id: PaneId) -> bool {
        if self.contains(id) {
            return false;
        }
        split_in(&mut self.root, target, zone.axis(), zone.before(), id)
    }

    /// Take `id` out of its place and put it on `zone`'s side of `target` (a pane's header dragged
    /// onto another pane's edge). The room it leaves goes to its siblings as when a pane closes,
    /// and the room it takes comes from half of the target's. `false`, with the tree as it was,
    /// when either isn't a pane here, they are the same pane, or the panes would end up arranged
    /// as they are (only their shares differ).
    pub fn move_pane(&mut self, id: PaneId, target: PaneId, zone: Zone) -> bool {
        if id == target || !self.contains(id) || !self.contains(target) {
            return false;
        }
        let mut moved = self.clone();
        if !moved.remove(id) || !moved.insert_beside(target, zone, id) || same_shape(&self.root, &moved.root)
        {
            return false;
        }
        *self = moved;
        true
    }

    /// Close a pane: its share goes to its siblings in proportion to theirs, a split left with
    /// one child gives way to it, and a split that ends up inside one of its own axis merges into
    /// it. The last pane stays (`false`, as for an id that isn't here).
    pub fn remove(&mut self, id: PaneId) -> bool {
        remove_in(&mut self.root, id)
    }

    /// A pane that has no place yet (a record without a layout) goes at the end: to the right of
    /// everything, as a sibling of a row at the root or in a new row round it, taking an equal
    /// share. `false` when it is here already.
    pub fn append(&mut self, id: PaneId) -> bool {
        if self.contains(id) {
            return false;
        }
        match &mut self.root {
            Node::Split { axis: Axis::Row, children } => {
                children.push(Child::new(0.0, Node::Leaf(id)));
                repair::equal_shares(children);
            }
            node => {
                let first = Child::new(0.5, std::mem::replace(node, Node::Leaf(id)));
                *node =
                    Node::Split { axis: Axis::Row, children: vec![first, Child::new(0.5, Node::Leaf(id))] };
            }
        }
        true
    }

    /// The panes in reading order: left to right, then top to bottom, of the tree with equal
    /// shares, so dragging a divider never reorders them. In a 2×2 grid: top left, top right,
    /// bottom left, bottom right.
    pub fn panes(&self) -> Vec<PaneId> {
        geometry::reading_order(&self.root)
    }

    /// How many panes the tree has (never 0).
    // A tree is never empty, so `is_empty` would only ever say `false`.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        count(&self.root)
    }

    pub fn contains(&self, id: PaneId) -> bool {
        contains(&self.root, id)
    }

    /// Equal shares in every split.
    pub fn equalize_all(&mut self) {
        equalize_in(&mut self.root);
    }

    /// The tree for the store. The shape is part of the saved record's format: a node is
    /// `{"leaf": <id>}` or `{"split": {"axis": "row" | "column", "children": [{"ratio", "node"}]}}`.
    pub fn to_json(&self) -> String {
        // Only non-string map keys or a failing `Serialize` make `to_string` fail; neither is here.
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A saved tree, repaired (see [`Tree::from_root`]); `None` when it isn't a tree or has no
    /// pane left, and the caller starts from what it knows.
    pub fn from_json(json: &str) -> Option<Tree> {
        serde_json::from_str(json).ok()
    }
}

impl<'de> Deserialize<'de> for Tree {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            root: Node,
        }
        let raw = Raw::deserialize(deserializer)?;
        Tree::from_root(raw.root).ok_or_else(|| serde::de::Error::custom("a pane tree needs a pane"))
    }
}

/// Splits the pane `target` along `axis`, the new pane after it, or before it when `before`.
fn split_in(node: &mut Node, target: PaneId, axis: Axis, before: bool, new_id: PaneId) -> bool {
    match node {
        Node::Leaf(id) if *id == target => {
            let (first, second) = if before { (new_id, target) } else { (target, new_id) };
            let children = vec![Child::new(0.5, Node::Leaf(first)), Child::new(0.5, Node::Leaf(second))];
            *node = Node::Split { axis, children };
            true
        }
        Node::Leaf(_) => false,
        Node::Split { axis: own, children } => {
            // Along the parent's own axis the new pane is a sibling, not a nested split, and
            // takes half of the target's share so the other siblings keep theirs.
            if *own == axis
                && let Some(i) = children.iter().position(|child| child.node == Node::Leaf(target))
            {
                let half = children[i].ratio / 2.0;
                children[i].ratio = half;
                children.insert(if before { i } else { i + 1 }, Child::new(half, Node::Leaf(new_id)));
                return true;
            }
            children.iter_mut().any(|child| split_in(&mut child.node, target, axis, before, new_id))
        }
    }
}

/// Removes the pane from this split or one below it, then tidies the split (ratios renormalised, a
/// lone child lifted, a same-axis child merged), so each level sees canonical children.
fn remove_in(node: &mut Node, id: PaneId) -> bool {
    let Node::Split { children, .. } = node else { return false };
    let removed = match children.iter().position(|child| child.node == Node::Leaf(id)) {
        Some(i) => {
            children.remove(i);
            true
        }
        None => children.iter_mut().any(|child| remove_in(&mut child.node, id)),
    };
    if removed {
        repair::tidy(node);
    }
    removed
}

/// Whether two trees arrange the same panes the same way, shares aside.
fn same_shape(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Leaf(a), Node::Leaf(b)) => a == b,
        (Node::Split { axis: a, children: ac }, Node::Split { axis: b, children: bc }) => {
            a == b && ac.len() == bc.len() && ac.iter().zip(bc).all(|(a, b)| same_shape(&a.node, &b.node))
        }
        _ => false,
    }
}

fn equalize_in(node: &mut Node) {
    if let Node::Split { children, .. } = node {
        repair::equal_shares(children);
        for child in children {
            equalize_in(&mut child.node);
        }
    }
}

fn count(node: &Node) -> usize {
    match node {
        Node::Leaf(_) => 1,
        Node::Split { children, .. } => children.iter().map(|child| count(&child.node)).sum(),
    }
}

fn contains(node: &Node, id: PaneId) -> bool {
    match node {
        Node::Leaf(own) => *own == id,
        Node::Split { children, .. } => children.iter().any(|child| contains(&child.node, id)),
    }
}
