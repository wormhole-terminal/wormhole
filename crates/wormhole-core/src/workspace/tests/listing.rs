//! The list's order, pins, folds and numbers: core's `view::sidebar::listing` tests, through the model.

use crate::view::sidebar::SidebarNumbers;
use crate::workspace::{Action, Event, ModelConfig, WorkspaceModel};

use super::{listed, model};

#[test]
fn terminals_in_no_group_and_groups_stand_in_the_order_given() {
    let mut m = model(5);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    m.apply(Action::RenameGroup { group: 1, name: "Docs".into() });
    assert_eq!(listed(&m), ["1:1", "[Docs]", "2:2", "3:3", "4:4", "5:5"]);
    let snapshot = m.sidebar_snapshot(0);
    let first = &snapshot.sections[0].rows[0];
    assert_eq!((first.position, first.of), (1, 4), "1 of 4 at the top level: 3 terminals and a group");
    assert_eq!(snapshot.sections[1].head.as_ref().map(|h| (h.position, h.of)), Some((2, 4)));
}

#[test]
fn a_folded_group_is_its_head_only_and_takes_no_number() {
    let mut m = model(12);
    m.apply(Action::NewGroup { members: vec![1, 2] });
    let open = m.sidebar_snapshot(0);
    assert_eq!(open.sections[0].rows.len(), 2);
    m.apply(Action::ToggleGroup(1));
    let folded = m.sidebar_snapshot(0);
    assert_eq!(folded.sections[0].terminal_count, 2, "the head still counts what is folded away");
    assert!(folded.sections[0].rows.is_empty());
    let numbers: Vec<_> = folded.sections.iter().flat_map(|s| &s.rows).filter_map(|r| r.number).collect();
    assert_eq!(numbers, (1..=9).collect::<Vec<_>>());
    assert_eq!(listed(&m)[..3], ["[Group 1]+", "3:1", "4:2"], "numbers go to what you can see");
}

#[test]
fn pinned_terminals_lead_the_list_and_their_group() {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![1, 2, 3] });
    m.apply(Action::RenameGroup { group: 1, name: "Docs".into() });
    m.apply(Action::TogglePin(3));
    assert_eq!(listed(&m), ["[Docs]", "3:1", "1:2", "2:3", "4:4"]);
    m.apply(Action::TogglePin(3));
    m.apply(Action::TogglePin(4));
    assert_eq!(listed(&m), ["4:1", "[Docs]", "1:2", "2:3", "3:4"], "a loose pin goes above the groups");
}

#[test]
fn a_pinned_empty_group_is_a_head_with_nothing_under_it() {
    let mut m = model(2);
    m.apply(Action::NewGroup { members: vec![1] });
    m.apply(Action::SetGroupPinned { group: 1, pinned: true });
    m.apply(Action::RemoveFromGroup(1));
    let snapshot = m.sidebar_snapshot(0);
    let head = snapshot.sections.iter().find_map(|s| s.head.as_ref()).expect("the pinned group stays");
    assert!(head.pinned);
    assert_eq!(listed(&m), ["[Group 1]", "1:1", "2:2"], "the empty group keeps its place");
}

#[test]
fn stepping_goes_through_what_is_visible_and_wraps() {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    m.apply(Action::ToggleGroup(1));
    m.apply(Action::SelectStep(1));
    assert_eq!(m.selected(), Some(4), "the folded rows are skipped");
    m.apply(Action::SelectStep(1));
    assert_eq!(m.selected(), Some(1), "wrapping round");
    m.apply(Action::SelectStep(-1));
    assert_eq!(m.selected(), Some(4));
}

#[test]
fn a_number_selects_its_terminal_and_a_number_past_them_the_last() {
    let mut m = model(3);
    m.apply(Action::SelectNumber(2));
    assert_eq!(m.selected(), Some(2));
    m.apply(Action::SelectNumber(9));
    assert_eq!(m.selected(), Some(3), "⌘9 is the last, as in a browser");
}

#[test]
fn numbers_that_stay_with_the_terminal_survive_a_new_terminal_and_a_fold() {
    let mut m = WorkspaceModel::new(ModelConfig { numbers: SidebarNumbers::Terminal, ..Default::default() });
    for _ in 0..4 {
        m.apply(Action::NewTerminal);
    }
    assert_eq!(listed(&m), ["1:1", "2:2", "3:3", "4:4"]);
    m.apply(Action::TogglePin(3));
    assert_eq!(listed(&m), ["3:1", "1:2", "2:3", "4:4"], "dealt again when the person pins");
    m.apply(Action::NewTerminal);
    assert_eq!(listed(&m).last().map(String::as_str), Some("5:5"), "a new terminal takes the next number");
}

#[test]
fn the_snapshot_carries_a_revision_that_moves_with_the_list() {
    let mut m = model(2);
    let before = m.sidebar_snapshot(0).revision;
    assert_eq!(m.sidebar_snapshot(0).revision, before, "pulling changes nothing");
    let events = m.apply(Action::TogglePin(2));
    assert!(events.contains(&Event::Listing));
    assert!(m.sidebar_snapshot(0).revision > before);
}
