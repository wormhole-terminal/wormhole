//! What a folded group still tells: its members' tiles and alert dot, and the list of every
//! terminal for the palette and the status bar.

use crate::view::sidebar::{Status, Urgency};
use crate::workspace::{Action, TileGlyph};

use super::{listed, model};

/// A model whose terminals 2 and 3 are in a folded group, 2 asking, 3 working.
fn folded() -> crate::workspace::WorkspaceModel {
    let mut m = model(4);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    m.update_facts(2, |f| {
        f.agent = Some(crate::agents::AgentId::Claude);
        f.status = Status::Busy;
    });
    m.update_facts(3, |f| {
        f.agent = Some(crate::agents::AgentId::Claude);
        f.status = Status::Attention;
    });
    m.apply(Action::ToggleGroup(1));
    m
}

#[test]
fn a_folded_head_lists_no_rows_but_its_members_tiles_loudest_first() {
    let m = folded();
    assert_eq!(listed(&m), ["1:1", "[Group 1]+", "4:2"]);
    let snapshot = m.sidebar_snapshot(0);
    let head = &snapshot.sections[1];
    assert!(head.rows.is_empty() && head.head.as_ref().is_some_and(|h| h.folded));
    let tiles: Vec<_> = head.tiles.iter().map(|t| (t.id, t.glyph, t.urgency)).collect();
    assert_eq!(
        tiles,
        [(3, TileGlyph::NeedsYou, Urgency::NeedsYou), (2, TileGlyph::Working, Urgency::Working)],
        "the one that asks first, the one that works after it"
    );
    assert_eq!(head.alert, Some(Urgency::NeedsYou));
    assert_eq!((head.terminal_count, head.alert_count, head.working_count), (2, 1, 1));
    assert!(head.tiles[0].agent.is_some(), "an agent's tile carries its logo's name");
}

#[test]
fn an_open_head_has_no_tiles_and_an_idle_group_no_alert() {
    let mut m = folded();
    m.apply(Action::ToggleGroup(1));
    let head = m.sidebar_snapshot(0).sections.remove(1);
    assert_eq!(head.rows.len(), 2);
    assert!(head.tiles.is_empty(), "the rows are the tiles' place while it is open");
    assert_eq!(head.alert, Some(Urgency::NeedsYou), "the dot is the head's own, folded or not");
    m.update_facts(3, |f| f.status = Status::Idle);
    assert_eq!(m.sidebar_snapshot(0).sections[1].alert, None);
}

#[test]
fn every_terminal_is_listed_with_its_group_and_whether_it_is_folded_away() {
    let m = folded();
    let all = m.terminals_snapshot();
    assert_eq!(all.terminals.iter().map(|t| t.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    let by = |id: i64| all.terminals.iter().find(|t| t.id == id).cloned().expect("listed");
    assert_eq!((by(2).group, by(2).folded, by(2).number), (Some(1), true, None));
    assert_eq!(by(3).group_name.as_deref(), Some("Group 1"));
    assert_eq!((by(4).group, by(4).folded, by(4).number), (None, false, Some(2)));
    assert!(by(1).selected);
    assert_eq!((all.counts.needs_you, all.counts.working, all.counts.quiet), (1, 1, 2));
}

#[test]
fn the_terminals_revision_moves_with_a_fact_and_not_with_a_pull() {
    let mut m = folded();
    let before = m.terminals_snapshot().revision;
    assert_eq!(m.terminals_snapshot().revision, before);
    m.update_facts(4, |f| f.branch = Some("main".into()));
    let after = m.terminals_snapshot();
    assert!(after.revision > before);
    assert_eq!(after.terminals[3].branch.as_deref(), Some("main"));
    assert_eq!(m.revisions().terminals, after.revision);
}
