//! Changes as an entry of the feature table (`window::feature`). The tab's state is still the
//! `Workspace`'s and the `Session`'s (the status, the folds and the list on `Session.git`, the open
//! diff on the window), so the tab is drawn, shown and walked through its [`WindowSideTab`], the
//! temporary side door that hands it the window. The facts still serve its count and words. Phase 5d
//! moves that state into [`ChangesPanel`], deletes the side door and fills in the entity's hooks.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureDef, FeatureEvent, StateDef, TabDef, WindowSideTab};

/// The Changes tab's entity: nothing yet. It exists so that the tab is made with the window like
/// any feature's, and says what it will say to the window ([`FeatureEvent`]).
pub(in crate::window) struct ChangesPanel;

impl EventEmitter<FeatureEvent> for ChangesPanel {}

/// The changed files the active terminal's repository has, if git has answered.
fn files(facts: &WindowFacts<'_>) -> Option<usize> {
    facts.active()?.git.status.as_ref().map(|status| status.files.len())
}

/// Changes' side door (5d deletes it): today's code on the window, unchanged.
const SIDE: WindowSideTab = WindowSideTab {
    view: |ws, window, cx| ws.render_changes_tab(window, cx),
    on_show: |ws, session, cx| {
        if let Some(ix) = ws.find_session(session) {
            ws.refresh_git(ix, cx);
        }
    },
    walkable: Workspace::changes_walkable,
    press: |ws, _, row, _, following, _, cx| ws.changes_press(row, following, cx),
    fold: |ws, _, row, cx| ws.changes_fold(row, cx),
    reveal: Workspace::changes_reveal,
};

/// The Changes entry: the tab, what showing it does, its count and words, its panel and its keys.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Changes,
    tabs: &[TabDef {
        id: TabId::Changes,
        icon: IconName::GitCompare,
        label: "Changes",
        action: ActionId::ShowChanges,
        on_show: None,
        shown: |_, _| true,
        count: Some(|_, facts, _| files(facts).filter(|&n| n > 0)),
        spoken: |_, facts, _| spoken::panel_tab(PanelTab::Changes { files: files(facts) }),
        // Drawn through the side door; there is no entity-side panel yet.
        view: |_, _, _, _| gpui_kit::div().into_any_element(),
        walk: None,
        actions: None,
        refresh_ticks: None,
        returns_to_previous: false,
        window_side: Some(&SIDE),
    }],
    bind: None,
    row_extras: None,
    close_stake: None,
    menu: None,
    tick: None,
    profile_shown: None,
    profiles: None,
    refresh: None,
    trusted: None,
    spawn: None,
    state: Some(StateDef {
        make: |window, cx| {
            let entity = cx.new(|_| ChangesPanel);
            Workspace::listen(&entity, window, cx)
        },
        selected: None,
        focus_in: None,
        // Nothing of Changes is saved with a terminal: its status is read again from git.
        record: |_, _, _, _| {},
        opened: |_, _, _, _| {},
        closed: |_, _, _| {},
        left: |_, _, _| {},
    }),
};
