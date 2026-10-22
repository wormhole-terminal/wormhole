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
use wormhole_core::view::focus::list::Item;
use wormhole_core::view::spoken::{self, PanelTab};

use crate::actions::ShowChanges;
use crate::window::Workspace;
use crate::window::feature::{FeatureDef, FeatureEvent, Stake, StateDef, TabDef, WalkFns};
use crate::window::features::TABLE;
use crate::window::features::git::changes;
use crate::window::features::notes::testing as notes;
use crate::window::focus::{Activate, Walkable};
use crate::window::sidebar::{RowExtra, RowExtras, RowInfo};
use crate::window::view_tests::{draw, drive, isolate_in, read, start, start_with_table, with_window};

/// Features core lists that have not moved into the app's table yet. Each phase 5 step takes one off
/// this list when it adds the entry; the list ends empty.
const NOT_YET: &[FeatureId] = &[FeatureId::History, FeatureId::Project, FeatureId::Worktrees];

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
    /// What the tab's hooks were asked: `shown:7`, `press:7:1`, `fold:7:1`.
    asked: Vec<String>,
    /// The row `reveal` was asked for: it only gets `&App`, so it can't change the entity.
    revealed: std::cell::Cell<Option<usize>>,
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
            on_show: Some(|entity, session, cx| {
                marker(entity).update(cx, |m, _| m.asked.push(format!("shown:{session}")));
            }),
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
            walk: Some(WalkFns {
                walkable: |_, _, _| {
                    (0..2)
                        .map(|row| Walkable { row, item: Item { level: 0, fold: None }, band: row == 0 })
                        .collect()
                },
                press: |entity, session, row, _, _, _, cx| {
                    marker(entity).update(cx, |m, _| m.asked.push(format!("press:{session}:{row}")));
                    row == 1
                },
                fold: |entity, session, row, cx| {
                    marker(entity).update(cx, |m, _| m.asked.push(format!("fold:{session}:{row}")));
                },
                reveal: |entity, _, row, cx| marker(entity).read(cx).revealed.set(Some(row)),
            }),
            actions: Some(|entity, facts, _, cx| {
                let on = facts.agent_on_screen(cx);
                marker(entity).update(cx, |marker, _| marker.count += 10);
                on.then(|| gpui_kit::div().into_any_element())
            }),
            refresh_ticks: None,
            returns_to_previous: false,
            window_side: None,
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
        tick: None,
        profile_shown: None,
        state: Some(StateDef {
            make: |window, cx| {
                let entity = cx.new(|_| Marker {
                    count: 41,
                    selected: Vec::new(),
                    focused_for: Vec::new(),
                    asked: Vec::new(),
                    revealed: Default::default(),
                });
                Workspace::listen(&entity, window, cx)
            },
            selected: Some(|entity, session, cx| marker(entity).update(cx, |m, _| m.selected.push(session))),
            focus_in: Some(|entity, _, session, _, cx| {
                marker(entity).update(cx, |m, _| m.focused_for.push(session));
                true
            }),
            record: |entity, session, record, cx| {
                record.name = Some(format!("{session}/{}", marker(entity).read(cx).count));
            },
            // The window calls it for every terminal it opens; a record the stand-in didn't write has
            // nothing for it.
            opened: |entity, record, _, cx| {
                if let Some(name) = &record.name {
                    marker(entity).update(cx, |m, _| m.count = name.len() as i64);
                }
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

/// The side door (`TabDef::window_side`) is Changes' until 5d and nobody else's: a second user would
/// make the window-shaped hook the contract's habit again. It asserts on where the door is *set*
/// (`window_side: Some`), so the frame code that only reads it (`panel.rs`) is no exemption to hide
/// behind. `Context<Workspace>` in a feature's other code is the same debt and is the 5g checker's to
/// ban when each feature has its own state.
#[test]
fn only_changes_uses_the_side_door() {
    fn walk(dir: &std::path::Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("reads the features folder") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && std::fs::read_to_string(&path).expect("reads a source file").contains("window_side: Some")
            {
                found.push(path.to_string_lossy().into_owned());
            }
        }
    }
    let mut found = Vec::new();
    walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/window/features"), &mut found);
    found.sort();
    let names: Vec<&str> =
        found.iter().filter_map(|path| path.split("src/window/features/").nth(1)).collect();
    assert_eq!(names, ["git/changes/entry.rs"]);
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

        // The tab's own hooks need no `Workspace`: the entity, the session and the app's `cx` are enough.
        (tab.on_show.expect("has on_show"))(&entity, 7, cx);
        let walk = tab.walk.as_ref().expect("has a list");
        assert_eq!((walk.walkable)(&entity, &facts, cx).len(), 2);
        assert!((walk.press)(&entity, 7, 1, Activate::Space, false, window, cx));
        assert!(!(walk.press)(&entity, 7, 0, Activate::Return, true, window, cx));
        (walk.fold)(&entity, 7, 1, cx);
        (walk.reveal)(&entity, &facts, 1, cx);
        assert_eq!(marker(&entity).read(cx).asked, ["shown:7", "press:7:1", "press:7:0", "fold:7:1"]);
        assert_eq!(marker(&entity).read(cx).revealed.get(), Some(1));

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

    // A field that is left behind does not keep the panel's content as the focused area: another tab,
    // or another terminal, ends it. The same tab does not.
    emit(cx, FeatureEvent::ShowTab(TabId::Changes));
    emit(cx, FeatureEvent::Editing(true));
    emit(cx, FeatureEvent::ShowTab(TabId::Changes));
    assert!(read(cx, &window, |ws| ws.feature_editing), "the tab did not change");
    emit(cx, FeatureEvent::ShowTab(TabId::History));
    assert!(!read(cx, &window, |ws| ws.feature_editing), "the tab went away");
    drive(cx, &window, |ws, window, cx| {
        ws.new_session(window, cx);
    });
    emit(cx, FeatureEvent::Editing(true));
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert!(!read(cx, &window, |ws| ws.feature_editing), "another terminal was selected");

    drive(cx, &window, |ws, _, _| ws.dirty = false);
    emit(cx, FeatureEvent::Dirty);
    assert!(read(cx, &window, |ws| ws.dirty));

    emit(cx, FeatureEvent::ShowTab(TabId::History));
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::History);

    // A text for a terminal that is not there is dropped; for one that is, pasted. Neither panics.
    emit(cx, FeatureEvent::SendText { session: 999, text: "nobody".into(), submit: false });
    emit(cx, FeatureEvent::SendText { session: 1, text: "hello".into(), submit: true });
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
fn a_window_has_the_features_of_the_table_it_was_made_with(cx: &mut TestAppContext) {
    if isolate("a_window_has_the_features_of_the_table_it_was_made_with") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, TABLE);
    draw(cx, &window);
    for id in FeatureId::ALL {
        assert_eq!(read(cx, &window, |ws| ws.feature_def(id).is_some()), TABLE.iter().any(|d| d.id == id));
    }
}

#[gpui_kit::test]
fn a_window_made_with_an_empty_table_has_no_features(cx: &mut TestAppContext) {
    if isolate("a_window_made_with_an_empty_table_has_no_features") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, &[]);
    draw(cx, &window);
    for id in FeatureId::ALL {
        assert!(read(cx, &window, |ws| ws.feature_def(id).is_none()), "{id:?}");
    }
    assert_eq!(read(cx, &window, |ws| ws.feature_states.len()), 0);
    assert!(read(cx, &window, |ws| ws.tab_entry(TabId::Changes).is_none()));
    // The panel still draws, with the tabs the window offers on its own and no Changes.
    let tabs = drive(cx, &window, |ws, _, cx| ws.panel_tab_list(cx));
    assert!(!tabs.contains(&TabId::Changes), "{tabs:?}");
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
        // The panel is drawn through the temporary side door, which has the window; the entity-side
        // `view` is a stub that draws nothing.
        let side = tab.window_side.expect("Changes uses the side door until 5d");
        let _ = (side.view)(ws, window, cx);
        assert!(tab.on_show.is_none() && tab.walk.is_none(), "its hooks are the side door's");
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
        let (_, tab) = ws.tab_entry(TabId::Changes).expect("in the table");
        let side = tab.window_side.expect("Changes uses the side door until 5d");
        let id = ws.sessions[0].id;

        let rows = (side.walkable)(ws);
        assert!(rows[0].band, "a section's band leads the list");
        assert!(rows.len() >= 4, "the band and the three files, in folders or not: {rows:?}");
        assert_eq!(ws.panel_lists(cx), [0], "one ⇥ stop");
        (side.reveal)(ws, rows[1].row);

        // → ← on the band folds or opens its section; Space on it does the same through `press`.
        let band = rows[0].row;
        (side.fold)(ws, id, band, cx);
        ws.sync_changes_list(cx);
        let folded = (side.walkable)(ws);
        assert_ne!(folded.len(), rows.len(), "the section changed shape");
        assert!((side.press)(ws, id, band, Activate::Space, false, window, cx), "the keys stay in the list");
        ws.sync_changes_list(cx);
        assert_eq!((side.walkable)(ws), rows, "and back");
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

#[gpui_kit::test]
fn showing_a_tab_runs_the_features_on_show_hook(cx: &mut TestAppContext) {
    if isolate("showing_a_tab_runs_the_features_on_show_hook") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    // A folder to ask git about: the refresh only starts for a terminal that reports one.
    let folder = std::env::temp_dir();
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |terminal, _| terminal.cwd = Some(folder));
        ws.show_tab(TabId::History, cx);
    });
    drive(cx, &window, |ws, _, cx| {
        assert!(!ws.sessions[0].git.loading(), "History's refresh is not git's");
        // `show_tab` → the table's `on_show` for Changes → `refresh_git`: a refresh is under way.
        ws.show_tab(TabId::Changes, cx);
        assert_eq!(ws.right_tab, TabId::Changes);
        assert!(ws.sessions[0].git.loading(), "showing Changes asked git through the table");
    });
}

/// Allocations the second draw of a one-terminal window on Changes (three changed files) may make.
/// Measured: 3,697 allocations (1.8 MB), the same on every run: the count is exact. The budget is that
/// plus about a hundredth. It guards the window's own frame, which the table's loops (tab bar, bindings, panel
/// body) are part of: a lookup, clone or `Vec` per feature per frame shows as a few dozen and a
/// per-row one as hundreds. Any rise must be explained in the change that makes it, and then raised
/// here with the new measurement.
const WINDOW_FRAME_ALLOCATIONS: u64 = 3_735;

#[gpui_kit::test]
fn a_window_frame_on_changes_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_window_frame_on_changes_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].git.status = Some(three_changes());
        ws.sync_changes_list(cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Changes);
    // The first draw builds what a window keeps (lists, layouts, caches); the second is a frame.
    draw(cx, &window);
    let before = thread_counts();
    draw(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on Changes: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= WINDOW_FRAME_ALLOCATIONS,
        "a window frame on Changes made {} allocations (budget {WINDOW_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}

/// Allocations the second draw of a one-terminal window on Notes (three next steps, three notes) may
/// make. Measured: 5,788 allocations (3.1 MB), the same on every run. Like the Changes budget it is that plus
/// about a hundredth; the notes tab adds its rows, the shared copy the closures hold and the list's
/// own, so a cost per note or per step per frame shows at once.
const NOTES_FRAME_ALLOCATIONS: u64 = 5_846;

#[gpui_kit::test]
fn a_window_frame_on_notes_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_window_frame_on_notes_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        for text in ["Fix the flaky login test", "Update the README", "Tag the release"] {
            notes::add_task(ws, 0, text, cx);
        }
        for text in ["First note\nwith a second line", "Second note", "Third note"] {
            notes::add_note(ws, 0, text, false, window, cx);
        }
        ws.show_tab(TabId::Notes, cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Notes);
    // The first draw builds what a window keeps (lists, layouts, caches); the second is a frame.
    draw(cx, &window);
    draw(cx, &window);
    let before = thread_counts();
    draw(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on Notes: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= NOTES_FRAME_ALLOCATIONS,
        "a window frame on Notes made {} allocations (budget {NOTES_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}

/// Allocations the third draw of a one-terminal window on Usage (an agent on screen, three agents'
/// reports) may make. Measured: 22,883 allocations (17 MB), the same on every run and the same before Usage joined the table. The budget is that
/// plus about a hundredth; the tab adds a card per report and a bar per limit, so a cost per report
/// or per limit per frame shows at once.
const USAGE_FRAME_ALLOCATIONS: u64 = 23_112;

#[gpui_kit::test]
fn a_window_frame_on_usage_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    use wormhole_core::agents::AgentId;
    use wormhole_core::agents::usage::{PlanLimit, Usage, Window};
    if isolate("a_window_frame_on_usage_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    let far = 4_000_000_000;
    let report = |agent, limits: Vec<PlanLimit>, five_hour, response_tokens| Usage {
        agent,
        five_hour,
        seven_day: five_hour,
        limits,
        response_tokens,
        updated: 100,
    };
    let limit = |label: &str, used| PlanLimit { label: label.into(), used, resets_at: Some(far) };
    cx.update(|cx| {
        use crate::window::features::usage::record;
        record(report(AgentId::Claude, Vec::new(), Some(Window { used: 85.0, resets_at: far }), None), cx);
        record(
            report(AgentId::Codex, vec![limit("5-hour limit", 30.0), limit("Weekly", 100.0)], None, None),
            cx,
        );
        record(report(AgentId::Gemini, Vec::new(), None, Some(1234)), cx);
    });
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, _| t.agent = Some(AgentId::Claude));
        ws.show_tab(TabId::Usage, cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Usage);
    // The first draws build what a window keeps; the third is a frame.
    draw(cx, &window);
    draw(cx, &window);
    let before = thread_counts();
    draw(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on Usage: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= USAGE_FRAME_ALLOCATIONS,
        "a window frame on Usage made {} allocations (budget {USAGE_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}

/// `n` changed files, in folders of ten, so a per-row cost scales with `n`.
fn many_changes(n: usize) -> RepoStatus {
    let mut status = three_changes();
    status.files = (0..n).map(|ix| change(&format!("src/dir{}/file{ix}.rs", ix / 10))).collect();
    status
}

/// A feature of the same shape as the stand-in above in another slot, for the crowd the table's loops
/// are measured over (tab bar, bindings, panel body). Its tab shows, counts and says its words.
macro_rules! crowd_member {
    ($id:expr, $tab:expr, $action:expr) => {
        FeatureDef {
            id: $id,
            tabs: &[TabDef {
                id: $tab,
                icon: IconName::PanelRight,
                label: "Crowd",
                action: $action,
                on_show: None,
                shown: |facts, _| facts.active().is_some(),
                count: Some(|entity, facts, cx| {
                    Some(marker(entity).read(cx).count as usize + facts.on_screen().count())
                }),
                spoken: |_, facts, _| format!("{:?} tab", facts.tab),
                view: |_, facts, _, _| -> AnyElement {
                    gpui_kit::div().w(gpui_kit::px(f32::from(facts.panel_width))).into_any_element()
                },
                walk: None,
                actions: None,
                refresh_ticks: None,
                returns_to_previous: false,
                window_side: None,
            }],
            bind: Some(|_, _, root, _| root),
            row_extras: None,
            close_stake: None,
            menu: None,
            tick: None,
            profile_shown: None,
            state: Some(StateDef {
                make: |window, cx| {
                    let entity = cx.new(|_| Marker {
                        count: 1,
                        selected: Vec::new(),
                        focused_for: Vec::new(),
                        asked: Vec::new(),
                        revealed: Default::default(),
                    });
                    Workspace::listen(&entity, window, cx)
                },
                selected: None,
                focus_in: None,
                record: |_, _, _, _| {},
                opened: |_, _, _, _| {},
                closed: |_, _, _| {},
                left: |_, _, _| {},
            }),
        }
    };
}

/// Changes and three stand-ins: what the loops are measured over.
static CROWD: &[FeatureDef] = &[
    changes::DEF,
    crowd_member!(FeatureId::History, TabId::History, ActionId::ShowHistory),
    crowd_member!(FeatureId::Usage, TabId::Usage, ActionId::ShowUsage),
    crowd_member!(FeatureId::Project, TabId::Project, ActionId::ShowProject),
];

/// Allocations the table's loops make in one frame, for 50 changed files and three stand-in features:
/// `bind_features`, the tab bar (`render_panel_tabs`, in `render_panel`) and the panel body. Measured:
/// 122 (26 KB), exact on every run; the budget is that plus about a hundredth. Run with 50 files and
/// four features so that a cost per row (fifty times) or per feature (four times) shows at once; a
/// change that adds either must say why, and raise this with the new measurement.
const TABLE_LOOP_ALLOCATIONS: u64 = 124;

#[gpui_kit::test]
fn the_tables_loops_stay_within_their_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("the_tables_loops_stay_within_their_allocation_budget") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, CROWD);
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].git.status = Some(many_changes(50));
        ws.sync_changes_list(cx);
    });
    draw(cx, &window);
    let drawn = drive(cx, &window, |ws, window, cx| {
        let width = ws.columns().panel;
        // The first pass builds what is kept; the second is the frame.
        for _ in 0..2 {
            let _ = ws.bind_features(gpui_kit::div(), cx);
            let _ = ws.render_panel(width, window, cx).into_any_element();
        }
        let before = thread_counts();
        let _ = ws.bind_features(gpui_kit::div(), cx);
        let _ = ws.render_panel(width, window, cx).into_any_element();
        thread_counts().since(before)
    });
    eprintln!("table loops, 50 files, 4 features: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= TABLE_LOOP_ALLOCATIONS,
        "the table's loops made {} allocations (budget {TABLE_LOOP_ALLOCATIONS})",
        drawn.allocations
    );
}
