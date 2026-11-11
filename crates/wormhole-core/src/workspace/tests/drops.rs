//! What a drop on the list does: core's `place` rules by id, and the plan a shell asks for first.

use crate::view::panes::Axis;
use crate::view::sidebar::{Item, Landing, Part};
use crate::workspace::{Action, CardRef, DropPlan, Event, Over};

use super::{listed, model, refused};

#[test]
fn dropping_on_the_bottom_of_a_row_puts_the_terminal_after_it() {
    let mut m = model(3);
    let plan = m.drop_plan(1, Over::Row { id: 2, part: Part::Bottom });
    assert_eq!(
        plan,
        Some(DropPlan::Move {
            landing: Landing::Top { before: Some(Item::Terminal(3)) },
            leave_split: false,
            with_mates: false
        })
    );
    let events = m.apply(Action::DropTerminal { dragged: 1, over: Over::Row { id: 2, part: Part::Bottom } });
    assert!(events.contains(&Event::Moved { id: 1 }));
    assert_eq!(listed(&m), ["2:1", "1:2", "3:3"]);
}

#[test]
fn dropping_under_the_list_puts_it_last_and_on_itself_changes_nothing() {
    let mut m = model(3);
    m.apply(Action::DropTerminal { dragged: 1, over: Over::Tail });
    assert_eq!(listed(&m), ["2:1", "3:2", "1:3"]);
    assert_eq!(m.drop_plan(1, Over::Tail), None, "already last");
    assert!(refused(&m.apply(Action::DropTerminal { dragged: 1, over: Over::Tail })));
}

#[test]
fn dropping_on_a_group_head_joins_the_group_and_the_top_quarter_stands_before_it() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::DropTerminal { dragged: 3, over: Over::Head { group: 1, part: Part::Middle } });
    assert_eq!(listed(&m), ["[Group 1]", "3:1", "1:2", "2:3"], "an open group takes it at the top");
    m.apply(Action::DropTerminal { dragged: 3, over: Over::Head { group: 1, part: Part::Top } });
    assert_eq!(listed(&m), ["3:1", "[Group 1]", "1:2", "2:3"]);
}

#[test]
fn a_group_dropped_over_a_row_stands_before_or_after_it() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::DropGroup { group: 1, over: Over::Row { id: 3, part: Part::Bottom } });
    assert_eq!(listed(&m), ["2:1", "3:2", "[Group 1]", "1:3"]);
    m.apply(Action::DropGroup { group: 1, over: Over::After(CardRef::Loose(2)) });
    assert_eq!(listed(&m), ["2:1", "[Group 1]", "1:2", "3:3"]);
}

#[test]
fn the_middle_of_a_row_joins_a_split_and_dragging_a_member_away_leaves_it() {
    let mut m = model(3);
    let events = m.apply(Action::DropTerminal { dragged: 2, over: Over::Row { id: 1, part: Part::Middle } });
    assert!(events.contains(&Event::Layout));
    let rows = &m.sidebar_snapshot(0).sections[0].rows;
    assert_eq!(rows.iter().map(|r| r.split.members).collect::<Vec<_>>(), [2, 2, 0]);
    assert_eq!(m.selected(), Some(2), "the joiner takes the selection");
    m.apply(Action::DropTerminal { dragged: 1, over: Over::Row { id: 3, part: Part::Bottom } });
    let snapshot = m.sidebar_snapshot(0);
    let rows: Vec<_> = snapshot.sections.iter().flat_map(|s| &s.rows).collect();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), [2, 3, 1]);
    assert!(rows.iter().all(|r| r.split.members == 0), "a row dragged away from its mates leaves the split");
}

#[test]
fn leaving_a_split_by_a_drop_below_the_list() {
    let mut m = model(3);
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    let split_mate = m.selected().expect("selected");
    m.apply(Action::DropTerminal { dragged: split_mate, over: Over::Tail });
    let snapshot = m.sidebar_snapshot(0);
    assert!(snapshot.sections.iter().flat_map(|s| &s.rows).all(|r| r.split.members == 0));
    assert_eq!(m.panes_snapshot(800.0, 600.0).panes.len(), 1);
}

#[test]
fn a_drop_on_a_terminal_that_is_not_here_is_refused() {
    let mut m = model(2);
    assert!(refused(&m.apply(Action::DropTerminal { dragged: 9, over: Over::Tail })));
    assert!(refused(
        &m.apply(Action::DropTerminal { dragged: 1, over: Over::Row { id: 9, part: Part::Top } })
    ));
}
