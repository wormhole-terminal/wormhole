//! The feature table's contract: the app's table agrees with core's, a window gets the table the app
//! installed, and the hooks' shapes work end to end on a stand-in feature.

use std::collections::HashSet;

use gpui_kit::assets::IconName;
use gpui_kit::{
    AnyElement, AnyEntity, AppContext as _, Entity, EventEmitter, IntoElement as _, Keystroke, Styled as _,
    TestAppContext,
};
use wormhole_core::config::{FEATURES, FeatureId, TabId};
use wormhole_core::git::{ChangeKind, FileChange, RepoStatus};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::focus::Area;
use wormhole_core::view::spoken::{self, PanelTab};

use crate::actions::ShowChanges;
use crate::window::Workspace;
use crate::window::feature::{FeatureDef, FeatureEvent, FeatureState, Stake, StateDef, TabDef};
use crate::window::features::TABLE;
use crate::window::features::git::changes;
use crate::window::focus::Activate;
use crate::window::sidebar::{RowExtra, RowExtras, RowInfo};
use crate::window::view_tests::{draw, drive, isolate_in, read, start, start_with_table, with_window};

/// Features core lists that have not moved into the app's table yet. Each phase 5 step takes one off
/// this list when it adds the entry; the list ends empty.
const NOT_YET: &[FeatureId] =
    &[FeatureId::History, FeatureId::Notes, FeatureId::Usage, FeatureId::Project, FeatureId::Worktrees];

#[test]
fn every_feature_is_in_the_table_or_waiting_for_its_step() {
    let in_table: HashSet<FeatureId> = TABLE.iter().map(|def| def.id).collect();
    assert_eq!(in_table.len(), TABLE.len(), "a feature has two entries");
    for info in &FEATURES {
        let waiting = NOT_YET.contains(&info.id);
        assert!(
            in_table.contains(&info.id) != waiting,
            "{}: must be in the table or in NOT_YET, not both and not neither",
            info.key
        );
    }
    assert_eq!(NOT_YET.iter().collect::<HashSet<_>>().len(), NOT_YET.len());
}

#[test]
fn an_entry_brings_the_tabs_core_says_in_core_order() {
    let order = |id: FeatureId| FEATURES.iter().position(|info| info.id == id);
    assert!(TABLE.windows(2).all(|pair| order(pair[0].id) < order(pair[1].id)), "tab order is core's");
    for def in TABLE {
        let tabs: Vec<TabId> = def.tabs.iter().map(|tab| tab.id).collect();
        assert_eq!(tabs, def.id.info().tabs, "{}", def.id.info().key);
    }
}

/// What a stand-in feature keeps: a number the tab counts, what the window told it and what it said.
struct Marker {
    count: i64,
    selected: Vec<i64>,
    focused_for: Vec<i64>,
}

impl EventEmitter<FeatureEvent> for Marker {}

fn marker(entity: &AnyEntity) -> Entity<Marker> {
    entity.clone().downcast::<Marker>().expect("the stand-in's entity")
}

/// A feature of its own in every slot, so the shapes are type-checked and exercised. It stands in for
/// Usage, which is always on, so what the window asks of it does not depend on a flag.
static STAND_IN: &[FeatureDef] = &[
    changes::DEF,
    FeatureDef {
        id: FeatureId::Usage,
        tabs: &[TabDef {
            id: TabId::Usage,
            icon: IconName::PanelRight,
            label: "Stand-in",
            action: ActionId::ShowUsage,
            on_show: None,
            shown: |facts, _| facts.active().is_some(),
            count: Some(|entity, facts, cx| {
                Some(marker(entity).read(cx).count as usize + facts.on_screen().count())
            }),
            spoken: |_, facts, _| format!("{:?} tab", facts.tab),
            view: |entity, facts, _, cx| -> AnyElement {
                // The facts are alive while `cx` is lent mutably: the point of holding no `&App`.
                marker(entity).update(cx, |marker, _| marker.count += 1);
                let width = f32::from(facts.panel_width);
                gpui_kit::div().w(gpui_kit::px(width)).into_any_element()
            },
            walk: None,
            actions: Some(|entity, facts, _, cx| {
                let on = facts.agent_on_screen(cx);
                marker(entity).update(cx, |marker, _| marker.count += 10);
                on.then(|| gpui_kit::div().into_any_element())
            }),
        }],
        bind: Some(|_, _, root, _| root),
        row_extras: Some(|_, _, extras, _| {
            for (id, slot) in extras.iter_mut() {
                *slot = Some(RowExtra { count: id as usize });
            }
        }),
        close_stake: Some(|_, sessions, _| {
            Some(Stake { count: sessions.len(), sentence: format!("{} stand-ins", sessions.len()) })
        }),
        menu: Some(|_, _, state, _| state.disabled.push(ActionId::NewNote)),
        state: Some(StateDef {
            make: |window, cx| {
                let entity = cx.new(|_| Marker { count: 41, selected: Vec::new(), focused_for: Vec::new() });
                let _events = Workspace::listen(&entity, window, cx);
                FeatureState { entity: entity.into(), _events }
            },
            selected: Some(|entity, session, cx| marker(entity).update(cx, |m, _| m.selected.push(session))),
            focus_in: Some(|entity, session, _, cx| {
                marker(entity).update(cx, |m, _| m.focused_for.push(session));
                true
            }),
            record: |entity, session, record, cx| {
                record.name = Some(format!("{session}/{}", marker(entity).read(cx).count));
            },
            opened: |entity, record, _, cx| {
                let count = record.name.as_deref().map_or(0, str::len) as i64;
                marker(entity).update(cx, |m, _| m.count = count);
            },
            closed: |entity, session, cx| {
                marker(entity).update(cx, |m, _| m.selected.retain(|&s| s != session));
            },
            left: |entity, session, cx| {
                marker(entity).update(cx, |m, _| m.focused_for.retain(|&s| s != session));
            },
        }),
    },
];

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn a_window_makes_its_features_state_and_hands_facts_to_its_hooks(cx: &mut TestAppContext) {
    if isolate("a_window_makes_its_features_state_and_hands_facts_to_its_hooks") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, STAND_IN);
    draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.feature_def(FeatureId::Usage).is_some()));
    assert!(read(cx, &window, |ws| ws.feature_def(FeatureId::History).is_none()), "only what is installed");
    assert_eq!(read(cx, &window, |ws| ws.feature_states.len()), 2, "one slot per entry, in table order");

    drive(cx, &window, |ws, window, cx| {
        let def = ws.feature_def(FeatureId::Usage).expect("installed");
        let entity = ws.feature_entity(1).expect("made with the window").clone();
        let facts = ws.facts(cx);
        assert_eq!(facts.tab, TabId::Changes);
        assert_eq!(facts.profile, ws.profile());
        assert!(facts.features.has(FeatureId::Usage), "an always-on feature is on");
        assert_eq!(facts.panel_width, gpui_kit::px(ws.columns().panel));
        let active = facts.active().expect("a terminal is selected");
        assert_eq!(active.folder(cx), active.terminal.read(cx).cwd.as_deref());
        assert_eq!(facts.on_screen().map(|s| s.id).collect::<Vec<_>>(), [active.id]);
        assert!(!facts.agent_on_screen(cx), "a plain terminal");

        let tab = &def.tabs[0];
        assert!((tab.shown)(&facts, cx));
        assert_eq!((tab.count.expect("counts"))(&entity, &facts, cx), Some(42), "41 and one on screen");
        assert_eq!((tab.spoken)(&entity, &facts, cx), "Changes tab");
        // `view` and `actions` take `cx` mutably while the facts, which borrow the window, are alive.
        let _ = (tab.view)(&entity, &facts, window, cx);
        assert!((tab.actions.expect("has actions"))(&entity, &facts, window, cx).is_none());
        assert_eq!(marker(&entity).read(cx).count, 41 + 1 + 10);

        // A row extra, through the narrow view of the rows.
        let mut rows = vec![RowInfo::test_row(3), RowInfo::test_row(4)];
        let mut extras = RowExtras::new(&mut rows);
        (def.row_extras.expect("has extras"))(&entity, &facts, &mut extras, cx);
        let marks: Vec<_> = extras.iter_mut().map(|(id, slot)| (id, *slot)).collect();
        assert_eq!(marks, [(3, Some(RowExtra { count: 3 })), (4, Some(RowExtra { count: 4 }))]);
        // The terminals' lifetime hooks and the close question's stake.
        let stake = (def.close_stake.expect("has a stake"))(&entity, &[1, 2], cx).expect("a stake");
        assert_eq!((stake.count, stake.sentence.as_str()), (2, "2 stand-ins"));
        let state_def = def.state.as_ref().expect("has state");
        let mut record = SessionRecord::new(7, None, None);
        (state_def.record)(&entity, 7, &mut record, cx);
        assert_eq!(record.name.as_deref(), Some("7/52"));
        (state_def.opened)(&entity, &record, window, cx);
        assert_eq!(marker(&entity).read(cx).count, 4);
        marker(&entity).update(cx, |m, _| (m.selected, m.focused_for) = (vec![7, 8], vec![7, 8]));
        (state_def.closed)(&entity, 7, cx);
        (state_def.left)(&entity, 8, cx);
        assert_eq!(
            (marker(&entity).read(cx).selected.clone(), marker(&entity).read(cx).focused_for.clone()),
            (vec![8], vec![7])
        );

        let mut state = MenuState::default();
        ws.feature_menus(&mut state, cx);
        assert_eq!(state.disabled, [ActionId::NewNote], "the hook speaks whether the feature is on or off");
    });
}

#[gpui_kit::test]
fn a_feature_tells_the_window_by_events(cx: &mut TestAppContext) {
    if isolate("a_feature_tells_the_window_by_events") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, STAND_IN);
    draw(cx, &window);
    let emit = |cx: &mut TestAppContext, event: FeatureEvent| {
        drive(cx, &window, |ws, _, cx| {
            let entity = marker(ws.feature_entity(1).expect("made with the window"));
            // Inside the window's own update: the event is heard when it ends, not through its lease.
            entity.update(cx, |_, cx| cx.emit(event));
        });
    };

    // The keys are in the panel's tabs; a feature's editor taking them makes the content the area.
    let area = |cx: &mut TestAppContext| drive(cx, &window, |ws, window, cx| ws.focus_area(window, cx));
    drive(cx, &window, |ws, window, cx| {
        let tabs = ws.areas.handle(Area::PanelTabs).cloned().expect("the panel's tabs have a handle");
        window.focus(&tabs, cx);
    });
    assert_eq!(area(cx), Some(Area::PanelTabs));
    emit(cx, FeatureEvent::Editing(true));
    assert!(read(cx, &window, |ws| ws.feature_editing));
    assert_eq!(area(cx), Some(Area::PanelContent), "an editor holds the keys");
    emit(cx, FeatureEvent::Editing(false));
    assert!(!read(cx, &window, |ws| ws.feature_editing));
    assert_eq!(area(cx), Some(Area::PanelTabs));

    drive(cx, &window, |ws, _, _| ws.dirty = false);
    emit(cx, FeatureEvent::Dirty);
    assert!(read(cx, &window, |ws| ws.dirty));

    emit(cx, FeatureEvent::ShowTab(TabId::History));
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::History);

    // A text for a terminal that is not there is dropped; for one that is, pasted. Neither panics.
    emit(cx, FeatureEvent::SendText { session: 999, text: "nobody".into() });
    emit(cx, FeatureEvent::SendText { session: 1, text: "hello".into() });
    emit(cx, FeatureEvent::FocusTerminal);
}

#[gpui_kit::test]
fn the_window_tells_a_feature_which_terminal_it_chose_and_asks_it_to_take_the_keys(cx: &mut TestAppContext) {
    if isolate("the_window_tells_a_feature_which_terminal_it_chose_and_asks_it_to_take_the_keys") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, STAND_IN);
    drive(cx, &window, |ws, window, cx| {
        ws.new_session(window, cx);
    });
    drive(cx, &window, |ws, window, cx| {
        let first = ws.sessions[0].id;
        let entity = marker(ws.feature_entity(1).expect("made with the window"));
        entity.update(cx, |m, _| m.selected.clear());
        ws.activate(0, window, cx);
        assert_eq!(entity.read(cx).selected, [first], "told once, for the terminal now on screen");
        ws.activate(0, window, cx);
        assert_eq!(entity.read(cx).selected, [first], "choosing the same one again says nothing");
        assert!(ws.focus_feature(window, cx), "the feature took the keys");
        assert_eq!(entity.read(cx).focused_for, [first]);
    });
}

#[gpui_kit::test]
fn a_window_made_with_no_table_has_no_features(cx: &mut TestAppContext) {
    if isolate("a_window_made_with_no_table_has_no_features") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, TABLE);
    draw(cx, &window);
    for id in FeatureId::ALL {
        assert_eq!(read(cx, &window, |ws| ws.feature_def(id).is_some()), TABLE.iter().any(|d| d.id == id));
    }
}

fn change(path: &str) -> FileChange {
    FileChange {
        path: path.into(),
        original_path: None,
        kind: ChangeKind::Modified,
        added: Some(1),
        removed: Some(0),
        lfs: false,
        submodule: None,
    }
}

/// A repository with three changed files, as git answers for the session shown.
fn three_changes() -> RepoStatus {
    RepoStatus {
        root: "/repo".into(),
        branch: Some("main".into()),
        files: vec![change("src/a.rs"), change("src/b.rs"), change("README.md")],
        base_rev: "HEAD".into(),
        compare: None,
        compare_missing: None,
        dirty: true,
        upstream: None,
        superproject: None,
    }
}

#[gpui_kit::test]
fn changes_is_a_table_entry_the_frame_reads(cx: &mut TestAppContext) {
    if isolate("changes_is_a_table_entry_the_frame_reads") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    drive(cx, &window, |ws, window, cx| {
        ws.sessions[0].git.status = Some(three_changes());
        ws.sync_changes_list(cx);
        let (at, tab) = ws.tab_entry(TabId::Changes).expect("Changes is in the table");
        assert_eq!(at, 0, "first in tab order");
        assert_eq!(ws.panel_tab_list(cx)[..2], [TabId::Changes, TabId::History]);
        let entity = ws.feature_entity(at).expect("made with the window").clone();

        // The tab's words and count, from the table: what the tab bar draws and VoiceOver reads.
        let facts = ws.facts(cx);
        assert_eq!((tab.count.expect("counts"))(&entity, &facts, cx), Some(3));
        assert_eq!(
            (tab.spoken)(&entity, &facts, cx),
            spoken::panel_tab(PanelTab::Changes { files: Some(3) })
        );
        assert_eq!(
            (tab.label, tab.icon, tab.action),
            ("Changes", IconName::GitCompare, ActionId::ShowChanges)
        );
        assert!((tab.shown)(&facts, cx));
        // The panel and its buttons are built with the facts alive and `cx` lent mutably.
        let _ = (tab.view)(&entity, &facts, window, cx);
        assert!((tab.actions.is_none()), "Changes has no buttons beside the tabs");
        assert_eq!(facts.active().expect("selected").git.status.as_ref().map(|s| s.files.len()), Some(3));
    });
}

#[gpui_kit::test]
fn the_keys_walk_the_changes_list_through_the_table(cx: &mut TestAppContext) {
    if isolate("the_keys_walk_the_changes_list_through_the_table") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    drive(cx, &window, |ws, window, cx| {
        ws.sessions[0].git.status = Some(three_changes());
        ws.sync_changes_list(cx);
        let (at, tab) = ws.tab_entry(TabId::Changes).expect("in the table");
        let walk = tab.walk.as_ref().expect("Changes has a list");
        let entity = ws.feature_entity(at).expect("made with the window").clone();
        let id = ws.sessions[0].id;

        let rows = (walk.walkable)(&entity, &ws.facts(cx), cx);
        assert!(rows[0].band, "a section's band leads the list");
        assert!(rows.len() >= 4, "the band and the three files, in folders or not: {rows:?}");
        assert_eq!(ws.panel_lists(cx), [0], "one ⇥ stop");
        (walk.reveal)(&entity, &ws.facts(cx), rows[1].row, cx);

        // → ← on the band folds or opens its section; Space on it does the same through `press`.
        let band = rows[0].row;
        (walk.fold)(&entity, ws, id, band, cx);
        ws.sync_changes_list(cx);
        let folded = (walk.walkable)(&entity, &ws.facts(cx), cx);
        assert_ne!(folded.len(), rows.len(), "the section changed shape");
        assert!(
            (walk.press)(&entity, ws, id, band, Activate::Space, false, window, cx),
            "the keys stay in the list"
        );
        ws.sync_changes_list(cx);
        assert_eq!((walk.walkable)(&entity, &ws.facts(cx), cx), rows, "and back");
    });

    // The window's key path reaches the same functions: ↓ moves the cursor onto the next row.
    drive(cx, &window, |ws, window, cx| {
        ws.right_tab = TabId::Changes;
        assert!(ws.panel_key(&Keystroke::parse("down").expect("a key"), window, cx));
        assert_eq!(ws.areas.panel_at.map(|(tab, _)| tab), Some(TabId::Changes));
    });
}

#[gpui_kit::test]
fn the_show_changes_key_is_bound_by_the_window_from_the_table(cx: &mut TestAppContext) {
    if isolate("the_show_changes_key_is_bound_by_the_window_from_the_table") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        ws.show_tab(TabId::History, cx);
        ws.focus_terminal(window, cx);
    });
    draw(cx, &window);
    with_window(cx, &window, |window, cx| window.dispatch_action(Box::new(ShowChanges), cx));
    cx.run_until_parked();
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Changes);
}
