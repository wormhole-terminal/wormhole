//! Usage's entry in the feature table: one tab, offered while a coding agent is detected in a terminal
//! on screen, redrawn every so often while it shows, and its key and menu line hidden when it is not
//! offered. The entity is the app's reports (`state`); it holds no per-terminal state, so the
//! terminal hooks of its `StateDef` have nothing to do.
use gpui_kit::assets::IconName;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use super::state::{self, profile_shown};
use super::view;
use crate::window::Workspace;
use crate::window::feature::{FeatureDef, StateDef, TabDef};

/// "resets 34m" and "Updated 2m ago" move on: the tab redraws this often while it shows, in ticks of
/// the window (400 ms each).
const REDRAW_TICKS: u64 = 25;

pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Usage,
    tabs: &[TabDef {
        id: TabId::Usage,
        icon: IconName::Gauge,
        label: "Usage",
        action: ActionId::ShowUsage,
        on_show: None,
        // It reads the same `TerminalView::agent` the title row's context status is keyed on.
        shown: |facts, cx| facts.agent_on_screen(cx),
        count: None,
        spoken: |_, _, _| spoken::panel_tab(PanelTab::Usage),
        view: view::tab,
        walk: None,
        actions: None,
        refresh_ticks: Some(REDRAW_TICKS),
        returns_to_previous: true,
        window_side: None,
    }],
    bind: None,
    row_extras: None,
    close_stake: None,
    // The key is the window's (it binds each offered tab's action); its menu line goes with the tab.
    menu: Some(|_, facts, state, cx| {
        if !facts.agent_on_screen(cx) {
            state.hidden.push(ActionId::ShowUsage);
        }
    }),
    tick: None,
    profile_shown: Some(profile_shown),
    profiles: None,
    refresh: None,
    trusted: None,
    spawn: None,
    state: Some(STATE),
};

const STATE: StateDef = StateDef {
    make: |window, cx| Workspace::listen(&state::shared(cx), window, cx),
    selected: None,
    focus_in: None,
    record: |_, _, _, _| {},
    opened: |_, _, _, _| {},
    closed: |_, _, _| {},
    left: |_, _, _| {},
};
