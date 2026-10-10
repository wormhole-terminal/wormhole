//! View → Layout: the current tab arranged in one step as 2 columns, 2 rows, a
//! 2×2 grid or a single pane, where splitting by hand would take several. A preset has a fixed
//! number of slots: the tab's panes fill them in reading order (the focused one always kept), new
//! panes fill what is left, and panes over the count leave for tabs of their own rather than being
//! closed, since they are running terminals.

use super::{Axis, Child, Node, PaneId, Tree};

/// A layout the View → Layout menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Preset {
    Single,
    TwoColumns,
    TwoRows,
    /// Two columns of two: the vertical divider spans the tab, each column's rows move on their
    /// own.
    Grid,
}

impl Preset {
    /// In the menu's order.
    pub const ALL: [Preset; 4] = [Preset::TwoColumns, Preset::TwoRows, Preset::Grid, Preset::Single];

    /// How many panes the layout holds.
    pub fn slots(self) -> usize {
        match self {
            Preset::Single => 1,
            Preset::TwoColumns | Preset::TwoRows => 2,
            Preset::Grid => 4,
        }
    }

    /// The menu item's title.
    pub fn title(self) -> &'static str {
        match self {
            Preset::Single => "Single Pane",
            Preset::TwoColumns => "2 Columns",
            Preset::TwoRows => "2 Rows",
            Preset::Grid => "2×2 Grid",
        }
    }

    /// The tree of `ids` (exactly [`Preset::slots`] of them, in reading order) in this layout,
    /// with equal shares.
    fn shape(self, ids: &[PaneId]) -> Node {
        let leaf = |ix: usize| Node::Leaf(ids[ix]);
        let split = |axis: Axis, nodes: Vec<Node>| {
            let share = 1.0 / nodes.len() as f32;
            Node::Split { axis, children: nodes.into_iter().map(|node| Child::new(share, node)).collect() }
        };
        match self {
            Preset::Single => leaf(0),
            Preset::TwoColumns => split(Axis::Row, vec![leaf(0), leaf(1)]),
            Preset::TwoRows => split(Axis::Column, vec![leaf(0), leaf(1)]),
            // Reading order is top left, top right, bottom left, bottom right.
            Preset::Grid => split(
                Axis::Row,
                vec![
                    split(Axis::Column, vec![leaf(0), leaf(2)]),
                    split(Axis::Column, vec![leaf(1), leaf(3)]),
                ],
            ),
        }
    }
}

/// What applying a preset to a tab takes: which panes fill its slots (in reading order), how many
/// new panes the app has to open for the slots left, and which panes leave for tabs of their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub keep: Vec<PaneId>,
    pub new: usize,
    pub out: Vec<PaneId>,
}

/// The plan for `preset` on `tree` whose focused pane is `focused`: the first panes in reading
/// order fill the slots, the focused one among them whatever its place (it is the one the person is
/// in), and the rest leave.
pub fn plan(tree: &Tree, focused: PaneId, preset: Preset) -> Plan {
    let order = tree.panes();
    let slots = preset.slots();
    let mut keep: Vec<PaneId> = order.iter().copied().take(slots).collect();
    let mut out: Vec<PaneId> = order.iter().copied().skip(slots).collect();
    if let Some(at) = out.iter().position(|id| *id == focused)
        && let Some(last) = keep.last_mut()
    {
        out[at] = *last;
        *last = focused;
        // The panes that leave keep their reading order.
        out.sort_by_key(|id| order.iter().position(|pane| pane == id));
    }
    let new = slots - keep.len();
    Plan { keep, new, out }
}

impl Tree {
    /// This tree as `preset`, holding `ids` (exactly its slots, in reading order) with equal
    /// shares. `false`, and nothing changes, with another number of ids.
    pub(super) fn arrange(&mut self, preset: Preset, ids: &[PaneId]) -> bool {
        if ids.len() != preset.slots() {
            return false;
        }
        self.root = preset.shape(ids);
        true
    }

    /// The preset this tree has the shape of, whatever its shares, for the menu's check mark.
    pub fn preset(&self) -> Option<Preset> {
        let leaves = |children: &[Child]| children.iter().all(|child| matches!(child.node, Node::Leaf(_)));
        let pair_of_pairs = |children: &[Child], inner: Axis| {
            children.len() == 2
                && children.iter().all(|child| {
                    matches!(&child.node, Node::Split { axis, children } if *axis == inner && children.len() == 2 && leaves(children))
                })
        };
        match &self.root {
            Node::Leaf(_) => Some(Preset::Single),
            Node::Split { axis: Axis::Row, children } if children.len() == 2 && leaves(children) => {
                Some(Preset::TwoColumns)
            }
            Node::Split { axis: Axis::Column, children } if children.len() == 2 && leaves(children) => {
                Some(Preset::TwoRows)
            }
            Node::Split { axis: Axis::Row, children } if pair_of_pairs(children, Axis::Column) => {
                Some(Preset::Grid)
            }
            Node::Split { axis: Axis::Column, children } if pair_of_pairs(children, Axis::Row) => {
                Some(Preset::Grid)
            }
            Node::Split { .. } => None,
        }
    }
}
