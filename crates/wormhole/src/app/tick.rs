//! What the app does on its tick, once whatever the number of windows, and with what arrives
//! between ticks: the hook socket's messages, the menu bar's commands, and the theme and the system's
//! look (kept in `app_state::look`, which the tick drives).
//!
//! Part of the app root (`app/`): it may use every surface, and nothing below the app
//! root may call into it.

use gpui_kit::*;
use wormhole_core::ipc;

use crate::app::{control, menus};
use crate::app_state::look::apply_theme;
use crate::app_state::{AppState, app_mut};
use crate::menubar::MenuCommand;
use crate::settings::{ParkedSettings, SettingsStore};
use crate::status_item;
use crate::updates::restart;
use crate::window::features::project;
use crate::window::window_lifecycle::open_workspace_window;
use crate::{notifications, updates};

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
    status_item::run_command(command, cx);
}

/// Once per tick, whatever the number of windows.
pub(super) fn tick(cx: &mut App) {
    // `kill` (SIGTERM): quit as ⌘Q does, without its question, so nothing unsaved is lost.
    if wormhole_core::platform::quit_signal::take() {
        tracing::info!("asked to quit (SIGTERM)");
        cx.quit();
        return;
    }
    let ticks = app_mut(cx).advance_tick();
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
    status_item::sync(cx);
    menus::sync(cx);
    updates::schedule_check(ticks, cx);
    restart::tick(cx);
    if ticks.is_multiple_of(5) {
        project::prune_projects(cx);
        if crate::fonts::poll_custom(cx) {
            cx.refresh_windows();
        }
    }
}
