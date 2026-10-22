//! Who handles the actions that aren't one window's, whichever window is in front, and the keys
//! taken before any binding sees them.
//!
//! A terminal takes keystrokes before GPUI's bindings (`TerminalKeys`), so a project action's
//! key is claimed there, first of all (projects keep priority, and `validate_key` keeps them
//! off the app's keys), and the app keys a terminal would send to its program (F6) are left to
//! the bindings.

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::onboarding::intro::{Handover, Style};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::sidebar::TerminalsIn;

use crate::menubar::MenuCommand;
use crate::settings::{SettingsStore, settings, update_settings};
use crate::terminal_view::{TerminalKeys, keystroke_spec};
use crate::workspace::actions::{
    About, BringAllToFront, CheckForUpdates, GoToFinished, GoToWaiting, Hide, HideOthers, Licenses,
    ManageProfiles, Minimize, MoveWindowToProfile, NewProfile, NewWindow, OpenFolder, OpenRecent,
    OpenSettings, OpenWindowInProfile, PlayIntroAurora, PlayIntroCollapse, PlayIntroEmerge, PlayIntroFtl,
    PlayIntroTentacles, PlayIntroVortex, PlayIntroVoyage, PlayWelcomeCursor, PlayWelcomeLogo,
    PlayWelcomePortal, ShowAll, ShowIntro, ShowKeyboardShortcuts, ShowLogs, SwitchProfile,
    ToggleCommandPalette, ToggleFullScreen, ToggleMinimap, ToggleStatusBar, ToggleTerminalTabs,
    ToggleThemeStudio, Zoom,
};
use crate::workspace::app::{AppState, app};
use crate::workspace::{
    about, ambient, app_menu, command_palette, focus, menu_bar, onboarding, settings_page, shortcut_sheet,
    studio, updates, window_lifecycle,
};

/// Bind the keys, let project keys go first, and handle the actions that aren't one window's.
/// Before the menus are set: they read the bindings for their key equivalents.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys(crate::workspace::actions::bindings());
    focus::init(cx);
    onboarding::init(cx);
    bind_jump_key(cx);
    cx.observe_global::<SettingsStore>(bind_jump_key).detach();
    TerminalKeys::claim_first(claim_window_key, cx);
    // Global listeners run inside the update of the window the action came from, which is then
    // unavailable: whatever opens or acts on windows is deferred.
    cx.on_action(|_: &NewWindow, cx| cx.defer(window_lifecycle::new_window_from_front));
    cx.on_action(|_: &CheckForUpdates, cx| {
        cx.defer(|cx| {
            settings_page::open_settings(Some(settings_page::Section::Updates), cx);
            updates::check_for_updates(true, cx);
        });
    });
    cx.on_action(|_: &NewProfile, cx| cx.defer(settings_page::open_new_profile_sheet));
    cx.on_action(|_: &ManageProfiles, cx| {
        cx.defer(|cx| settings_page::open_settings(Some(settings_page::Section::Profile), cx))
    });
    cx.on_action(|switch: &SwitchProfile, cx| {
        let id = switch.0.to_string();
        cx.defer(move |cx| {
            let Ok(id) = ProfileId::parse(&id) else { return };
            if let Err(message) = AppState::switch_profile(&id, cx) {
                tracing::warn!("switching profile: {message}");
            }
        });
    });
    cx.on_action(|open: &OpenWindowInProfile, cx| {
        let id = open.0.to_string();
        cx.defer(move |cx| {
            if let Ok(id) = ProfileId::parse(&id) {
                AppState::open_window_in_profile(&id, cx);
            }
        });
    });
    cx.on_action(|moved: &MoveWindowToProfile, cx| {
        let id = moved.0.to_string();
        cx.defer(move |cx| {
            let Ok(id) = ProfileId::parse(&id) else { return };
            if let Err(message) = AppState::move_front_window_to_profile(&id, cx) {
                tracing::warn!("moving the window: {message}");
            }
        });
    });
    cx.on_action(|_: &About, cx| cx.defer(about::open_about));
    cx.on_action(|_: &Licenses, cx| cx.defer(about::open_licenses));
    cx.on_action(|_: &OpenSettings, cx| cx.defer(|cx| settings_page::open_settings(None, cx)));
    // ⌘O: a terminal in the picked folder, in the window in front (the status item's too).
    cx.on_action(|_: &OpenFolder, cx| cx.defer(menu_bar::open_folder));
    // ⌥⌘J: the terminal that has waited longest, in any window.
    cx.on_action(|_: &GoToWaiting, cx| cx.defer(ambient::jump_to_waiting));
    // ⇧⌥⌘J: the terminal that finished last, then the one before it, in any window.
    cx.on_action(|_: &GoToFinished, cx| cx.defer(ambient::jump_to_finished));
    cx.on_action(|_: &ToggleThemeStudio, cx| cx.defer(theme_studio));
    // ⇧⌘P and ⌥⌘/: over the window in front, from any window and the terminal too.
    cx.on_action(|_: &ToggleCommandPalette, cx| cx.defer(command_palette::toggle));
    cx.on_action(|_: &ShowKeyboardShortcuts, cx| cx.defer(shortcut_sheet::toggle));
    cx.on_action(|_: &ToggleStatusBar, cx| update_settings(cx, |s| s.status_bar = !s.status_bar));
    cx.on_action(|_: &ToggleMinimap, cx| update_settings(cx, |s| s.minimap = !s.minimap));
    // Terminals in tabs or in the sidebar: every window at once.
    cx.on_action(|_: &ToggleTerminalTabs, cx| {
        update_settings(cx, |s| {
            s.terminals_in = match s.terminals_in {
                TerminalsIn::Sidebar => TerminalsIn::Tabs,
                TerminalsIn::Tabs => TerminalsIn::Sidebar,
            }
        })
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &BringAllToFront, cx| crate::background::activate(cx));
    cx.on_action(|_: &Minimize, cx| in_front_window(cx, Window::minimize_window));
    cx.on_action(|_: &Zoom, cx| in_front_window(cx, Window::zoom_window));
    cx.on_action(|_: &ToggleFullScreen, cx| in_front_window(cx, Window::toggle_fullscreen));
    cx.on_action(|_: &ShowLogs, cx| cx.open_with_system(&crate::logging::dir()));
    cx.on_action(|_: &ShowIntro, cx| {
        cx.defer(|cx| {
            AppState::with_front_workspace(cx, |ws, window, cx| {
                crate::background::activate_window(window, cx);
                ws.show_intro(window, cx);
            });
        });
    });
    cx.on_action(|_: &PlayIntroEmerge, cx| play_intro_in_front(Style::Emerge, cx));
    cx.on_action(|_: &PlayIntroVortex, cx| play_intro_in_front(Style::Vortex, cx));
    cx.on_action(|_: &PlayIntroCollapse, cx| play_intro_in_front(Style::Collapse, cx));
    cx.on_action(|_: &PlayIntroFtl, cx| play_intro_in_front(Style::Ftl, cx));
    cx.on_action(|_: &PlayIntroAurora, cx| play_intro_in_front(Style::Aurora, cx));
    cx.on_action(|_: &PlayIntroTentacles, cx| play_intro_in_front(Style::Tentacles, cx));
    cx.on_action(|_: &PlayIntroVoyage, cx| play_intro_in_front(Style::Voyage, cx));
    cx.on_action(|_: &PlayWelcomeCursor, cx| {
        crate::workspace::welcome::replay_in_front(Handover::Cursor, cx)
    });
    cx.on_action(|_: &PlayWelcomePortal, cx| {
        crate::workspace::welcome::replay_in_front(Handover::Portal, cx)
    });
    cx.on_action(|_: &PlayWelcomeLogo, cx| crate::workspace::welcome::replay_in_front(Handover::Logo, cx));
    cx.on_action(|recent: &OpenRecent, cx| {
        let root = recent.0.clone();
        cx.defer(move |cx| menu_bar::run_command(MenuCommand::OpenProject(root), cx));
    });
    // A key that ran an action may have flipped a label or moved a check mark: bring the menus
    // up to date now rather than on the next tick (typing runs no action, so it costs nothing).
    cx.observe_keystrokes(|event, _, cx| {
        if event.action.is_some() {
            cx.defer(app_menu::sync);
        }
    })
    .detach();
}

/// Wormhole → Theme Studio (⌥⌘,): the Studio to the front, opened if need be. With the Studio
/// already in front it closes it, as its key always did.
fn theme_studio(cx: &mut App) {
    let in_front = app(cx)
        .studio_window
        .as_ref()
        .zip(cx.active_window())
        .is_some_and(|((studio, _), active)| studio.window_id() == active.window_id());
    if in_front { studio::toggle_studio(cx) } else { studio::open_studio(cx) }
}

/// The jump-to-waiting key now bound, as `jump_waiting_key` said it.
struct JumpKey(String);

impl Global for JumpKey {}

/// Bind `jump_waiting_key` (⌥⌘J unless the user recorded another) to Go to Waiting Terminal, and
/// again when the setting changes: the old key is unbound, so it goes back to whatever else has
/// it, and the menus are put up again to show the new one.
fn bind_jump_key(cx: &mut App) {
    let spec = settings(cx).jump_waiting_key.trim().to_string();
    let bound = cx.try_global::<JumpKey>().map(|key| key.0.clone());
    if bound.as_deref() == Some(spec.as_str()) {
        return;
    }
    let spellings = |text: &str| -> Vec<String> {
        let Ok(key) = text.parse::<KeySpec>() else { return Vec::new() };
        std::iter::once(key.clone()).chain(key.shifted_spelling()).map(|k| k.to_string()).collect()
    };
    let mut bindings = Vec::new();
    for text in bound.as_deref().map(spellings).unwrap_or_default() {
        let unbind = Box::new(Unbind(GoToWaiting.name().into()));
        bindings.extend(KeyBinding::load(&text, unbind, None, false, None, &DummyKeyboardMapper).ok());
    }
    for text in spellings(&spec) {
        match KeyBinding::load(&text, Box::new(GoToWaiting), None, false, None, &DummyKeyboardMapper) {
            Ok(binding) => bindings.push(binding),
            Err(err) => tracing::warn!("jump_waiting_key {text}: {err}"),
        }
    }
    cx.bind_keys(bindings);
    cx.set_global(JumpKey(spec));
    app_menu::rebuild(cx);
}

/// Play the intro `style` around the window in front (About's buttons, the keys): brought forward
/// first, as About may be in front of it. Here, not in `intro`, which only knows what it draws.
fn play_intro_in_front(style: Style, cx: &mut App) {
    cx.defer(move |cx| {
        AppState::with_front_workspace(cx, |_, window, cx| {
            crate::background::activate_window(window, cx);
            crate::intro::play(window, style, cx);
        });
    });
}

/// Run `f` on the window in front (any of ours, not only a terminal window), once the update
/// that dispatched the action is over.
fn in_front_window(cx: &mut App, f: fn(&Window)) {
    if let Some(handle) = cx.active_window() {
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, _| f(window));
        });
    }
}

/// Keys the window takes before the terminal or a binding sees them (but in a dialog): Esc while
/// a diff covers the terminal that has the keys, and a project action's key, wherever the focus is.
fn claim_window_key(keystroke: &Keystroke, window: &mut Window, cx: &mut App) -> bool {
    let escape = keystroke.key == "escape" && !keystroke.modifiers.modified();
    // A project key carries ⌘ or is an F-key (`validate_key`): typing never gets further.
    let m = &keystroke.modifiers;
    if !escape && !m.platform && !keystroke_spec(keystroke).is_function_key() {
        return false;
    }
    let id = window.window_handle().window_id();
    let workspace = cx
        .try_global::<AppState>()
        .and_then(|state| state.windows.iter().find(|w| w.handle.window_id() == id))
        .and_then(|w| w.workspace.upgrade());
    let Some(workspace) = workspace else { return false };
    if window.has_active_dialog(cx) {
        return false;
    }
    if escape {
        return workspace.update(cx, |ws, cx| ws.terminal_escape(window, cx));
    }
    let Some(action) = workspace.read(cx).project_action_for_key(keystroke, cx) else { return false };
    workspace.update(cx, |ws, cx| ws.run_project_action(&action, window, cx));
    true
}
