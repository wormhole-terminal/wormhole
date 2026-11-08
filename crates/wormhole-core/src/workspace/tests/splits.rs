//! Splits: a new pane is a new terminal, a split's members share the list's card and the pin, and
//! what is shown follows the selection (the app's `pane_tests` and `structure_tests`, in-process).

use std::path::PathBuf;

use crate::view::panes::{Axis, Direction, Zone};
use crate::workspace::{Action, Event, Refusal, TerminalFacts};

use super::{listed, model, refused};

#[test]
fn splitting_makes_a_terminal_in_the_same_folder_beside_and_selects_it() {
    let mut m = model(1);
    m.update_facts(1, |f| f.cwd = Some(PathBuf::from("/work/app")));
    let events = m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    assert!(events.contains(&Event::Opened { id: 2, cwd: Some(PathBuf::from("/work/app")) }));
    assert_eq!(m.selected(), Some(2));
    let snapshot = m.sidebar_snapshot(0);
    let rows = &snapshot.sections[0].rows;
    assert_eq!(rows.iter().map(|r| (r.id, r.split.members)).collect::<Vec<_>>(), [(1, 2), (2, 2)]);
    assert!(rows[0].split.on_screen, "the other pane shows beside the selected one");
    assert_eq!(rows[0].split.id, rows[1].split.id);
    let panes = m.panes_snapshot(1000.0, 600.0);
    assert_eq!(panes.panes.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(panes.focused, Some(2));
    assert!((panes.panes[0].w - panes.panes[1].w).abs() < 1.0, "side by side, halves");
}

#[test]
fn a_split_stands_in_one_card_and_its_pane_joins_its_group() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![2] });
    m.apply(Action::SplitBeside { anchor: 2, axis: Axis::Column });
    assert_eq!(
        listed(&m),
        ["1:1", "[Group 1]", "2:2", "4:3", "3:4"],
        "the new pane sits after its mate, in its group"
    );
}

#[test]
fn the_pin_is_the_splits() {
    let mut m = model(2);
    m.apply(Action::SplitBeside { anchor: 2, axis: Axis::Row });
    m.apply(Action::TogglePin(2));
    let rows = &m.sidebar_snapshot(0).sections[0].rows;
    assert!(rows.iter().filter(|r| r.split.members > 0).all(|r| r.pinned), "every member pinned");
    assert_eq!(rows[0].id, 2, "pinned members lead the list");
}

#[test]
fn nothing_happens_without_the_room() {
    let mut m = model(1);
    m.apply(Action::SetPaneArea { width: 300.0, height: 200.0 });
    let events = m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    assert!(events.contains(&Event::Refused(Refusal::TooSmall)));
    assert_eq!(m.ids(), [1], "no terminal was made for a split that did not happen");
    m.apply(Action::SetPaneArea { width: 1200.0, height: 800.0 });
    assert!(!refused(&m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row })));
}

#[test]
fn a_row_dropped_on_another_joins_it_and_leaving_dissolves_a_split_of_two() {
    let mut m = model(3);
    let events = m.apply(Action::JoinSplit { id: 3, target: 1, zone: Zone::Right });
    assert!(events.contains(&Event::Layout));
    assert_eq!(m.selected(), Some(3));
    m.apply(Action::LeaveSplit(3));
    let rows = &m.sidebar_snapshot(0).sections[0].rows;
    assert!(rows.iter().all(|r| r.split.members == 0), "one left is no split");
    assert_eq!(m.panes_snapshot(900.0, 500.0).split, None);
    assert!(refused(&m.apply(Action::LeaveSplit(3))), "nothing to leave");
}

#[test]
fn selecting_a_row_of_a_split_focuses_its_pane_and_zoom_fills_the_area() {
    let mut m = model(1);
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    m.apply(Action::Select(1));
    assert_eq!(m.panes_snapshot(800.0, 500.0).focused, Some(1));
    m.apply(Action::ToggleZoom);
    let zoomed = m.panes_snapshot(800.0, 500.0);
    assert_eq!(zoomed.zoomed, Some(1));
    assert_eq!(zoomed.panes.len(), 1);
    assert_eq!((zoomed.panes[0].w, zoomed.panes[0].h), (800.0, 500.0));
    m.apply(Action::ToggleZoom);
    assert_eq!(m.panes_snapshot(800.0, 500.0).zoomed, None);
}

#[test]
fn a_terminal_alone_shows_alone_and_cannot_zoom() {
    let mut m = model(2);
    let panes = m.panes_snapshot(800.0, 500.0);
    assert_eq!((panes.split, panes.panes.len()), (None, 1));
    assert!(refused(&m.apply(Action::ToggleZoom)));
    // facts are untouched by any of it
    assert_eq!(m.facts(1), Some(&TerminalFacts::default()));
}

#[test]
fn a_pane_moves_inside_its_split_and_the_focus_follows_the_arrows() {
    let mut m = model(3);
    m.apply(Action::SetPaneArea { width: 2000.0, height: 900.0 });
    m.apply(Action::JoinSplit { id: 2, target: 1, zone: Zone::Right });
    m.apply(Action::JoinSplit { id: 3, target: 2, zone: Zone::Bottom });
    // 1 on the left, 2 above 3 on the right; 3 has the keyboard.
    assert_eq!(m.pane_drop_refusal(3, 1, Zone::Left), None, "a member moves");
    m.apply(Action::FocusDirection(Direction::Up));
    assert_eq!(m.selected(), Some(2));
    m.apply(Action::FocusDirection(Direction::Left));
    assert_eq!(m.selected(), Some(1));
    assert!(refused(&m.apply(Action::FocusDirection(Direction::Left))), "the area's edge");
    let events = m.apply(Action::MovePane { id: 3, target: 1, zone: Zone::Left });
    assert!(events.contains(&Event::Layout));
    let panes = m.panes_snapshot(2000.0, 900.0);
    assert_eq!(panes.panes[0].id, 3, "3 is now first in reading order, to the left of 1");
    assert_eq!(m.selected(), Some(3));
    assert!(refused(&m.apply(Action::MovePane { id: 3, target: 99, zone: Zone::Left })));
}

#[test]
fn a_terminal_that_is_not_in_the_split_is_checked_as_a_join() {
    let mut m = model(3);
    m.apply(Action::SetPaneArea { width: 300.0, height: 900.0 });
    m.apply(Action::JoinSplit { id: 2, target: 1, zone: Zone::Right });
    assert_eq!(m.pane_drop_refusal(3, 1, Zone::Right), Some(Refusal::TooSmall), "no room for a third column");
    assert_eq!(m.pane_drop_refusal(3, 99, Zone::Right), Some(Refusal::Unknown));
}
