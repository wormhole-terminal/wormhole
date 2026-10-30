//! The app's launch: load what was saved, start the app-level tasks (the tick, the hook socket's
//! messages, the menu bar's commands, the system's display options) and open the windows.
//!
//! What the system says about how windows look is heard from here on too: its appearance (for Match
//! system), through every window's appearance observer, and the accessibility display options
//! (Reduce Transparency, Increase Contrast, Reduce Motion), through their change notification
//! and again whenever a window comes to the front (macOS is not relied on to tell). The
//! look reaches the settings store, which re-derives what it touches; Reduce Motion is GPUI's
//! own flag, which its animations follow (`app_state::look::sync_system_look`).
//!
//! Part of the app root (`app/`): it may use every surface, and nothing below the app
//! root may call into it.

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::front_last;

use crate::app::menus;
use crate::app::tick::{dispatch_hook, handle_menu_command, tick};
use crate::app_state::AppState;
use crate::app_state::look::{apply_theme, sync_system_look};
use crate::app_state::profiles;
use crate::settings::ParkedSettings;
use crate::status_item;
use crate::window::window_lifecycle::{
    open_first_workspace_window, open_workspace_window, save_all, window_closed,
};
use crate::{notifications, update};

const TICK: Duration = Duration::from_millis(400);

/// Load what was saved, start the app-level tasks and open the windows
/// that were open last time (or one fresh window).
pub(crate) fn start(
    served: ipc::Served,
    root: DataRoot,
    profiles: ProfileList,
    profiles_error: Option<wormhole_core::config::profile::ProfileError>,
    cx: &mut App,
) {
    let ipc::Served { events: hook_events, bus } = served;
    let paths = root.profile(profiles.active_id());
    if let Some(err) = &profiles_error {
        tracing::warn!("profiles.json was set aside and rebuilt: {err}");
    }
    // Before any window: they publish what happens to whoever watches, and nothing else is built
    // while nobody does.
    crate::automation::install(bus.clone(), cx);
    // Before any window: each copies the table when it is made.
    crate::features::install(cx);
    upgrade_legacy_hooks(cx);
    cx.background_spawn(async { crate::platform::default_app::upgrade_legacy_default() }).detach();
    let data = profiles::ProfileData::open(&paths);
    let mut saved = data
        .store
        .as_ref()
        .and_then(|s| {
            s.load().inspect_err(|err| tracing::error!("loading saved sessions failed: {err:#}")).ok()
        })
        .unwrap_or_default();
    // The window that was in front opens last, and so is in front again (each comes forward as it opens).
    let front = data.store.as_ref().and_then(|s| s.front_window().ok().flatten());
    front_last(&mut saved, front);
    let launch_hosts = crate::window::hosting::scan_at_launch(&root, &profiles, data.store.as_ref());
    let (menu, menu_commands) = status_item::MenuState::new();
    let (display_changed, display_options) = smol::channel::unbounded::<()>();
    let display_observer = crate::platform::look::observe_display_options(move || {
        let _ = display_changed.try_send(());
    });
    crate::open_in::OpenIn::install(cx);
    cx.set_global(ParkedSettings::new(profiles.active_id().clone()));
    // The updater's state is a global of its own, so the shared state does not name it.
    cx.set_global(update::Updates::new(Some(root.update_schedule_file()), data.store.as_ref()));
    cx.set_global(AppState::new(
        root,
        profiles,
        profiles_error.map(|err| err.to_string()),
        data,
        launch_hosts,
        display_observer,
    ));
    crate::hotkey::install(menu.sender(), cx);
    cx.set_global(menu);
    // The features that keep app-wide data for the shown profile read it now that its database is open.
    crate::window::feature::FeatureTable::profile_shown(cx);
    // A profile switch changes what the menus show; they follow it at once, not on the next tick.
    menus::follow_changes(cx);
    // Before the first frame, so windows don't open in the default colors (nor in the
    // dark pick when Match system and a light system ask for the light one).
    sync_system_look(cx);
    crate::platform::scroller::ScrollerPrefs::install(cx);
    apply_theme(cx);
    cx.spawn(async move |cx| {
        while display_options.recv().await.is_ok() {
            cx.update(sync_system_look);
        }
    })
    .detach();

    // Every way of quitting (⌘Q, the menu bar, the Dock, logging out, the updater's relaunch)
    // ends up here, before GPUI drops the windows and their terminals: the hosts are let go of
    // first, so those drops leave their programs running.
    cx.on_app_quit(move |cx| {
        // Streams say why they end, if they get the time before the process does.
        bus.close_all(wormhole_core::ipc::control::event::CloseReason::Quitting);
        crate::window::hosting::detach_all(cx);
        save_all(cx);
        ipc::remove_socket();
        async {}
    })
    .detach();
    cx.on_window_closed(window_closed).detach();
    // A person taking hold of a window's edge and letting go of it: heard on the spot, passed on from a task.
    let drags = cx.to_async();
    crate::platform::live_resize::watch(move |event| {
        drags
            .spawn({
                let drags = drags.clone();
                async move |_| drags.update(|cx| crate::window::scaling::live_resize_changed(event, cx))
            })
            .detach();
    });
    notifications::init(cx);
    update::init(cx);
    crate::app::saved_text::init(cx);

    cx.spawn(async move |cx| {
        while let Ok(message) = hook_events.recv().await {
            cx.update(|cx| dispatch_hook(message, cx));
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        while let Ok(command) = menu_commands.recv().await {
            cx.update(|cx| handle_menu_command(command, cx));
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(TICK).await;
            cx.update(tick);
        }
    })
    .detach();

    if saved.is_empty() {
        // With the terminal welcome on, the window's first terminal runs it and the run plays
        // around the window; otherwise the intro window opens over a plain terminal.
        let welcome = crate::app::welcome::first_launch_plan(cx);
        crate::onboarding::welcome::set_first_launch(welcome, cx);
        // The first run plays around this window, which it brings in itself: no system animation then.
        if welcome.is_some() || crate::app::welcome::first_run_pending(cx) {
            open_first_workspace_window(cx);
        } else {
            open_workspace_window(None, None, cx);
        }
        crate::onboarding::welcome::set_first_launch(None, cx);
        match welcome {
            Some(plan) => crate::app::welcome::play_first(plan, cx),
            None => crate::app::welcome::start_if_first_launch(cx),
        }
    }
    // Decided before the windows are handed over: a window saved in the Dock is not brought forward.
    let front = crate::window::window_lifecycle::front_to_restore(&saved);
    for window in saved {
        open_workspace_window(Some(window), None, cx);
    }
    // A window restored full screen comes forward as it opens, a windowed one once its first frame is
    // drawn (`window_open::reveal`, a task): so the one that was in front is brought forward by a task
    // queued after those, and lands last whatever the order of the others was.
    if let Some(front) = front {
        crate::window::window_lifecycle::bring_forward_later(front, cx);
    }
    crate::window::hosting::adopt_orphans(cx);
    menus::sync(cx);
}
/// Agent hooks an earlier version installed under the app's old codename are replaced with
/// today's, off the main thread: they keep working meanwhile (terminals still export the old
/// variables), and an agent without any is left alone.
fn upgrade_legacy_hooks(cx: &mut App) {
    cx.background_spawn(async move {
        let home = wormhole_core::agents::integrations::home();
        for (agent, result) in wormhole_core::agents::integrations::upgrade_legacy(&home) {
            match result {
                Ok(()) => tracing::info!(?agent, "replaced the hooks an earlier version installed"),
                Err(err) => {
                    tracing::warn!(?agent, "couldn't replace the hooks an earlier version installed: {err}")
                }
            }
        }
    })
    .detach();
}
