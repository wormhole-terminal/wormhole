//! Putting panes beside each other by drop: the tree's `insert_beside` and `move_pane`, and a
//! layout's `join`, `move_to` and `leave` with the minimum sizes, the focus and the zoom.

use super::tests::{H, W, grid, leaf, shares, split, tree};
use super::{Axis, DropError, Leave, PANE_MIN_H, PANE_MIN_W, Room, TabPanes, Tree, Zone};

fn room() -> Room {
    Room::new(W, H)
}

/// Panes 1, 2 and 3 side by side in equal shares, the keyboard on 2.
fn three() -> TabPanes {
    let mut panes = TabPanes::restored(None, &[1, 2, 3]).expect("three panes");
    assert!(panes.focus(2));
    panes
}

// ---------------------------------------------------------------- the tree

#[test]
fn a_pane_goes_on_the_side_of_the_target_the_zone_names() {
    let beside = |zone| {
        let mut t = Tree::single(1);
        assert!(t.insert_beside(1, zone, 2));
        t
    };
    assert_eq!(beside(Zone::Right).root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert_eq!(beside(Zone::Left).root(), &split(Axis::Row, vec![(0.5, leaf(2)), (0.5, leaf(1))]));
    assert_eq!(beside(Zone::Bottom).root(), &split(Axis::Column, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert_eq!(beside(Zone::Top).root(), &split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(1))]));
}

#[test]
fn beside_a_pane_of_a_split_of_the_same_axis_is_a_sibling_out_of_its_share_and_across_it_a_nest() {
    let mut t = tree(split(Axis::Row, vec![(0.6, leaf(1)), (0.4, leaf(2))]));
    assert!(t.insert_beside(1, Zone::Left, 3));
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.3, leaf(3)), (0.3, leaf(1)), (0.4, leaf(2))]));
    assert!(t.insert_beside(2, Zone::Top, 4));
    assert_eq!(
        t.root(),
        &split(
            Axis::Row,
            vec![
                (0.3, leaf(3)),
                (0.3, leaf(1)),
                (0.4, split(Axis::Column, vec![(0.5, leaf(4)), (0.5, leaf(2))]))
            ]
        )
    );
    assert_eq!(t.panes(), [3, 1, 4, 2]);
}

#[test]
fn splitting_right_or_down_is_inserting_after() {
    let mut split_down = grid();
    let mut inserted = grid();
    assert!(split_down.split(2, Axis::Row, 5));
    assert!(inserted.insert_beside(2, Zone::Right, 5));
    assert_eq!(split_down, inserted);
    assert!(split_down.split(1, Axis::Column, 6));
    assert!(inserted.insert_beside(1, Zone::Bottom, 6));
    assert_eq!(split_down, inserted);
}

#[test]
fn inserting_refuses_an_unknown_target_and_an_id_that_is_here() {
    let mut t = grid();
    assert!(!t.insert_beside(9, Zone::Left, 5), "no such pane");
    assert!(!t.insert_beside(1, Zone::Left, 4), "the id is taken");
    assert!(!t.insert_beside(1, Zone::Left, 1), "not beside itself");
    assert_eq!(t, grid());
}

#[test]
fn moving_a_pane_leaves_its_room_to_its_siblings_and_takes_half_of_the_targets() {
    let mut t = tree(split(Axis::Row, vec![(0.2, leaf(1)), (0.3, leaf(2)), (0.5, leaf(3))]));
    assert!(t.move_pane(3, 1, Zone::Left));
    // 1 and 2 shared 3's half in proportion (0.4, 0.6), then 1 gave half of its 0.4 to 3.
    assert_eq!(t.panes(), [3, 1, 2]);
    assert_eq!(shares(&t), [0.2, 0.2, 0.6]);
}

#[test]
fn moving_a_pane_across_the_axis_nests_it_and_a_split_left_with_one_gives_way() {
    let mut t = tree(split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));
    assert!(t.move_pane(2, 1, Zone::Top));
    assert_eq!(t.root(), &split(Axis::Column, vec![(0.5, leaf(2)), (0.5, leaf(1))]));
    assert!(t.move_pane(1, 2, Zone::Left));
    assert_eq!(t.root(), &split(Axis::Row, vec![(0.5, leaf(1)), (0.5, leaf(2))]));

    // 3 leaves the left column of the grid, which gives way to 1, and lands in the right one.
    let mut t = grid();
    assert!(t.move_pane(3, 2, Zone::Right));
    assert_eq!(
        t.root(),
        &split(
            Axis::Row,
            vec![
                (0.5, leaf(1)),
                (
                    0.5,
                    split(
                        Axis::Column,
                        vec![(0.5, split(Axis::Row, vec![(0.5, leaf(2)), (0.5, leaf(3))])), (0.5, leaf(4))]
                    )
                )
            ]
        )
    );
    assert_eq!(t.panes(), [1, 2, 3, 4]);
}

#[test]
fn a_move_that_arranges_the_panes_as_they_are_changes_nothing() {
    let mut t = tree(split(Axis::Row, vec![(0.2, leaf(1)), (0.5, leaf(2)), (0.3, leaf(3))]));
    let before = t.clone();
    assert!(!t.move_pane(2, 1, Zone::Right), "2 is right of 1 already");
    assert!(!t.move_pane(1, 2, Zone::Left));
    assert!(!t.move_pane(2, 3, Zone::Left));
    assert!(!t.move_pane(2, 2, Zone::Left), "onto itself");
    assert!(!t.move_pane(9, 1, Zone::Left), "no such pane");
    assert!(!t.move_pane(1, 9, Zone::Left));
    assert_eq!(t, before, "even the shares are as they were");
    assert!(!Tree::single(1).move_pane(1, 1, Zone::Left));
    assert!(t.move_pane(1, 2, Zone::Right), "past it is a move");
    assert_eq!(t.panes(), [2, 1, 3]);
}

// ---------------------------------------------------------------- a layout

#[test]
fn a_terminal_joins_beside_the_target_takes_half_its_room_and_the_keyboard() {
    let mut panes = TabPanes::single(1);
    assert_eq!(panes.join(1, Zone::Right, 2, room()), Ok(()));
    assert_eq!(panes.panes(), [1, 2]);
    assert_eq!(panes.focused(), 2);
    assert_eq!(panes.join(1, Zone::Bottom, 3, room()), Ok(()));
    assert_eq!(panes.panes(), [1, 2, 3], "3 is below 1, so it reads after 2");
    let rects = panes.tree().rects(W, H);
    assert_eq!(rects[0].1, super::Rect::new(0.0, 0.0, 400.0, 300.0), "1 gave half of its height to 3");
    assert_eq!(panes.focused(), 3);
}

#[test]
fn a_join_shows_a_zoomed_pane_among_the_others_again() {
    let mut panes = three();
    assert!(panes.toggle_zoom());
    assert_eq!(panes.zoomed(), Some(2));
    assert_eq!(panes.join(3, Zone::Top, 4, room()), Ok(()));
    assert_eq!(panes.zoomed(), None);
    assert_eq!(panes.panes(), [1, 2, 4, 3]);
}

#[test]
fn a_join_refuses_what_is_not_there_and_what_would_not_fit() {
    let mut panes = three();
    let before = panes.clone();
    assert_eq!(panes.join(9, Zone::Left, 4, room()), Err(DropError::Unknown), "no such target");
    assert_eq!(panes.join(1, Zone::Left, 3, room()), Err(DropError::Unknown), "3 is in it already");
    // A third of 800 is 266: half of it is under the minimum width, but not the minimum height.
    assert_eq!(panes.join(1, Zone::Right, 4, room()), Err(DropError::TooSmall));
    assert_eq!(panes.join(1, Zone::Top, 4, Room::new(W, 150.0)), Err(DropError::TooSmall));
    assert_eq!(panes, before, "a refused drop changes nothing, the keyboard included");
    assert_eq!(panes.join(1, Zone::Top, 4, room()), Ok(()));
}

#[test]
fn the_minimum_is_the_callers_and_both_dimensions_count() {
    // The app's panes carry a header: the same room, a taller minimum.
    let tall = Room { min_h: PANE_MIN_H + 28.0, ..Room::new(W, 200.0) };
    assert_eq!(TabPanes::single(1).join(1, Zone::Bottom, 2, tall), Err(DropError::TooSmall));
    assert_eq!(TabPanes::single(1).join(1, Zone::Bottom, 2, Room::new(W, 200.0)), Ok(()));
    // Exactly the minimum fits.
    assert_eq!(TabPanes::single(1).join(1, Zone::Right, 2, Room::new(2.0 * PANE_MIN_W, H)), Ok(()));
    assert_eq!(
        TabPanes::single(1).join(1, Zone::Right, 2, Room::new(2.0 * PANE_MIN_W - 1.0, H)),
        Err(DropError::TooSmall)
    );
    // A tab that has not been laid out yet knows nothing, and the drop is allowed.
    assert_eq!(TabPanes::single(1).join(1, Zone::Right, 2, Room::new(0.0, 0.0)), Ok(()));
}

#[test]
fn only_the_two_panes_a_drop_touches_are_held_to_the_minimum() {
    // 2 and 3 are narrower than the minimum already (the window shrank); 1 has plenty.
    let mut panes = TabPanes::from_json(
        r#"{"tree":{"root":{"split":{"axis":"row","children":[
            {"ratio":0.8,"node":{"leaf":1}},{"ratio":0.1,"node":{"leaf":2}},{"ratio":0.1,"node":{"leaf":3}}]}}},
            "focused":1}"#,
    )
    .expect("a layout");
    assert_eq!(panes.join(1, Zone::Right, 4, room()), Ok(()), "1 and 4 get 320 each");
    assert_eq!(panes.join(2, Zone::Right, 5, room()), Err(DropError::TooSmall), "2 can't take it");
}

#[test]
fn can_join_and_can_move_to_say_what_would_happen_and_change_nothing() {
    let panes = three();
    let before = panes.clone();
    assert_eq!(panes.can_join(1, Zone::Top, 4, room()), Ok(()));
    assert_eq!(panes.can_join(1, Zone::Right, 4, room()), Err(DropError::TooSmall));
    assert_eq!(panes.can_join(1, Zone::Right, 2, room()), Err(DropError::Unknown));
    assert_eq!(panes.can_move_to(3, 1, Zone::Top, room()), Ok(()));
    assert_eq!(panes.can_move_to(1, 2, Zone::Left, room()), Err(DropError::Unchanged));
    assert_eq!(panes, before);
}

#[test]
fn a_pane_moved_by_its_header_lands_beside_the_target_with_the_keyboard() {
    let mut panes = three();
    assert_eq!(panes.focused(), 2);
    assert_eq!(panes.move_to(3, 1, Zone::Top, room()), Ok(()));
    // 3 left the row, so 1 and 2 are halves again, and 3 took half of 1's.
    assert_eq!(panes.panes(), [3, 2, 1]);
    assert_eq!(panes.tree().rects(W, H)[0], (3, super::Rect::new(0.0, 0.0, 400.0, 300.0)));
    assert_eq!(panes.focused(), 3);
    assert_eq!(panes.len(), 3, "a move never adds or drops a pane");
}

#[test]
fn a_move_refuses_what_is_not_there_what_stays_and_what_would_not_fit() {
    let mut panes = three();
    let before = panes.clone();
    assert_eq!(panes.move_to(9, 1, Zone::Left, room()), Err(DropError::Unknown));
    assert_eq!(panes.move_to(1, 9, Zone::Left, room()), Err(DropError::Unknown));
    assert_eq!(panes.move_to(1, 1, Zone::Left, room()), Err(DropError::Unchanged), "onto itself");
    assert_eq!(panes.move_to(1, 2, Zone::Left, room()), Err(DropError::Unchanged), "it is there already");
    // Stacked in a tab 180 tall, each would be 90 under the minimum height of 96.
    assert_eq!(panes.move_to(3, 1, Zone::Top, Room::new(W, 180.0)), Err(DropError::TooSmall));
    assert_eq!(panes, before);
}

#[test]
fn a_move_shows_a_zoomed_pane_among_the_others_again() {
    let mut panes = three();
    assert!(panes.toggle_zoom());
    assert_eq!(panes.move_to(3, 1, Zone::Left, room()), Ok(()));
    assert_eq!(panes.zoomed(), None);
    assert_eq!(panes.panes(), [3, 1, 2]);
}

#[test]
fn leaving_hands_the_room_to_the_panes_beside_and_the_keyboard_on() {
    let mut panes = three();
    assert_eq!(panes.leave(2), Some(Leave::Shrunk));
    assert_eq!(panes.panes(), [1, 3]);
    assert_eq!(panes.focused(), 3, "as when a pane closes: the one after it");
    let rects = panes.tree().rects(W, H);
    assert_eq!((rects[0].1.w, rects[1].1.w), (400.0, 400.0));
}

#[test]
fn a_split_left_with_one_pane_dissolves_and_the_last_pane_has_nothing_to_leave() {
    let mut panes = three();
    assert!(panes.toggle_zoom());
    assert_eq!(panes.leave(2), Some(Leave::Shrunk));
    assert_eq!(panes.zoomed(), None, "the zoomed pane left");
    assert_eq!(panes.leave(1), Some(Leave::Dissolved));
    assert!(!panes.is_split());
    assert_eq!((panes.panes(), panes.focused()), (vec![3], 3));
    assert_eq!(panes.leave(3), None, "the last one stays");
    assert_eq!(panes.leave(9), None, "not here");
    assert_eq!(panes, TabPanes::single(3));
}

#[test]
fn the_minimum_sizes_are_the_panes() {
    // The constants the room defaults to are the ones a divider drag holds.
    let room = Room::new(W, H);
    assert_eq!((room.min_w, room.min_h), (PANE_MIN_W, PANE_MIN_H));
}
