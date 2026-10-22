//! Changes as an entry of the feature table (`window::feature`). The tab's state is still the
//! `Workspace`'s and the `Session`'s (the status, the folds and the list on `Session.git`, the open
//! diff on the window), so each hook here calls today's code on them through the facts, or through
//! the `Workspace` the window passes for the hooks that change it. Phase 5d moves that state into
//! [`ChangesPanel`], and the hooks then lose those arguments.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureDef, FeatureEvent, FeatureState, StateDef, TabDef, WalkFns};
use crate::window::focus::Walkable;

/// The Changes tab's entity: nothing yet. It exists so that the tab is made with the window like
/// any feature's, and says what it will say to the window ([`FeatureEvent`]).
pub(in crate::window) struct ChangesPanel;

impl EventEmitter<FeatureEvent> for ChangesPanel {}

/// The changed files the active terminal's repository has, if git has answered.
fn files(facts: &WindowFacts<'_>) -> Option<usize> {
    facts.active()?.git.status.as_ref().map(|status| status.files.len())
}

fn walkable(_: &AnyEntity, facts: &WindowFacts<'_>, _: &App) -> Vec<Walkable> {
    facts.workspace().changes_walkable()
}

/// The Changes entry: the tab, what showing it does, its count and words, its panel and its keys.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Changes,
    tabs: &[TabDef {
        id: TabId::Changes,
        icon: IconName::GitCompare,
        label: "Changes",
        action: ActionId::ShowChanges,
        on_show: Some(|_, ws, session, cx| {
            if let Some(ix) = ws.find_session(session) {
                ws.refresh_git(ix, cx);
            }
        }),
        shown: |_, _| true,
        count: Some(|_, facts, _| files(facts).filter(|&n| n > 0)),
        spoken: |_, facts, _| spoken::panel_tab(PanelTab::Changes { files: files(facts) }),
        view: |_, facts, window, cx| facts.workspace().render_changes_tab(window, cx),
        walk: Some(WalkFns {
            walkable,
            press: |_, ws, _, row, _, following, _, cx| ws.changes_press(row, following, cx),
            fold: |_, ws, _, row, cx| ws.changes_fold(row, cx),
            reveal: |_, facts, row, _| facts.workspace().changes_reveal(row),
        }),
        actions: None,
    }],
    bind: None,
    row_extras: None,
    close_stake: None,
    menu: None,
    state: Some(StateDef {
        make: |window, cx| {
            let entity = cx.new(|_| ChangesPanel);
            let _events = Workspace::listen(&entity, window, cx);
            FeatureState { entity: entity.into(), _events }
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
