//! Groups: made, named, folded, joined and taken apart.

use crate::view::sidebar::{Item, Landing};
use crate::workspace::{Action, Event};

use super::{listed, model, refused};

#[test]
fn a_new_group_stands_where_its_first_terminal_stood_and_is_named_in_turn() {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![3] });
    m.apply(Action::NewGroup { members: vec![4] });
    let names: Vec<_> = m.sidebar_snapshot(0).sections.iter().filter_map(|s| s.head.clone()).collect();
    assert_eq!(names.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(), ["Group 1", "Group 2"]);
    assert_eq!(listed(&m), ["1:1", "2:2", "[Group 1]", "3:3", "[Group 2]", "4:4"]);
}

#[test]
fn a_group_of_nothing_that_exists_is_refused() {
    let mut m = model(2);
    assert!(refused(&m.apply(Action::NewGroup { members: vec![] })));
    assert!(refused(&m.apply(Action::NewGroup { members: vec![42] })));
}

#[test]
fn a_blank_name_keeps_the_old_one() {
    let mut m = model(2);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::RenameGroup { group: 1, name: "  Release  ".into() });
    m.apply(Action::RenameGroup { group: 1, name: "   ".into() });
    assert_eq!(listed(&m)[0], "[Release]");
}

#[test]
fn ungrouping_leaves_the_terminals_where_the_group_stood() {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    m.apply(Action::Ungroup(1));
    assert_eq!(listed(&m), ["1:1", "2:2", "3:3", "4:4"]);
}

#[test]
fn adding_a_terminal_to_a_group_puts_it_last_and_an_empty_group_is_gone_with_it() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::NewGroup { members: vec![2] });
    let events = m.apply(Action::AddToGroup { id: 3, group: 1 });
    assert!(events.contains(&Event::Moved { id: 3 }));
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "[Group 2]", "2:3"]);
    m.apply(Action::AddToGroup { id: 2, group: 1 });
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "2:3"], "Group 2 lost its last terminal");
}

#[test]
fn a_terminal_moves_to_a_landing_and_a_group_among_the_top_level() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![3] });
    m.apply(Action::MoveGroup { group: 1, landing: Landing::Top { before: Some(Item::Terminal(1)) } });
    assert_eq!(listed(&m), ["[Group 1]", "3:1", "1:2", "2:3"]);
    m.apply(Action::MoveTerminal { id: 1, landing: Landing::Group { group: 1, before: Some(3) } });
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "3:2", "2:3"]);
}

#[test]
fn a_group_that_does_not_exist_is_refused() {
    let mut m = model(2);
    assert!(refused(&m.apply(Action::ToggleGroup(7))));
    assert!(refused(&m.apply(Action::AddToGroup { id: 1, group: 7 })));
}
