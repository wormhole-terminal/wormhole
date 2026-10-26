use super::geometry::{DividerId, Rect};
use super::preset::{Plan, Preset, plan};
use super::{Axis, Child, Direction, Node, PANE_MIN_H, PANE_MIN_W, PaneId, Room, TabPanes, Tree};

pub(super) const W: f32 = 800.0;
pub(super) const H: f32 = 600.0;

pub(super) fn leaf(id: PaneId) -> Node {
    Node::Leaf(id)
}

pub(super) fn split(axis: Axis, children: Vec<(f32, Node)>) -> Node {
    Node::Split { axis, children: children.into_iter().map(|(ratio, node)| Child { ratio, node }).collect() }
}

pub(super) fn tree(root: Node) -> Tree {
    Tree::from_root(root).expect("a tree with panes")
}

/// Ratios of the root's children.
pub(super) fn shares(tree: &Tree) -> Vec<f32> {
    match tree.root() {
        Node::Split { children, .. } => {
            children.iter().map(|c| (c.ratio * 1000.0).round() / 1000.0).collect()
        }
        Node::Leaf(_) => Vec::new(),
    }
}

pub(super) fn grid() -> Tree {
    // The 2×2: 1 and 2 on top, 3 and 4 below (two columns of two).
    tree(split(
        Axis::Row,
        vec![
            (0.5, split(Axis::Column, vec![(0.5, leaf(1)), (0.5, leaf(3))])),
            (0.5, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(4))])),
        ],
    ))
}

fn divider(path: &[usize], index: usize) -> DividerId {
    DividerId { path: path.to_vec(), index }
}

fn drag(tree: &mut Tree, divider: &DividerId, to: f32) -> Option<f32> {
    tree.drag(divider, to, W, H, PANE_MIN_W, PANE_MIN_H)
}

// ---------------------------------------------------------------- the tree

#[test]
fn split_right_puts_the_new_pane_beside_and_split_down_below_each_taking_half() {
    let mut t = Tree::single(1);
    assert!(t.split(1, Axis::Row, 2));
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert!(t.split(2, Axis::Column, 3));
    assert_eq!(
        t.root(),
        &split(
            Axis::Row,
            vec![(0.5, leaf(1)), (0.5, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(3))]))]
        )
    );
    assert_eq!(t.panes(), [1, 2, 3]);
    assert_eq!(t.len(), 3);
}

#[test]
fn splitting_along_the_parents_axis_adds_a_sibling_out_of_the_targets_share() {
    let mut t = tree(split(Axis::Row, vec![(0.6, leaf(1)), (0.4, leaf(2))]));
    assert!(t.split(1, Axis::Row, 3));
    // Three side by side, not a row inside a row; pane 2 keeps its 0.4.
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.3, leaf(1)), (0.3, leaf(3)), (0.4, leaf(2))]));
    assert_eq!(t.panes(), [1, 3, 2]);
}

#[test]
fn a_split_refuses_an_unknown_target_and_a_pane_that_is_here_already() {
    let mut t = Tree::single(1);
    assert!(!t.split(9, Axis::Row, 2), "no such pane");
    assert!(!t.split(1, Axis::Row, 1), "the id is taken");
    assert_eq!(t, Tree::single(1));
}

#[test]
fn closing_a_pane_gives_its_share_to_its_siblings_in_proportion() {
    let mut t = tree(split(Axis::Row, vec![(0.5, leaf(1)), (0.3, leaf(2)), (0.2, leaf(3))]));
    assert!(t.remove(1));
    assert_eq!(shares(&t), [0.6, 0.4]);
    assert!(t.remove(3));
    assert_eq!(t, Tree::single(2), "a split with one child gives way to it");
    assert!(!t.remove(2), "the last pane stays");
    assert!(!t.remove(7), "nothing to close");
}

#[test]
fn closing_a_pane_merges_a_split_that_is_left_inside_one_of_its_own_axis() {
    // Row[1, Column[2, Row[3, 4]]]: closing 2 leaves Row[1, Row[3, 4]], which is Row[1, 3, 4].
    let mut t = tree(split(
        Axis::Row,
        vec![
            (0.5, leaf(1)),
            (
                0.5,
                split(
                    Axis::Column,
                    vec![(0.5, leaf(2)), (0.5, split(Axis::Row, vec![(0.5, leaf(3)), (0.5, leaf(4))]))],
                ),
            ),
        ],
    ));
    assert!(t.remove(2));
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.25, leaf(3)), (0.25, leaf(4))]));
}

#[test]
fn reading_order_is_left_to_right_then_top_to_bottom_whatever_the_shares() {
    assert_eq!(grid().panes(), [1, 2, 3, 4]);
    // The same four with the right column's rows swapped by a drag: still read as laid out.
    let mut uneven = grid();
    assert!(drag(&mut uneven, &divider(&[], 0), 100.0).is_some());
    assert!(drag(&mut uneven, &divider(&[1], 0), 500.0).is_some());
    assert_eq!(uneven.panes(), [1, 2, 3, 4], "a drag never reorders the panes");
    // A column of rows reads the same way.
    let rows = tree(split(
        Axis::Column,
        vec![
            (0.5, split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))])),
            (0.5, split(Axis::Row, vec![(0.5, leaf(3)), (0.5, leaf(4))])),
        ],
    ));
    assert_eq!(rows.panes(), [1, 2, 3, 4]);
}

#[test]
fn appending_puts_a_pane_at_the_right_end_with_an_equal_share() {
    let mut t = Tree::single(1);
    assert!(t.append(2));
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert!(t.append(3));
    assert_eq!(t.panes(), [1, 2, 3]);
    assert!(shares(&t).iter().all(|share| (share - 1.0 / 3.0).abs() < 0.001));
    assert!(!t.append(2), "already here");
    // A column at the root gets a row round it.
    let mut column = tree(split(Axis::Column, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert!(column.append(3));
    assert_eq!(column.panes(), [1, 3, 2], "3 beside the column, which reads top to bottom after it");
    assert_eq!(column.len(), 3);
}

// ---------------------------------------------------------------- geometry

#[test]
fn panes_get_their_share_of_the_tab_and_the_last_edge_is_the_tabs_own() {
    let t = tree(split(
        Axis::Row,
        vec![(0.25, leaf(1)), (0.75, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(3))]))],
    ));
    let rects = t.rects(W, H);
    assert_eq!(rects[0], (1, Rect::new(0.0, 0.0, 200.0, 600.0)));
    assert_eq!(rects[1], (2, Rect::new(200.0, 0.0, 600.0, 300.0)));
    assert_eq!(rects[2], (3, Rect::new(200.0, 300.0, 600.0, 300.0)));
    // Shares that don't quite sum to 1 after a round trip still fill the tab.
    let rounded = tree(split(Axis::Row, vec![(0.33333, leaf(1)), (0.33333, leaf(2)), (0.33333, leaf(3))]));
    let last = rounded.rects(W, H)[2].1;
    assert_eq!(last.right(), W);
}

#[test]
fn dividers_lie_on_the_shared_edges_splits_before_their_children() {
    let t = tree(split(
        Axis::Row,
        vec![(0.25, leaf(1)), (0.75, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(3))]))],
    ));
    let dividers = t.dividers(W, H);
    assert_eq!(dividers.len(), 2);
    assert_eq!(dividers[0].id, divider(&[], 0));
    assert_eq!((dividers[0].axis, dividers[0].rect), (Axis::Row, Rect::new(200.0, 0.0, 0.0, 600.0)));
    assert_eq!(dividers[0].position(), 200.0);
    assert_eq!(dividers[1].id, divider(&[1], 0));
    assert_eq!((dividers[1].axis, dividers[1].rect), (Axis::Column, Rect::new(200.0, 300.0, 600.0, 0.0)));
    assert_eq!(dividers[1].position(), 300.0);
    assert!(Tree::single(1).dividers(W, H).is_empty());
}

#[test]
fn dragging_a_divider_moves_only_the_two_panes_it_separates() {
    let mut t = tree(split(Axis::Row, vec![(0.25, leaf(1)), (0.25, leaf(2)), (0.5, leaf(3))]));
    assert_eq!(drag(&mut t, &divider(&[], 0), 220.0), Some(220.0));
    assert_eq!(shares(&t), [0.275, 0.225, 0.5], "pane 3 is untouched");
    assert_eq!(t.rects(W, H)[1].1, Rect::new(220.0, 0.0, 180.0, 600.0));
}

#[test]
fn a_drag_stops_where_a_pane_would_get_too_small() {
    let mut t = tree(split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert_eq!(drag(&mut t, &divider(&[], 0), 20.0), Some(PANE_MIN_W));
    assert_eq!(drag(&mut t, &divider(&[], 0), 790.0), Some(W - PANE_MIN_W));
    // Panes inside the two sides count too: a row of two inside the left side needs two widths.
    let mut nested = tree(split(
        Axis::Row,
        vec![
            (
                0.5,
                split(
                    Axis::Column,
                    vec![(0.5, split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))])), (0.5, leaf(5))],
                ),
            ),
            (0.5, leaf(3)),
        ],
    ));
    assert_eq!(drag(&mut nested, &divider(&[], 0), 100.0), Some(2.0 * PANE_MIN_W));
    // The pane on the other side holds the divider too.
    let mut three = tree(split(Axis::Row, vec![(0.25, leaf(1)), (0.25, leaf(2)), (0.5, leaf(3))]));
    assert_eq!(drag(&mut three, &divider(&[], 0), 300.0), Some(400.0 - PANE_MIN_W), "pane 2 keeps its width");
    // Across the axis the tallest minimum counts once: a column beside a pane needs one width.
    let mut beside = tree(split(
        Axis::Row,
        vec![(0.5, leaf(1)), (0.5, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(3))]))],
    ));
    assert_eq!(drag(&mut beside, &divider(&[], 0), 790.0), Some(W - PANE_MIN_W));
    // A horizontal divider is held by heights, and the inner split's divider by its own rect.
    let mut rows = tree(split(
        Axis::Column,
        vec![(0.5, split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))])), (0.5, leaf(3))],
    ));
    assert_eq!(drag(&mut rows, &divider(&[], 0), 10.0), Some(PANE_MIN_H));
    assert_eq!(drag(&mut rows, &divider(&[], 0), 590.0), Some(H - PANE_MIN_H));
    assert_eq!(drag(&mut rows, &divider(&[0], 0), 50.0), Some(PANE_MIN_W));
}

#[test]
fn a_drag_in_a_tab_too_small_for_both_minimums_changes_nothing() {
    let mut t = tree(split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert_eq!(t.drag(&divider(&[], 0), 100.0, 200.0, H, PANE_MIN_W, PANE_MIN_H), None);
    assert_eq!(shares(&t), [0.5, 0.5]);
    assert_eq!(drag(&mut t, &divider(&[], 5), 100.0), None, "no such divider");
    assert_eq!(drag(&mut t, &divider(&[3], 0), 100.0), None, "no such split");
    assert_eq!(drag(&mut Tree::single(1), &divider(&[], 0), 100.0), None);
}

fn room() -> Room {
    Room::new(W, H)
}

/// A percent without the float's last digits.
fn rounded(percent: Option<f32>) -> Option<f32> {
    percent.map(|p| (p * 100.0).round() / 100.0)
}

#[test]
fn a_share_is_set_out_of_the_neighbour_and_nothing_else_moves() {
    let mut panes = TabPanes::single(1);
    assert!(panes.split(Axis::Row, 2));
    assert_eq!(rounded(panes.set_share(2, 1, 30.0, room())), Some(30.0));
    assert_eq!(shares(panes.tree()), [0.7, 0.3]);

    // In a row of three the pair keeps what it had between them: pane 3 is untouched.
    let mut three = TabPanes::restored(None, &[1, 2, 3]).expect("three panes");
    assert_eq!(shares(three.tree()), [0.333, 0.333, 0.333]);
    assert_eq!(rounded(three.set_share(2, 1, 20.0, room())), Some(20.0));
    assert_eq!(shares(three.tree()), [0.467, 0.2, 0.333]);

    // The neighbour may be on either side; the share is held where it would leave it under a width.
    let held = three.set_share(2, 3, 90.0, room()).expect("held, not refused");
    assert!((held - 100.0 * (1.0 - 0.467 - 0.2)).abs() < 0.2, "{held}");
    assert_eq!(shares(three.tree()), [0.467, 0.333, 0.2], "pane 3 keeps its minimum width");
    let least = three.set_share(2, 3, 1.0, room()).expect("held, not refused");
    assert!((least - 20.0).abs() < 0.1, "{least}: pane 2 keeps its own minimum");
}

#[test]
fn a_share_is_refused_between_panes_that_are_not_neighbours_or_in_a_tab_too_small_for_both() {
    let mut three = TabPanes::restored(None, &[1, 2, 3]).expect("three panes");
    let before = three.clone();
    assert_eq!(three.set_share(3, 1, 30.0, room()), None, "pane 2 is between them");
    assert_eq!(three.set_share(9, 1, 30.0, room()), None, "no such pane");
    assert_eq!(three.set_share(1, 9, 30.0, room()), None, "no such neighbour");
    assert_eq!(three.set_share(1, 1, 30.0, room()), None, "a pane is not its own neighbour");
    assert_eq!(three.set_share(1, 2, 30.0, Room::new(300.0, H)), None, "two minimums do not fit in 300");
    assert_eq!(three, before, "nothing changed");
    // Before the first frame the room is not known: the share is taken as given, both panes kept alive.
    assert_eq!(rounded(three.set_share(1, 2, 30.0, Room::new(0.0, 0.0))), Some(30.0));
    let alive = three.set_share(1, 2, 0.0, Room::new(0.0, 0.0)).expect("kept alive");
    assert!(alive > 0.0 && alive <= 1.0, "{alive}");
}

#[test]
fn a_share_inside_a_nested_split_is_its_share_of_that_split() {
    let mut panes = TabPanes::single(1);
    assert!(panes.split(Axis::Row, 2));
    assert!(panes.focus(2));
    assert!(panes.split(Axis::Column, 4));
    // 2 above 4, in the right half: 4 takes a quarter of that column and the row is left alone.
    assert_eq!(rounded(panes.set_share(4, 2, 25.0, room())), Some(25.0));
    assert_eq!(shares(panes.tree()), [0.5, 0.5]);
    let rects = panes.tree().rects(W, H);
    let height_of = |id| rects.iter().find(|(pane, _)| *pane == id).map(|(_, rect)| rect.h).expect("a pane");
    assert_eq!((height_of(2), height_of(4)), (450.0, 150.0));
}

#[test]
fn a_double_click_equalizes_the_dividers_split_and_that_one_only() {
    let mut t = tree(split(
        Axis::Row,
        vec![(0.7, leaf(1)), (0.3, split(Axis::Column, vec![(0.8, leaf(2)), (0.2, leaf(3))]))],
    ));
    assert!(t.equalize(&divider(&[1], 0)));
    assert_eq!(shares(&t), [0.7, 0.3], "the outer split keeps its shares");
    assert_eq!(t.rects(W, H)[1].1.h, 300.0);
    assert!(t.equalize(&divider(&[], 0)));
    assert_eq!(shares(&t), [0.5, 0.5]);
    assert!(!t.equalize(&divider(&[], 1)), "no such divider");
    let mut all = tree(split(
        Axis::Row,
        vec![(0.7, leaf(1)), (0.3, split(Axis::Column, vec![(0.8, leaf(2)), (0.2, leaf(3))]))],
    ));
    all.equalize_all();
    assert_eq!(
        all.rects(W, H).iter().map(|(_, r)| (r.w, r.h)).collect::<Vec<_>>(),
        [(400.0, 600.0), (400.0, 300.0), (400.0, 300.0)]
    );
}

#[test]
fn the_neighbour_in_a_direction_shares_an_edge_and_overlaps_most() {
    let g = grid();
    let neighbour = |from, direction| g.neighbour(from, direction, W, H);
    assert_eq!(neighbour(1, Direction::Right), Some(2));
    assert_eq!(neighbour(1, Direction::Down), Some(3));
    assert_eq!(neighbour(1, Direction::Left), None, "the tab's edge: the app decides whether to wrap");
    assert_eq!(neighbour(1, Direction::Up), None);
    assert_eq!(neighbour(4, Direction::Up), Some(2));
    assert_eq!(neighbour(4, Direction::Left), Some(3));
    assert_eq!(neighbour(9, Direction::Left), None, "no such pane");
    // Beside a column, the pane that overlaps the most wins; from inside it, the one pane.
    let uneven = tree(split(
        Axis::Row,
        vec![(0.5, leaf(1)), (0.5, split(Axis::Column, vec![(0.3, leaf(2)), (0.7, leaf(3))]))],
    ));
    assert_eq!(uneven.neighbour(1, Direction::Right, W, H), Some(3));
    assert_eq!(uneven.neighbour(2, Direction::Left, W, H), Some(1));
    assert_eq!(uneven.neighbour(2, Direction::Down, W, H), Some(3));
    // Touching at a corner only is not a neighbour: 1 is not below 2 in the grid.
    assert_eq!(neighbour(2, Direction::Down), Some(4));
    // At a tie in overlap the nearer centre wins.
    let tie = tree(split(
        Axis::Row,
        vec![
            (0.5, split(Axis::Column, vec![(0.2, leaf(1)), (0.8, leaf(2))])),
            (0.5, split(Axis::Column, vec![(0.5, leaf(3)), (0.5, leaf(4))])),
        ],
    ));
    assert_eq!(tie.neighbour(2, Direction::Right, W, H), Some(4), "2 spans y 120..600; 4 overlaps 300 of it");
    assert_eq!(tie.neighbour(3, Direction::Left, W, H), Some(2), "3 spans 0..300: 1 has 120, 2 has 180");
}

// ---------------------------------------------------------------- the saved record

#[test]
fn a_tree_round_trips_through_json_in_the_documented_shape() {
    let t = tree(split(
        Axis::Row,
        vec![(0.25, leaf(1)), (0.75, split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(3))]))],
    ));
    let json = t.to_json();
    assert_eq!(
        json,
        r#"{"root":{"split":{"axis":"row","children":[{"ratio":0.25,"node":{"leaf":1}},{"ratio":0.75,"node":{"split":{"axis":"column","children":[{"ratio":0.5,"node":{"leaf":2}},{"ratio":0.5,"node":{"leaf":3}}]}}}]}}}"#
    );
    assert_eq!(Tree::from_json(&json), Some(t));
    assert_eq!(Tree::from_json("not json"), None);
    assert_eq!(Tree::from_json(r#"{"root":{"split":{"axis":"row","children":[]}}}"#), None, "no pane left");
}

#[test]
fn a_damaged_record_is_repaired_rather_than_refused() {
    // Duplicates: the later one goes. An empty split goes. A lone child replaces its split.
    let json = r#"{"root":{"split":{"axis":"row","children":[
        {"ratio":0.5,"node":{"leaf":1}},
        {"ratio":0.3,"node":{"split":{"axis":"column","children":[{"ratio":1,"node":{"leaf":1}}]}}},
        {"ratio":0.2,"node":{"split":{"axis":"column","children":[{"ratio":1,"node":{"leaf":2}}]}}}
    ]}}}"#;
    let t = Tree::from_json(json).expect("repaired");
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.5 / 0.7, leaf(1)), (0.2 / 0.7, leaf(2))]));
    // Ratios that aren't shares (missing, zero, negative, not a number) become equal shares.
    let json = r#"{"root":{"split":{"axis":"column","children":[
        {"node":{"leaf":1}},{"ratio":-2,"node":{"leaf":2}},{"ratio":0,"node":{"leaf":3}}
    ]}}}"#;
    let t = Tree::from_json(json).expect("repaired");
    assert!(shares(&t).iter().all(|s| (s - 1.0 / 3.0).abs() < 0.001));
    let nan = Tree::from_root(split(Axis::Row, vec![(f32::NAN, leaf(1)), (0.5, leaf(2))])).expect("repaired");
    assert_eq!(shares(&nan), [0.5, 0.5]);
    // Same-axis nesting is merged, the inner shares scaled into the child's.
    let nested = Tree::from_root(split(
        Axis::Row,
        vec![(0.5, leaf(1)), (0.5, split(Axis::Row, vec![(0.5, leaf(2)), (0.5, leaf(3))]))],
    ))
    .expect("repaired");
    assert_eq!(nested.root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.25, leaf(2)), (0.25, leaf(3))]));
    assert_eq!(Tree::from_root(split(Axis::Row, vec![])), None);
}

// ---------------------------------------------------------------- a tab's panes

#[test]
fn a_split_focuses_the_new_pane_and_closing_the_focused_one_moves_the_keyboard_on() {
    let mut tab = TabPanes::single(1);
    assert!(!tab.is_split());
    assert!(tab.split(Axis::Row, 2));
    assert_eq!((tab.focused(), tab.panes()), (2, vec![1, 2]));
    assert!(tab.split(Axis::Column, 3));
    assert_eq!((tab.focused(), tab.panes()), (3, vec![1, 2, 3]));
    assert!(!tab.split(Axis::Row, 1), "the id is a pane already");
    // Closing the focused pane: the next in reading order, or the previous when it was last.
    assert!(tab.focus(2));
    assert!(tab.close(2));
    assert_eq!(tab.focused(), 3);
    assert!(tab.close(3));
    assert_eq!((tab.focused(), tab.len()), (1, 1));
    assert!(!tab.close(1), "the last pane stays: the tab closes instead");
    // Closing another pane leaves the keyboard where it is.
    assert!(tab.split(Axis::Row, 4));
    assert!(tab.focus(1));
    assert!(tab.close(4));
    assert_eq!(tab.focused(), 1);
    assert!(!tab.focus(9), "no such pane");
}

#[test]
fn zoom_fills_the_tab_with_the_focused_pane_until_something_moves() {
    let mut tab = TabPanes::single(1);
    assert!(!tab.toggle_zoom(), "nothing to zoom");
    assert!(tab.split(Axis::Row, 2));
    assert!(tab.toggle_zoom());
    assert_eq!(tab.zoomed(), Some(2));
    assert_eq!(tab.shown(W, H), [(2, Rect::new(0.0, 0.0, W, H))]);
    assert!(tab.dividers(W, H).is_empty());
    assert!(tab.toggle_zoom());
    assert_eq!(tab.zoomed(), None);
    assert_eq!(tab.shown(W, H).len(), 2);
    assert_eq!(tab.dividers(W, H).len(), 1);
    // Moving the keyboard to another pane brings them all back; a split does too.
    assert!(tab.toggle_zoom());
    assert!(tab.focus(2), "the zoomed pane itself keeps the zoom");
    assert_eq!(tab.zoomed(), Some(2));
    assert!(tab.focus(1));
    assert_eq!(tab.zoomed(), None);
    assert!(tab.toggle_zoom());
    assert!(tab.split(Axis::Column, 3));
    assert_eq!(tab.zoomed(), None);
    // Closing the zoomed pane, or the one beside it, leaves nothing zoomed.
    assert!(tab.toggle_zoom());
    assert_eq!(tab.zoomed(), Some(3));
    assert!(tab.close(3));
    assert_eq!(tab.zoomed(), None);
    assert!(tab.toggle_zoom());
    assert!(tab.close(2));
    assert_eq!((tab.zoomed(), tab.len()), (None, 1));
}

#[test]
fn the_keyboard_moves_to_the_neighbour_of_the_focused_pane() {
    let mut tab = TabPanes::single(1);
    assert!(tab.split(Axis::Row, 2) && tab.split(Axis::Column, 3));
    assert_eq!(tab.focused(), 3);
    assert_eq!(tab.neighbour(Direction::Up, W, H), Some(2));
    assert_eq!(tab.neighbour(Direction::Left, W, H), Some(1));
    assert_eq!(tab.neighbour(Direction::Right, W, H), None);
}

#[test]
fn a_tabs_layout_round_trips_and_a_damaged_one_settles_on_panes_that_exist() {
    let mut tab = TabPanes::single(1);
    assert!(tab.split(Axis::Row, 2) && tab.toggle_zoom());
    let json = tab.to_json();
    assert!(
        json.starts_with(r#"{"tree":{"root":"#) && json.ends_with(r#""focused":2,"zoomed":2}"#),
        "{json}"
    );
    assert_eq!(TabPanes::from_json(&json), Some(tab));
    // The focused pane is gone: the first has the keyboard. A zoom on a gone pane is dropped.
    let json = r#"{"tree":{"root":{"split":{"axis":"row","children":[{"ratio":0.5,"node":{"leaf":1}},{"ratio":0.5,"node":{"leaf":2}}]}}},"focused":7,"zoomed":7}"#;
    let tab = TabPanes::from_json(json).expect("repaired");
    assert_eq!((tab.focused(), tab.zoomed()), (1, None));
    // No focus saved at all; a zoom in a tab that isn't split means nothing.
    let json = r#"{"tree":{"root":{"leaf":4}},"zoomed":4}"#;
    let tab = TabPanes::from_json(json).expect("repaired");
    assert_eq!((tab.focused(), tab.zoomed()), (4, None));
    assert_eq!(TabPanes::from_json(r#"{"tree":{"root":{"split":{"axis":"row","children":[]}}}}"#), None);
    assert_eq!(TabPanes::from_json("[]"), None);
}

#[test]
fn a_restored_tab_fits_its_layout_to_the_panes_the_store_has() {
    // No layout: the panes side by side in their order, the first focused.
    let tab = TabPanes::restored(None, &[5, 6, 7]).expect("panes");
    assert_eq!((tab.panes(), tab.focused()), (vec![5, 6, 7], 5));
    assert_eq!(tab.preset(), None);
    assert_eq!(TabPanes::restored(None, &[5]).expect("panes"), TabPanes::single(5));
    assert_eq!(TabPanes::restored(None, &[]), None, "a tab without panes isn't one");
    // The layout names a pane that has no row (dropped) and misses one that has (appended).
    let mut saved = TabPanes::single(1);
    assert!(saved.split(Axis::Column, 2) && saved.focus(1) && saved.toggle_zoom());
    let tab = TabPanes::restored(Some(&saved.to_json()), &[2, 3]).expect("panes");
    assert_eq!(tab.panes(), [2, 3], "1 is gone, 3 is new at the right");
    assert_eq!((tab.focused(), tab.zoomed()), (2, None), "the focus and the zoom were 1's");
    // Every pane of the layout is gone: start from the rows.
    let tab = TabPanes::restored(Some(&saved.to_json()), &[8, 9]).expect("panes");
    assert_eq!(tab.panes(), [8, 9]);
    // Unreadable JSON: the rows.
    let tab = TabPanes::restored(Some("{"), &[8]).expect("panes");
    assert_eq!(tab, TabPanes::single(8));
    // A layout that matches its rows comes back as saved.
    assert!(saved.focus(2));
    assert_eq!(TabPanes::restored(Some(&saved.to_json()), &[1, 2]), Some(saved));
}

// ---------------------------------------------------------------- presets

#[test]
fn a_preset_keeps_the_first_panes_and_the_focused_one_and_the_rest_leave() {
    let mut tab = TabPanes::single(1);
    assert_eq!(tab.plan(Preset::Grid), Plan { keep: vec![1], new: 3, out: vec![] });
    assert_eq!(tab.plan(Preset::Single), Plan { keep: vec![1], new: 0, out: vec![] });
    assert!(tab.split(Axis::Row, 2) && tab.split(Axis::Row, 3));
    assert_eq!(tab.panes(), [1, 2, 3]);
    // Focused 3, the last: it takes the last slot from 2.
    assert_eq!(tab.plan(Preset::TwoColumns), Plan { keep: vec![1, 3], new: 0, out: vec![2] });
    assert_eq!(tab.plan(Preset::Single), Plan { keep: vec![3], new: 0, out: vec![1, 2] });
    assert!(tab.focus(1));
    assert_eq!(tab.plan(Preset::TwoRows), Plan { keep: vec![1, 2], new: 0, out: vec![3] });
    assert_eq!(tab.plan(Preset::Grid), Plan { keep: vec![1, 2, 3], new: 1, out: vec![] });
    assert_eq!(plan(tab.tree(), 1, Preset::Grid), tab.plan(Preset::Grid));
}

#[test]
fn arranging_builds_the_presets_shape_and_the_menu_recognises_it() {
    let mut tab = TabPanes::single(1);
    assert_eq!(tab.preset(), Some(Preset::Single));
    assert_eq!(tab.arrange(Preset::Grid, &[2, 3]), None, "three new panes are needed");
    assert_eq!(tab.len(), 1);
    assert_eq!(tab.arrange(Preset::Grid, &[2, 3, 4, 5]), Some(vec![]), "the first three ids are used");
    assert_eq!(tab.tree(), &grid());
    assert_eq!(tab.preset(), Some(Preset::Grid));
    assert_eq!(tab.focused(), 1, "the focused pane stays");
    let rects = tab.shown(W, H);
    assert_eq!(rects.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert!(rects.iter().all(|(_, r)| r.w == 400.0 && r.h == 300.0));
    // Down to two columns: panes 3 and 4 leave for tabs of their own.
    assert!(tab.focus(4) && tab.toggle_zoom());
    assert_eq!(tab.arrange(Preset::TwoColumns, &[]), Some(vec![2, 3]), "4 takes 2's slot, 2 leaves");
    assert_eq!((tab.panes(), tab.focused(), tab.zoomed()), (vec![1, 4], 4, None));
    assert_eq!(tab.preset(), Some(Preset::TwoColumns));
    assert_eq!(tab.arrange(Preset::TwoRows, &[]), Some(vec![]));
    assert_eq!(tab.preset(), Some(Preset::TwoRows));
    assert_eq!(tab.shown(W, H)[1].1, Rect::new(0.0, 300.0, W, 300.0));
    assert_eq!(tab.arrange(Preset::Single, &[]), Some(vec![1]));
    assert_eq!((tab.panes(), tab.focused()), (vec![4], 4));
    // Shapes that are no preset: three in a row, a row beside a column.
    let mut three = TabPanes::single(1);
    assert!(three.split(Axis::Row, 2) && three.split(Axis::Row, 3));
    assert_eq!(three.preset(), None);
    let mut lopsided = TabPanes::single(1);
    assert!(lopsided.split(Axis::Row, 2) && lopsided.split(Axis::Column, 3));
    assert_eq!(lopsided.preset(), None);
    // A column of two rows is a grid too, and a dragged grid still is.
    let rows = tree(split(
        Axis::Column,
        vec![
            (0.3, split(Axis::Row, vec![(0.2, leaf(1)), (0.8, leaf(2))])),
            (0.7, split(Axis::Row, vec![(0.5, leaf(3)), (0.5, leaf(4))])),
        ],
    ));
    assert_eq!(rows.preset(), Some(Preset::Grid));
}

#[test]
fn the_presets_are_the_menus_four_with_their_slots_and_titles() {
    assert_eq!(Preset::ALL.map(Preset::title), ["2 Columns", "2 Rows", "2×2 Grid", "Single Pane"]);
    assert_eq!(Preset::ALL.map(Preset::slots), [2, 2, 4, 1]);
}
