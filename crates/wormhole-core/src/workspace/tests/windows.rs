//! A window saved and made again, ids shared by several windows, terminals moving between windows.

use crate::storage::store::{Store, WindowRecord};
use crate::view::panes::Axis;
use crate::workspace::{Action, Event, IdCounter, ModelConfig, WorkspaceModel};

use super::model;

fn opened(events: &[Event]) -> Vec<i64> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { id, .. } => Some(*id),
            _ => None,
        })
        .collect()
}

fn shared(counter: &IdCounter) -> WorkspaceModel {
    WorkspaceModel::new(ModelConfig { ids: Some(counter.clone()), ..ModelConfig::default() })
}

#[test]
fn windows_sharing_a_counter_never_give_the_same_id_twice() {
    let ids = IdCounter::starting_at(10);
    let (mut a, mut b) = (shared(&ids), shared(&ids));
    a.apply(Action::NewTerminal);
    b.apply(Action::NewTerminal);
    a.apply(Action::NewTerminal);
    assert_eq!((a.ids(), b.ids()), (vec![10, 12], vec![11]));
}

#[test]
fn a_saved_window_comes_back_with_its_terminals_names_pins_groups_split_and_selection() {
    let mut m = model(4);
    m.apply(Action::Rename { id: 2, name: "api".into() });
    m.apply(Action::SetPinned { id: 3, pinned: true });
    m.apply(Action::NewGroup { members: vec![1, 2] });
    m.apply(Action::SplitBeside { anchor: 4, axis: Axis::Row });
    m.apply(Action::ToggleSidebar);
    m.apply(Action::Select(2));
    m.update_facts(2, |facts| facts.cwd = Some("/work/api".into()));
    let before = super::listed(&m);
    let record = m.window_record(7, Some("{\"frame\":1}".into()));
    assert_eq!((record.id, record.sessions.len(), record.splits.len()), (7, 5, 1));

    let mut back = WorkspaceModel::new(ModelConfig::default());
    let events = back.restore(&record);
    assert_eq!(opened(&events), [1, 2, 3, 4, 5]);
    assert!(events.contains(&Event::Opened { id: 2, cwd: Some("/work/api".into()) }));
    assert_eq!(back.selected(), Some(2));
    assert_eq!(super::listed(&back), before);
    assert!(!back.chrome_snapshot().sidebar_visible);
    // New ids start above the restored ones (the split took 6), and a second restore changes nothing.
    assert!(back.restore(&record).is_empty());
    back.apply(Action::NewTerminal);
    assert_eq!(back.ids().last(), Some(&7));
}

#[test]
fn a_window_goes_through_the_store_and_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut store = Store::open_at(dir.path()).expect("opens");
    let mut m = model(3);
    m.apply(Action::NewGroup { members: vec![2, 3] });
    store.save_window(&m.window_record(5, None)).expect("saves");
    let loaded = store.load().expect("loads");
    let mut back = WorkspaceModel::new(ModelConfig::default());
    back.restore(&loaded[0]);
    assert_eq!(super::listed(&back), super::listed(&m));
    assert_eq!(store.next_ids().expect("ids").1, 6);
}

#[test]
fn a_record_with_garbage_layout_or_no_terminals_restores_what_it_can() {
    let record = WindowRecord { id: 1, layout: Some("nonsense".into()), ..Default::default() };
    let mut m = WorkspaceModel::new(ModelConfig::default());
    assert!(m.restore(&record).is_empty());
    let mut source = model(2).window_record(1, None);
    source.layout = Some("nonsense".into());
    source.active = 99;
    let events = m.restore(&source);
    assert_eq!(opened(&events), [1, 2]);
    assert!(m.selected().is_some());
}

#[test]
fn taking_terminals_out_writes_them_down_closes_them_and_another_window_takes_them_in() {
    let ids = IdCounter::starting_at(1);
    let (mut a, mut b) = (shared(&ids), shared(&ids));
    for _ in 0..4 {
        a.apply(Action::NewTerminal);
    }
    a.apply(Action::Rename { id: 3, name: "build".into() });
    a.apply(Action::Select(3));
    let (departing, events) = a.take_out(&[3, 4]).expect("some of them are here");
    assert_eq!(departing.ids(), [3, 4]);
    assert_eq!((departing.active, departing.records[0].name.as_deref()), (0, Some("build")));
    assert!(events.contains(&Event::Closed { id: 3 }) && events.contains(&Event::Closed { id: 4 }));
    assert_eq!(a.ids(), [1, 2]);
    assert!(a.selected().is_some());

    let events = b.receive(&departing);
    assert!(events.contains(&Event::Adopted { id: 3 }) && events.contains(&Event::Adopted { id: 4 }));
    assert!(opened(&events).is_empty());
    assert_eq!((b.ids(), b.selected()), (vec![3, 4], Some(3)));
    assert!(a.take_out(&[3]).is_none());
}

#[test]
fn a_split_moves_whole_or_not_at_all() {
    let mut m = model(2);
    m.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    let split_members = m.ids();
    let (whole, _) = m.take_out(&split_members).expect("all of them");
    assert_eq!(whole.splits.len(), 1);
    let members = whole.records.iter().filter(|r| r.split_id == Some(whole.splits[0].id)).count();
    assert_eq!(members, 2);
    let mut n = WorkspaceModel::new(ModelConfig::default());
    n.receive(&whole);
    assert_eq!(n.window_record(1, None).splits.len(), 1);

    let mut half = model(1);
    half.apply(Action::SplitBeside { anchor: 1, axis: Axis::Row });
    let (part, _) = half.take_out(&[1]).expect("one of two");
    assert!(part.splits.is_empty() && part.records[0].split_id.is_none());
    assert_eq!(half.window_record(1, None).splits.len(), 0);
}

#[test]
fn taking_out_the_last_terminal_starts_a_new_one_as_closing_it_does() {
    let mut m = model(1);
    let (_, events) = m.take_out(&[1]).expect("it is here");
    assert_eq!(opened(&events).len(), 1);
    assert_eq!(m.ids().len(), 1);
}
