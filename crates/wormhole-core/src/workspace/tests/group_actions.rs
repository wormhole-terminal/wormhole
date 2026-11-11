//! What a group's menu and a row's menu do beyond rearranging the list: colour, folder, a new
//! terminal in a group, closing it, privacy, "Show beside", a drop's plan applied, a group on
//! its way to a window of its own.

use std::path::PathBuf;

use crate::view::panes::Axis;
use crate::view::sidebar::{GroupColor, Item, Landing, Part};
use crate::workspace::{Action, CardRef, Event, Over};

use super::{listed, model, refused};

#[test]
fn a_groups_colour_is_one_of_the_nine_names_or_one_of_its_own() {
    let mut m = model(2);
    m.apply(Action::NewGroup { members: vec![1] });
    let colour = |m: &crate::workspace::WorkspaceModel| {
        m.sidebar_snapshot(0).sections[0].head.clone().map(|h| h.color)
    };
    assert_eq!(colour(&m).as_deref(), Some("blue"), "the first colour in turn");
    m.apply(Action::SetGroupColor { group: 1, color: Some(GroupColor::Pink) });
    assert_eq!(colour(&m).as_deref(), Some("pink"));
    m.apply(Action::SetGroupCustomColor { group: 1, r: 0x33, g: 0x99, b: 0xcc });
    assert_eq!(colour(&m).as_deref(), Some("#3399cc"));
    m.apply(Action::SetGroupColor { group: 1, color: None });
    assert_eq!(colour(&m).as_deref(), Some(""));
    assert!(refused(&m.apply(Action::SetGroupColor { group: 9, color: None })));
}

#[test]
fn a_folder_pins_its_group_and_clearing_it_leaves_it_pinned() {
    let mut m = model(2);
    m.apply(Action::NewGroup { members: vec![2] });
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/Users/me/app".into()) });
    let head = m.sidebar_snapshot(0).sections[1].head.clone().expect("head");
    assert_eq!(head.folder_path.as_deref(), Some("/Users/me/app"));
    assert!(head.pinned, "a group with a folder is a place: it stays");
    m.apply(Action::SetGroupFolder { group: 1, folder: None });
    let head = m.sidebar_snapshot(0).sections[1].head.clone().expect("head");
    assert!((head.folder_path.is_none(), head.pinned) == (true, true));
}

#[test]
fn a_new_terminal_in_a_group_lands_last_in_it_and_starts_in_its_folder() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![2] });
    // Without a folder that is a directory the terminal starts where the selected one is.
    m.update_facts(1, |f| f.cwd = Some(PathBuf::from("/tmp")));
    let events = m.apply(Action::NewTerminalInGroup(1));
    assert!(events.contains(&Event::Opened { id: 4, cwd: Some("/tmp".into()) }));
    assert_eq!(m.selected(), Some(4));
    assert_eq!(
        listed(&m),
        ["1:1", "[Group 1]", "2:2", "4:3", "3:4"],
        "the group stands where its first terminal did"
    );
    // With a folder that exists, it starts there.
    m.apply(Action::SetGroupFolder { group: 1, folder: Some("/".into()) });
    let events = m.apply(Action::NewTerminalInGroup(1));
    assert!(events.contains(&Event::Opened { id: 5, cwd: Some("/".into()) }));
    assert!(refused(&m.apply(Action::NewTerminalInGroup(7))));
}

#[test]
fn a_new_terminal_in_a_new_group_makes_the_group_with_it_and_selects_it() {
    let mut m = model(1);
    m.apply(Action::NewTerminalInNewGroup);
    assert_eq!(m.selected(), Some(2));
    assert_eq!(listed(&m), ["1:1", "[Group 1]", "2:2"]);
}

#[test]
fn closing_a_group_ends_its_terminals_and_the_group_unless_it_is_pinned() {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    let events = m.apply(Action::CloseGroup(1));
    assert!(events.contains(&Event::Closed { id: 2 }) && events.contains(&Event::Closed { id: 3 }));
    assert_eq!(listed(&m), ["1:1", "4:2"]);
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    m.apply(Action::SetGroupPinned { group: 1, pinned: true });
    m.apply(Action::CloseGroup(1));
    assert_eq!(listed(&m), ["1:1", "[Group 1]"], "a pinned group stays, as one line");
}

#[test]
fn closing_the_only_group_of_the_window_starts_a_terminal() {
    let mut m = model(2);
    m.apply(Action::NewGroup { members: vec![1, 2] });
    let events = m.apply(Action::CloseGroup(1));
    assert!(events.iter().any(|e| matches!(e, Event::Opened { .. })), "a window is never left empty");
    assert_eq!(m.ids().len(), 1);
}

#[test]
fn privacy_choices_belong_to_the_terminal_and_forgetting_asks_to_erase_what_was_saved() {
    let mut m = model(2);
    assert!(m.apply(Action::ToggleAgentHidden(2)).contains(&Event::Listing));
    assert!(m.apply(Action::SetAgentHidden { id: 2, hidden: true }).is_empty(), "already so");
    let events = m.apply(Action::SetForgetText { id: 1, forget: true });
    assert!(events.contains(&Event::SavedTextForgotten { id: 1 }));
    let rows = m.sidebar_snapshot(0).sections.into_iter().flat_map(|s| s.rows).collect::<Vec<_>>();
    assert_eq!((rows[0].forget_text, rows[0].agent_hidden), (true, false));
    assert_eq!((rows[1].forget_text, rows[1].agent_hidden), (false, true));
    let all = m.terminals_snapshot();
    assert!(all.terminals[1].agent_hidden && all.terminals[0].forget_text);
    let events = m.apply(Action::ToggleForgetText(1));
    assert!(!events.contains(&Event::SavedTextForgotten { id: 1 }), "turning it off erases nothing");
    assert!(refused(&m.apply(Action::ToggleAgentHidden(42))));
}

#[test]
fn show_beside_makes_the_two_a_split_with_the_other_on_the_right() {
    let mut m = model(3);
    m.apply(Action::SetPaneArea { width: 2000.0, height: 900.0 });
    m.apply(Action::ShowBeside { id: 1, other: 3 });
    let panes = m.panes_snapshot(2000.0, 900.0);
    assert_eq!(panes.panes.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 3]);
    assert!(panes.panes[1].x > panes.panes[0].x, "to the right of its pane");
    assert_eq!(listed(&m), ["1:1", "3:2", "2:3"], "a split's members sit together");
}

#[test]
fn a_drop_plan_applied_as_given_leaves_the_split_and_takes_its_mates_as_the_flags_say() {
    let mut m = model(4);
    m.apply(Action::SetPaneArea { width: 2000.0, height: 900.0 });
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    // 1 and 5 are a split; move 1 alone out, to the end of the list.
    m.apply(Action::MoveTerminalAs {
        id: 1,
        landing: Landing::Top { before: None },
        leave_split: true,
        with_mates: false,
    });
    assert_eq!(
        m.sidebar_snapshot(0).sections.iter().flat_map(|s| &s.rows).filter(|r| r.split.id.is_some()).count(),
        0
    );
    assert_eq!(*listed(&m).last().expect("rows"), "1:5");
    assert!(refused(&m.apply(Action::MoveTerminalAs {
        id: 42,
        landing: Landing::Top { before: None },
        leave_split: false,
        with_mates: false
    })));
}

#[test]
fn a_group_dragged_has_a_plan_and_lands_where_it_says() {
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![1] });
    let over = Over::After(CardRef::Loose(3));
    let plan = m.group_drop_plan(1, over).expect("a group may go to the end");
    assert_eq!(plan, Landing::Top { before: None });
    m.apply(Action::DropGroup { group: 1, over });
    assert_eq!(listed(&m), ["2:1", "3:2", "[Group 1]", "1:3"]);
    assert_eq!(m.group_drop_plan(1, over), None, "already last");
    let over_head = Over::Head { group: 1, part: Part::Top };
    assert_eq!(m.group_drop_plan(1, over_head), None, "not onto itself");
    let _ = Item::Group(1);
}

#[test]
fn moving_a_group_to_a_window_names_its_terminals_and_their_splits() {
    let mut m = model(4);
    m.apply(Action::SetPaneArea { width: 2000.0, height: 900.0 });
    m.apply(Action::SplitBeside { anchor: 2, axis: Axis::Row });
    m.apply(Action::NewGroup { members: vec![2] });
    let events = m.apply(Action::MoveGroupToWindow(1));
    assert_eq!(events, [Event::GroupToWindow { group: 1, terminals: vec![2, 5] }]);
    assert!(refused(&m.apply(Action::MoveGroupToWindow(9))));
}

#[test]
fn the_numbers_mode_can_change_while_the_window_runs() {
    let mut m = model(3);
    m.apply(Action::TogglePin(3));
    assert_eq!(listed(&m), ["3:1", "1:2", "2:3"], "the list's numbers follow the list");
    m.apply(Action::SetNumbersFollowList(false));
    assert!(!m.chrome_snapshot().numbers_follow_list);
    m.apply(Action::NewTerminal);
    m.apply(Action::TogglePin(4));
    let number_of = |m: &crate::workspace::WorkspaceModel, id: i64| {
        m.terminals_snapshot().terminals.iter().find(|t| t.id == id).and_then(|t| t.number)
    };
    assert_eq!((number_of(&m, 3), number_of(&m, 4)), (Some(1), Some(2)), "pinned: dealt again, top first");
    m.apply(Action::SetNumbersFollowList(true));
    assert!(m.chrome_snapshot().numbers_follow_list);
}
