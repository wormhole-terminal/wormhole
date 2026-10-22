//! What the app does on its tick, once whatever the number of windows, and with what arrives
//! between ticks: the hook socket's messages, the menu bar's commands, the theme and the system's look.
//!
//! Part of the app root (it goes to `app/`): it may use every surface, and nothing below the app
//! root may call into it.

use gpui_kit::*;
use wormhole_core::ipc;

use crate::menubar::MenuCommand;
use crate::platform::vibrancy;
use crate::settings::SystemLook;
use crate::settings::{ParkedSettings, SettingsStore};
#[cfg(feature = "devctl")]
use crate::workspace::app::LookOverride;
use crate::workspace::app::{AppState, app, app_mut};
use crate::workspace::window_lifecycle::open_workspace_window;
use crate::workspace::{app_menu, control, menu_bar, notifications, project, update_restart, updates};

/// What the socket delivered: a hook's message, or a control request to answer (`control`).
pub(super) fn dispatch_hook(incoming: ipc::Incoming, cx: &mut App) {
    let message = match incoming {
        ipc::Incoming::Message(message) => message,
        ipc::Incoming::Control { request, reply } => {
            // The connection waits on its thread for this; a receiver that gave up is no error.
            let _ = reply.send(control::handle(request, cx));
            return;
        }
    };
    match message {
        // A request never comes this way: the server answers it (above) instead of forwarding it.
        ipc::Message::Control(_) => {}
        ipc::Message::Activate => {
            crate::background::activate(cx);
            match AppState::front_window(cx) {
                Some(window) => {
                    let _ = window
                        .handle
                        .update(cx, |_, window, cx| crate::background::activate_window(window, cx));
                }
                None => open_workspace_window(None, None, cx),
            }
        }
        ipc::Message::Usage { usage } => {
            app_mut(cx).record_usage(usage);
            // Every window's Usage tab shows it.
            cx.refresh_windows();
        }
        // The terminal's own: its model and context window, for its title row.
        ipc::Message::Gauge { tab, gauge } => {
            let Some(owner) = AppState::find_window(cx, |ws, _| ws.owns_terminal(&tab)) else { return };
            AppState::with_workspace(&owner, cx, |ws, _, cx| ws.handle_gauge(&tab, gauge, cx));
        }
        ipc::Message::Hook { tab, agent, event, message } => {
            let Some(owner) = AppState::find_window(cx, |ws, _| ws.owns_terminal(&tab)) else { return };
            AppState::with_workspace(&owner, cx, |ws, window, cx| {
                ws.handle_hook(&tab, agent, &event, message, window, cx)
            });
        }
    }
}

pub(super) fn handle_menu_command(command: MenuCommand, cx: &mut App) {
    tracing::debug!(?command, "menu command");
    menu_bar::run_command(command, cx);
}

/// Once per tick, whatever the number of windows.
pub(super) fn tick(cx: &mut App) {
    // `kill` (SIGTERM): quit as ⌘Q does, without its question, so nothing unsaved is lost.
    if wormhole_core::platform::quit_signal::take() {
        tracing::info!("asked to quit (SIGTERM)");
        cx.quit();
        return;
    }
    app_mut(cx).ticks += 1;
    let ticks = app(cx).ticks;
    let mut moved = SettingsStore::poll(cx);
    if cx.try_global::<ParkedSettings>().is_some_and(|parked| !parked.is_empty()) {
        let system = cx.global::<SettingsStore>().system();
        moved |= cx.global_mut::<ParkedSettings>().follow(system);
    }
    if moved {
        cx.refresh_windows();
    }
    apply_theme(cx);
    notifications::tick(ticks, cx);
    menu_bar::sync(cx);
    app_menu::sync(cx);
    updates::schedule_check(ticks, cx);
    update_restart::tick(cx);
    if ticks.is_multiple_of(5) {
        project::prune_projects(cx);
        if crate::fonts::poll_custom(cx) {
            cx.refresh_windows();
        }
    }
}

/// Push the resolved theme to GPUI Kit (its theme is global, the settings and
/// Studio windows use it too), and the theme's appearance to every window's material.
pub(super) fn apply_theme(cx: &mut App) {
    let store = cx.global::<SettingsStore>();
    let revision = store.revision;
    if app(cx).applied_theme == Some(revision) {
        return;
    }
    let (tokens, fonts) = (store.tokens.clone(), store.fonts.clone());
    crate::theme_bridge::apply(&tokens, &fonts, cx);
    vibrancy::set_material_appearance(tokens.appearance);
    app_mut(cx).applied_theme = Some(revision);
    cx.refresh_windows();
}

/// Hand the settings store what the system says now about how windows look (or what devctl
/// says in its place): the appearance Match system follows, Reduce Transparency and Increase
/// Contrast; and set GPUI's Reduce Motion flag, which its animations hold still under.
pub(super) fn sync_system_look(cx: &mut App) {
    let known = cx.global::<SettingsStore>().system();
    let stand_in = app(cx).look_override;
    let look = SystemLook {
        appearance: stand_in
            .appearance
            .or_else(crate::platform::look::system_appearance)
            .unwrap_or(known.appearance),
        reduce_transparency: stand_in
            .reduce_transparency
            .unwrap_or_else(crate::platform::look::reduce_transparency),
        increase_contrast: stand_in
            .increase_contrast
            .unwrap_or_else(crate::platform::look::increase_contrast),
    };
    // GPUI Kit reads Reduce Motion once, at start, and stops following the system once the app
    // has set the flag itself (devctl does); reading it here keeps both cases in step.
    if let Some(still) = stand_in.reduce_motion.or_else(crate::platform::look::reduce_motion) {
        cx.set_reduce_motion(still);
    }
    if SettingsStore::set_system(cx, look) {
        apply_theme(cx);
    }
}

#[cfg(feature = "devctl")]
impl AppState {
    /// devctl's `appearance`, `reduce-transparency`, `increase-contrast` and `reduce-motion`: say
    /// this in place of the system (or follow it again), and apply it now.
    pub(crate) fn override_look(cx: &mut App, change: impl FnOnce(&mut LookOverride)) -> SystemLook {
        change(&mut app_mut(cx).look_override);
        sync_system_look(cx);
        cx.global::<SettingsStore>().system()
    }
}
