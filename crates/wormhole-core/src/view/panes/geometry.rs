//! Where the panes and the dividers of a tab are, for a tab of a given size, and what a drag on a
//! divider or ⌥⌘arrow does with that. Positions are in points from the tab's top left.

use super::{Axis, Child, Direction, Node, PaneId, Tree};

/// Edges closer than this are one edge. Adjacent children share one computed edge, so within a
/// tree edges match exactly; the slack covers ratios rounded on their way through a record.
const EDGE_EPS: f32 = 0.5;

/// The least share of a split a pane keeps when the tab's size is not known yet.
const MIN_SHARE: f32 = 0.01;

/// A pane's or divider's place in the tab.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// The start and the extent along `axis`: `x` and `w` across a row, `y` and `h` down a column.
    fn along(&self, axis: Axis) -> (f32, f32) {
        match axis {
            Axis::Row => (self.x, self.w),
            Axis::Column => (self.y, self.h),
        }
    }
}

/// Which divider: the split it belongs to, as the child indexes from the root down to it, and the
/// index of the child before it (divider `i` separates children `i` and `i + 1`). Stable while
/// the tree's shape is, which a drag never changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DividerId {
    pub path: Vec<usize>,
    pub index: usize,
}

/// A divider between two children of a split: a line with no thickness along their shared edge
/// (the app gives it a hit area). A `Row` split's divider is a vertical line whose drag moves in
/// x; a `Column` split's is horizontal and moves in y.
#[derive(Clone, Debug, PartialEq)]
pub struct Divider {
    pub id: DividerId,
    pub axis: Axis,
    pub rect: Rect,
}

impl Divider {
    /// Where the line is along its split's axis: its x in a row, its y in a column.
    pub fn position(&self) -> f32 {
        self.rect.along(self.axis).0
    }
}

impl Tree {
    /// Every pane's rectangle in a tab `width` × `height` points, in reading order
    /// ([`Tree::panes`]).
    pub fn rects(&self, width: f32, height: f32) -> Vec<(PaneId, Rect)> {
        let order = self.panes();
        let mut rects = Vec::with_capacity(order.len());
        layout(&self.root, Rect::new(0.0, 0.0, width, height), false, &mut rects);
        rects.sort_by_key(|(id, _)| order.iter().position(|pane| pane == id));
        rects
    }

    /// Every divider in a tab `width` × `height` points, splits before their children.
    pub fn dividers(&self, width: f32, height: f32) -> Vec<Divider> {
        let mut dividers = Vec::new();
        let mut path = Vec::new();
        collect_dividers(&self.root, Rect::new(0.0, 0.0, width, height), &mut path, &mut dividers);
        dividers
    }

    /// Drag a divider to `position` (x for a row's, y for a column's) in a tab `width` × `height`.
    /// Only the two children it separates change size; the position is held where either of them,
    /// or any pane inside them, would drop under `min_w` × `min_h`. Returns where the divider
    /// landed, or `None` when it isn't there or can't move at all (the tab is already too small
    /// for both sides' minimums, so the ratios stay as they were).
    pub fn drag(
        &mut self,
        divider: &DividerId,
        position: f32,
        width: f32,
        height: f32,
        min_w: f32,
        min_h: f32,
    ) -> Option<f32> {
        let (node, rect) = split_at(&mut self.root, &divider.path, Rect::new(0.0, 0.0, width, height))?;
        let Node::Split { axis, children } = node else { return None };
        let axis = *axis;
        let k = divider.index;
        if k + 1 >= children.len() {
            return None;
        }
        let (start, extent) = rect.along(axis);
        if extent <= 0.0 {
            return None;
        }
        let before: f32 = children[..k].iter().map(|child| child.ratio).sum();
        let pair = children[k].ratio + children[k + 1].ratio;
        let first_start = start + extent * before;
        let lo = first_start + min_extent(&children[k].node, axis, min_w, min_h);
        let hi = first_start + extent * pair - min_extent(&children[k + 1].node, axis, min_w, min_h);
        if lo > hi {
            return None;
        }
        let position = position.clamp(lo, hi);
        children[k].ratio = (position - first_start) / extent;
        children[k + 1].ratio = pair - children[k].ratio;
        Some(position)
    }

    /// Give pane `pane` the share `share` (0 to 1) of the split it stands in, taking from or giving to its
    /// neighbour `beside`, which must be the child next to it in that split; the pair keeps what it had
    /// between them, so no other pane moves. The share is held where either would drop under `min_w` ×
    /// `min_h` (as a drag holds a divider), and when the tab's size is not known (a `width` or `height` of 0)
    /// it only keeps both from vanishing. Returns the share it ended with, or `None`, with nothing changed,
    /// when the panes are not neighbours or the tab is too small for both minimums.
    pub fn set_share(
        &mut self,
        pane: PaneId,
        beside: PaneId,
        share: f32,
        width: f32,
        height: f32,
        (min_w, min_h): (f32, f32),
    ) -> Option<f32> {
        let mut path = Vec::new();
        let at = parent_of(&self.root, pane, &mut path)?;
        let (node, rect) = split_at(&mut self.root, &path, Rect::new(0.0, 0.0, width, height))?;
        let Node::Split { axis, children } = node else { return None };
        let other = children.iter().position(|child| child.node == Node::Leaf(beside))?;
        if at.abs_diff(other) != 1 {
            return None;
        }
        let (_, extent) = rect.along(*axis);
        let pair = children[at].ratio + children[other].ratio;
        let (lo, hi) = if extent > 0.0 {
            (
                min_extent(&children[at].node, *axis, min_w, min_h) / extent,
                pair - min_extent(&children[other].node, *axis, min_w, min_h) / extent,
            )
        } else {
            (MIN_SHARE, pair - MIN_SHARE)
        };
        if lo > hi {
            return None;
        }
        let share = share.clamp(lo, hi);
        children[at].ratio = share;
        children[other].ratio = pair - share;
        Some(share)
    }

    /// Equal shares for the children of the split this divider belongs to (a double-click on it).
    pub fn equalize(&mut self, divider: &DividerId) -> bool {
        let Some((node, _)) = split_at(&mut self.root, &divider.path, Rect::default()) else { return false };
        let Node::Split { children, .. } = node else { return false };
        if divider.index + 1 >= children.len() {
            return false;
        }
        super::repair::equal_shares(children);
        true
    }

    /// The pane the focus moves to from `id` in `direction` (⌥⌘arrows), by geometry in a tab
    /// `width` × `height`: among the panes across the shared edge, the one overlapping it most; at
    /// a tie, the one whose centre is nearest, and at a dead tie the top or left one. `None` at
    /// the tab's edge, so the app decides whether to wrap.
    pub fn neighbour(&self, id: PaneId, direction: Direction, width: f32, height: f32) -> Option<PaneId> {
        let rects = self.rects(width, height);
        let (_, from) = rects.iter().find(|(pane, _)| *pane == id)?;
        let (from_cx, from_cy) = from.center();
        let mut best: Option<(PaneId, f32, f32)> = None;
        for (pane, rect) in &rects {
            if *pane == id {
                continue;
            }
            let (touching, overlap, distance) = match direction {
                Direction::Right => (
                    near(rect.x, from.right()),
                    overlap(rect.y, rect.bottom(), from.y, from.bottom()),
                    (rect.center().1 - from_cy).abs(),
                ),
                Direction::Left => (
                    near(rect.right(), from.x),
                    overlap(rect.y, rect.bottom(), from.y, from.bottom()),
                    (rect.center().1 - from_cy).abs(),
                ),
                Direction::Down => (
                    near(rect.y, from.bottom()),
                    overlap(rect.x, rect.right(), from.x, from.right()),
                    (rect.center().0 - from_cx).abs(),
                ),
                Direction::Up => (
                    near(rect.bottom(), from.y),
                    overlap(rect.x, rect.right(), from.x, from.right()),
                    (rect.center().0 - from_cx).abs(),
                ),
            };
            // Touching at a corner only is not sharing an edge.
            if !touching || overlap <= EDGE_EPS {
                continue;
            }
            let better = match best {
                None => true,
                Some((_, best_overlap, best_distance)) => {
                    if (overlap - best_overlap).abs() > EDGE_EPS {
                        overlap > best_overlap
                    } else {
                        distance < best_distance - EDGE_EPS
                    }
                }
            };
            if better {
                best = Some((*pane, overlap, distance));
            }
        }
        best.map(|(pane, _, _)| pane)
    }
}

/// The panes in reading order: the tree laid out with equal shares on a unit square, sorted by
/// top then left. Equal shares, so the order depends on the shape alone and never on a drag.
pub(super) fn reading_order(root: &Node) -> Vec<PaneId> {
    let mut rects = Vec::new();
    layout(root, Rect::new(0.0, 0.0, 1.0, 1.0), true, &mut rects);
    rects.sort_by(|(_, a), (_, b)| {
        // Shares of 1/n reached along different paths can differ by an ulp; that is one row.
        if (a.y - b.y).abs() > 1e-4 { a.y.total_cmp(&b.y) } else { a.x.total_cmp(&b.x) }
    });
    rects.into_iter().map(|(id, _)| id).collect()
}

/// The children's rectangles inside `rect` along `axis`. Each child's end is the next one's start,
/// computed once, so neighbours' edges compare equal. `equal` lays them out as if every share were
/// `1 / n`.
fn child_rects(rect: Rect, axis: Axis, children: &[Child], equal: bool) -> Vec<Rect> {
    let (start, extent) = rect.along(axis);
    let n = children.len();
    let mut edges = Vec::with_capacity(n + 1);
    let mut cumulative = 0.0;
    edges.push(start);
    for (i, child) in children.iter().enumerate() {
        cumulative += if equal { 1.0 / n as f32 } else { child.ratio };
        // The last edge is the split's own, whatever rounding left of the sum.
        edges.push(if i + 1 == n { start + extent } else { start + extent * cumulative });
    }
    edges
        .windows(2)
        .map(|pair| match axis {
            Axis::Row => Rect::new(pair[0], rect.y, pair[1] - pair[0], rect.h),
            Axis::Column => Rect::new(rect.x, pair[0], rect.w, pair[1] - pair[0]),
        })
        .collect()
}

fn layout(node: &Node, rect: Rect, equal: bool, out: &mut Vec<(PaneId, Rect)>) {
    match node {
        Node::Leaf(id) => out.push((*id, rect)),
        Node::Split { axis, children } => {
            for (child, rect) in children.iter().zip(child_rects(rect, *axis, children, equal)) {
                layout(&child.node, rect, equal, out);
            }
        }
    }
}

fn collect_dividers(node: &Node, rect: Rect, path: &mut Vec<usize>, out: &mut Vec<Divider>) {
    let Node::Split { axis, children } = node else { return };
    let rects = child_rects(rect, *axis, children, false);
    for (index, next) in rects.iter().enumerate().skip(1) {
        let line = match axis {
            Axis::Row => Rect::new(next.x, rect.y, 0.0, rect.h),
            Axis::Column => Rect::new(rect.x, next.y, rect.w, 0.0),
        };
        out.push(Divider { id: DividerId { path: path.clone(), index: index - 1 }, axis: *axis, rect: line });
    }
    for (i, (child, rect)) in children.iter().zip(rects).enumerate() {
        path.push(i);
        collect_dividers(&child.node, rect, path, out);
        path.pop();
    }
}

/// The split at `path` with its rectangle, `rect` being the root's.
fn split_at<'a>(node: &'a mut Node, path: &[usize], rect: Rect) -> Option<(&'a mut Node, Rect)> {
    let Some((&i, rest)) = path.split_first() else {
        return matches!(node, Node::Split { .. }).then_some((node, rect));
    };
    let Node::Split { axis, children } = node else { return None };
    let child_rect = child_rects(rect, *axis, children, false).get(i).copied()?;
    split_at(&mut children.get_mut(i)?.node, rest, child_rect)
}

/// Where pane `id` stands directly in a split: the index among that split's children, with `path` left as the
/// child indexes from the root down to the split.
fn parent_of(node: &Node, id: PaneId, path: &mut Vec<usize>) -> Option<usize> {
    let Node::Split { children, .. } = node else { return None };
    if let Some(at) = children.iter().position(|child| child.node == Node::Leaf(id)) {
        return Some(at);
    }
    for (i, child) in children.iter().enumerate() {
        path.push(i);
        if let Some(found) = parent_of(&child.node, id, path) {
            return Some(found);
        }
        path.pop();
    }
    None
}

/// The least room a subtree needs along `axis` for every pane in it to keep its minimum: panes
/// in a split of that axis add up, panes across it need the widest one's.
fn min_extent(node: &Node, axis: Axis, min_w: f32, min_h: f32) -> f32 {
    match node {
        Node::Leaf(_) => match axis {
            Axis::Row => min_w,
            Axis::Column => min_h,
        },
        Node::Split { axis: own, children } => {
            let extents = children.iter().map(|child| min_extent(&child.node, axis, min_w, min_h));
            if *own == axis { extents.sum() } else { extents.fold(0.0, f32::max) }
        }
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= EDGE_EPS
}

/// How much of `a0..a1` and `b0..b1` coincide (0 when they don't).
fn overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}
