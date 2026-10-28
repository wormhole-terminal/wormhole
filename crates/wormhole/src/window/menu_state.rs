//! What the menus show for the window in front: which items are off, checked, disabled or hidden,
//! the terminals ⌘1–9 and the project's actions. The menu bar (`app::menus`) puts it up and the
//! command palette offers the same actions, so it is the window's to say, not the menu bar's.
//!
//! Part of the window: it reads the window's own state and the shared state (the profiles, the recent
//! projects, the settings) and must not use the menu bar (`app::menus`), the status item or the palette,
//! which read it.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::terminal::shells;

use crate::actions::DEBUG;
use crate::app_state::{AppState, app};
use crate::settings::settings;
use crate::window::Workspace;

/// What the menus show changed outside a key or the tick (a profile switch, a window moved to
/// another profile). The app root observes it and puts the menus up again at once.
pub(crate) struct MenusChanged;

impl Global for MenusChanged {}

/// Say that what the menus show changed; the app root's observer syncs them when the update that
/// called this is done, so it sees the finished state.
pub(crate) fn menus_changed(cx: &mut App) {
    cx.set_global(MenusChanged);
}

/// What the menus show for the window in front (the command palette offers the same actions).
pub(crate) fn front_window_state(cx: &mut App) -> MenuState {
    let front = AppState::front_window(cx);
    let full_screen = front
        .as_ref()
        .and_then(|front| front.handle.update(cx, |_, window, _| window.is_fullscreen()).ok())
        .unwrap_or(false);
    let mut state = front
        .and_then(|front| front.workspace.read_with(cx, |ws, cx| ws.menu_state(cx)).ok())
        .unwrap_or_default();
    // Zen mode hides the status bar and the minimap and the panel whatever they are set to: their toggles
    // have nothing to do until it ends (and the menu says which way Zen mode itself goes).
    let zen = crate::settings::zen(cx);
    if zen {
        state.disabled.extend([
            ActionId::ToggleStatusBar,
            ActionId::ToggleMinimap,
            ActionId::ToggleRightPanel,
        ]);
    } else {
        state.off.push(ActionId::ToggleZenMode);
    }
    // Their titles follow the person's own values, which are what comes back.
    let own = crate::settings::settings_without_zen(cx);
    if !own.status_bar {
        state.off.push(ActionId::ToggleStatusBar);
    }
    if !own.minimap {
        state.off.push(ActionId::ToggleMinimap);
    }
    let settings = settings(cx);
    if settings.terminals_in == wormhole_core::view::sidebar::TerminalsIn::Sidebar {
        state.off.push(ActionId::ToggleTerminalTabs);
    }
    if !full_screen {
        state.off.push(ActionId::FullScreen);
    }
    state.recent = app(cx).recent_projects.roots().iter().map(|root| folder_name(root)).collect();
    state.shells = shells::choices(settings).into_iter().map(|choice| choice.name).collect();
    let list = &app(cx).profiles;
    state.profiles = list.profiles().iter().map(|p| p.name.clone()).collect();
    state.current_profile = list.profiles().iter().position(|p| &p.id == list.active_id());
    state.window_profile = AppState::front_window(cx)
        .and_then(|front| list.profiles().iter().position(|p| p.id == front.profile));
    state.debug = DEBUG;
    state.persistent = crate::window::windows::persistent_terminals_on(cx);
    state
}

fn folder_name(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string())
}

impl Workspace {
    /// What the menus show about this window.
    pub(crate) fn menu_state(&self, cx: &App) -> MenuState {
        use ActionId::*;
        let mut state = MenuState::default();
        if !self.sidebar_shown() {
            state.off.push(ToggleSidebar);
        }
        if !self.notes_visible_without_zen() {
            state.off.push(ToggleRightPanel);
        }
        // The tab on show is the checked one: the table says each of its tabs' action.
        state.checked.extend(self.tab_entry(self.right_tab).map(|(_, entry)| entry.action));
        if self.active_session().pinned {
            state.checked.push(PinTerminal);
        }
        // Only with the Automation feature, or while it is hidden (to show it again).
        let hidden = self.active_terminal().read(cx).agent_hidden();
        if hidden {
            state.checked.push(HideFromAgents);
        } else if !crate::settings::settings_in(&self.profile, cx)
            .feature_enabled(wormhole_core::config::features::FeatureFlag::Automation)
        {
            state.hidden.push(HideFromAgents);
        }
        // An action whose feature flag is off isn't in the menus, the palette or the sheet.
        let settings = crate::settings::settings_in(&self.profile, cx);
        state.hidden.extend(wormhole_core::keys::keymap::entries().iter().filter_map(|entry| {
            let flag = wormhole_core::config::features::FeatureFlag::from_key(entry.flag?)?;
            (!settings.feature_enabled(flag)).then_some(entry.id)
        }));
        self.pane_menu_state(&mut state);
        if self.active_terminal().read(cx).cwd.is_none() {
            state.disabled.extend([RevealInFinder, CopyPath]);
        }
        self.feature_menus(&mut state, cx);
        let numbered = self.session_listing(cx).numbered();
        state.terminals = numbered.iter().map(|&ix| self.sessions[ix].name().to_string()).collect();
        state.current_terminal = numbered.iter().position(|&ix| ix == self.active);
        state
    }
}
