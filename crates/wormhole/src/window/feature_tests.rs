//! The feature table's contract: the app's table agrees with core's, a window gets the table the app
//! installed, and the hooks' shapes work end to end on a stand-in feature.

use std::collections::HashSet;

use gpui_kit::assets::IconName;
use gpui_kit::{AnyElement, AnyEntity, AppContext as _, IntoElement as _, TestAppContext};
use wormhole_core::config::{FEATURES, FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::storage::store::WindowRecord;

use crate::window::feature::{FeatureDef, StateDef, TabDef};
use crate::window::features::TABLE;
use crate::window::view_tests::{draw, drive, isolate_in, read, start_with_table};

/// Features core lists that have not moved into the app's table yet. Each phase 5 step takes one off
/// this list when it adds the entry; the list ends empty.
const NOT_YET: &[FeatureId] = &[
    FeatureId::Changes,
    FeatureId::History,
    FeatureId::Notes,
    FeatureId::Usage,
    FeatureId::Project,
    FeatureId::Worktrees,
];

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

/// What a stand-in feature keeps.
struct Marker(i64);

/// A feature of its own in every slot, so the shapes are type-checked and exercised.
static STAND_IN: &[FeatureDef] = &[FeatureDef {
    id: FeatureId::Notes,
    tabs: &[TabDef {
        id: TabId::Notes,
        icon: IconName::PanelRight,
        label: "Stand-in",
        action: ActionId::ShowNotes,
        shown: |facts, _| facts.active().is_some(),
        count: Some(|entity, facts, cx| {
            let marker = entity.clone().downcast::<Marker>().ok()?;
            Some(marker.read(cx).0 as usize + facts.on_screen().count())
        }),
        spoken: |_, facts, _| format!("{:?} tab", facts.tab),
        view: |_, _, _, _| -> AnyElement { gpui_kit::div().into_any_element() },
        walk: None,
        actions: None,
    }],
    bind: Some(|_, root| root),
    row_extras: None,
    close_stake: None,
    menu: None,
    state: Some(StateDef {
        make: |_, _, cx| cx.new(|_| Marker(41)).into(),
        record: |_, _, _, _| {},
        opened: |_, _, _, _| {},
        closed: |_, _, _| {},
        left: |_, _, _| {},
    }),
}];

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

#[gpui_kit::test]
fn a_window_gets_the_table_the_app_installed_and_hands_facts_to_its_hooks(cx: &mut TestAppContext) {
    if isolate("a_window_gets_the_table_the_app_installed_and_hands_facts_to_its_hooks") {
        return;
    }
    let window = start_with_table(cx, WindowRecord { id: 1, ..Default::default() }, STAND_IN);
    draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.feature_def(FeatureId::Notes).is_some()));
    assert!(read(cx, &window, |ws| ws.feature_def(FeatureId::Usage).is_none()), "only what is installed");

    drive(cx, &window, |ws, window, cx| {
        let def = ws.feature_def(FeatureId::Notes).expect("installed");
        let make = def.state.as_ref().expect("has state").make;
        let entity: AnyEntity = make(cx.weak_entity(), window, cx);
        let facts = ws.facts(cx);
        assert_eq!(facts.tab, TabId::Changes);
        assert_eq!(facts.profile, ws.profile());
        let active = facts.active().expect("a terminal is selected");
        assert_eq!(active.folder, active.terminal.read(cx).cwd.as_deref());
        assert_eq!(facts.on_screen().map(|s| s.id).collect::<Vec<_>>(), [active.id]);
        let tab = &def.tabs[0];
        assert!((tab.shown)(&facts, cx));
        assert_eq!((tab.count.expect("counts"))(&entity, &facts, cx), Some(42), "41 and one on screen");
        assert_eq!((tab.spoken)(&entity, &facts, cx), "Changes tab");
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
