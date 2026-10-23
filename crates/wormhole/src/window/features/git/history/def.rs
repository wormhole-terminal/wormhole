//! History's entry in the feature table: its tab, its key and menu line (core's keymap tags them with the
//! feature, so off hides them), its keys over the list, and the terminals' lifetime. Nothing of it is
//! saved with a terminal: the log is read again from git.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use super::{History, HistoryState, keys, view};
use crate::window::Workspace;
use crate::window::feature::{FeatureDef, StateDef, TabDef};

/// The window's `History`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<History> {
    entity.clone().downcast::<History>().expect("the history entity is made by this entry's `make`")
}

/// The History entry: the tab, its panel and keys, the log's polling and the terminals' state.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::History,
    tabs: &[TabDef {
        id: TabId::History,
        icon: IconName::GitCommitHorizontal,
        label: "History",
        action: ActionId::ShowHistory,
        on_show: Some(|entity, facts, session, cx| {
            typed(entity).update(cx, |history, cx| history.shown(facts, session, cx));
        }),
        shown: |_, _| true,
        count: None,
        spoken: |_, _, _| spoken::panel_tab(PanelTab::History),
        view: view::tab,
        walk: Some(keys::WALK),
        actions: None,
        pick_branch: None,
        refresh_ticks: None,
        returns_to_previous: false,
    }],
    close_extra: None,
    bind: None,
    row_extras: None,
    close_stake: None,
    menu: None,
    // The log is read again while the tab shows.
    tick: Some(|entity, facts, _, cx| {
        typed(entity).update(cx, |history, cx| history.tick(facts, cx));
    }),
    profile_shown: None,
    profiles: None,
    // The window came to the front.
    refresh: Some(|entity, facts, cx| {
        typed(entity).update(cx, |history, cx| history.came_to_front(facts, cx));
    }),
    approves_programs: None,
    spawn: None,
    cover: None,
    // Its picker is closed by `shown`, which the window runs for every tab: a picker of this tab is not
    // open on another.
    tab_changed: Some(|entity, tab, cx| {
        typed(entity).update(cx, |history, cx| {
            if tab != TabId::History && history.picker.is_open() {
                history.picker.close();
                cx.notify();
            }
        });
    }),
    repo: None,
    state: Some(STATE),
};

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|cx| History::new(window, cx));
        Workspace::listen(&entity, window, cx)
    },
    // The picker belongs to the terminal it was opened from.
    selected: Some(|entity, _, cx| {
        typed(entity).update(cx, |history, cx| {
            if history.picker.is_open() {
                history.picker.close();
                cx.notify();
            }
        });
    }),
    focus_in: None,
    record: |_, _, _, _| {},
    opened: |entity, record, _, cx| {
        typed(entity).update(cx, |history, _| {
            history.of.insert(record.id, HistoryState::default());
        });
    },
    closed: |entity, session, cx| typed(entity).update(cx, |history, _| history.forget(session)),
    left: |entity, session, cx| typed(entity).update(cx, |history, _| history.forget(session)),
};
