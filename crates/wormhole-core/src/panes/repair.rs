//! Keeping a tree's invariants: after a pane is closed, and when a saved record is loaded. A record
//! comes from the store, but an older build, a hand edit or a damaged row may hand us anything:
//! duplicate ids, a split with no children, ratios that are negative, zero, infinite or missing.
//! Repairing beats refusing, because the tab's panes are still running and the user would lose
//! their arrangement over a number.

use std::collections::BTreeSet;

use super::{Child, Node, PaneId};

/// The root with every invariant restored, or `None` when no pane is left in it.
pub(super) fn repaired(root: Node) -> Option<Node> {
    let mut seen = BTreeSet::new();
    let mut root = repair(root, &mut seen)?;
    tidy(&mut root);
    Some(root)
}

/// Bottom-up: drops later duplicates of an id and splits left without children, repairs each
/// split's ratios and tidies it so the parent sees canonical children.
fn repair(node: Node, seen: &mut BTreeSet<PaneId>) -> Option<Node> {
    match node {
        Node::Leaf(id) => seen.insert(id).then_some(Node::Leaf(id)),
        Node::Split { axis, children } => {
            let children: Vec<Child> = children
                .into_iter()
                .filter_map(|child| repair(child.node, seen).map(|node| Child { ratio: child.ratio, node }))
                .collect();
            if children.is_empty() {
                return None;
            }
            let mut node = Node::Split { axis, children };
            tidy(&mut node);
            Some(node)
        }
    }
}

/// A split whose children just changed, made canonical again: a child split left with one child
/// is replaced by it, a child split of the same axis is merged in (its children scaled into the
/// child's share), the ratios sum to 1 again, and the split itself gives way to a lone child.
/// Children below are assumed canonical already (callers work bottom-up).
pub(super) fn tidy(node: &mut Node) {
    let Node::Split { axis, children } = node else { return };
    for child in children.iter_mut() {
        lift_lone_child(&mut child.node);
    }
    let mut merged = Vec::with_capacity(children.len());
    for Child { ratio, node } in children.drain(..) {
        match node {
            Node::Split { axis: inner, children: grandchildren } if inner == *axis => {
                merged.extend(grandchildren.into_iter().map(|mut child| {
                    child.ratio *= ratio;
                    child
                }));
            }
            node => merged.push(Child { ratio, node }),
        }
    }
    *children = merged;
    normalise(children);
    lift_lone_child(node);
}

fn lift_lone_child(node: &mut Node) {
    if let Node::Split { children, .. } = node
        && children.len() == 1
        && let Some(only) = children.pop()
    {
        *node = only.node;
    }
}

/// Make the ratios sum to 1, keeping their proportions; a share that isn't a positive finite
/// number spoils them all (a zero-size pane is as useless as a NaN one), and the split falls back
/// to equal shares.
pub(super) fn normalise(children: &mut [Child]) {
    let sum: f32 = children.iter().map(|child| child.ratio).sum();
    if children.iter().all(|child| is_share(child.ratio)) && is_share(sum) {
        for child in children.iter_mut() {
            child.ratio /= sum;
        }
    } else {
        equal_shares(children);
    }
}

pub(super) fn equal_shares(children: &mut [Child]) {
    let share = 1.0 / children.len().max(1) as f32;
    for child in children.iter_mut() {
        child.ratio = share;
    }
}

fn is_share(ratio: f32) -> bool {
    ratio.is_finite() && ratio > 0.0
}
