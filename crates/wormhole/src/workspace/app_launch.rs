//! The app's launch: load what was saved, start the app-level tasks (the tick, the hook socket's
//! messages, the menu bar's commands, the system's display options) and open the windows.
//!
//! What the system says about how windows look is heard from here on too: its appearance (for Match
//! system), through every window's appearance observer, and the accessibility display options
//! (Reduce Transparency, Increase Contrast, Reduce Motion), through their change notification
//! and again whenever a window comes to the front (macOS is not relied on to tell). The
//! look reaches the settings store, which re-derives what it touches; Reduce Motion is GPUI's
//! own flag, which its animations follow (`app_look::sync_system_look`).
//!
//! Part of the app root (it goes to `app/`): it may use every surface, and nothing below the app
//! root may call into it.

use std::collections::HashMap;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc;
use wormhole_core::storage::paths::DataRoot;

use crate::settings::ParkedSettings;
use crate::workspace::app::{AppState, app_mut};
use crate::workspace::app_look::{LookOverride, apply_theme, sync_system_look};
use crate::workspace::app_tick::{dispatch_hook, handle_menu_command, tick};
use crate::workspace::window_lifecycle::{open_workspace_window, save_all, window_closed};
use crate::workspace::{app_menu, menu_bar, notifications, onboarding, profiles, updates};

const TICK: Duration = Duration::from_millis(400);

impl AppState {
    /// Load what was saved, start the app-level tasks and open the windows
    /// that were open last time (or one fresh window).
    pub(crate) fn start(
        hook_events: smol::channel::Receiver<ipc::Incoming>,
        root: DataRoot,
        profiles: ProfileList,
        profiles_error: Option<wormhole_core::config::profile::ProfileError>,
        cx: &mut App,
    ) {
        let paths = root.profile(profiles.active_id());
        if let Some(err) = &profiles_error {
            tracing::warn!("profiles.json was set aside and rebuilt: {err}");
        }
        upgrade_legacy_hooks(cx);
        cx.background_spawn(async { crate::platform::default_app::upgrade_legacy_default() }).detach();
        let profiles::ProfileData { store, tips, mut hub, usage, worktrees, last_panes, .. } =
            profiles::ProfileData::open(&paths);
        let saved = store
            .as_ref()
            .and_then(|s| {
                s.load().inspect_err(|err| tracing::error!("loading saved sessions failed: {err:#}")).ok()
            })
            .unwrap_or_default();
        let (next_id, next_window_id) = profiles::first_free_ids(&root, &profiles, store.as_ref());
        let launch_hosts = crate::workspace::hosting::scan_at_launch(&root, &profiles, store.as_ref());
        let (menu, menu_commands) = menu_bar::MenuState::new(store.as_ref());
        let updates = updates::Updates::new(Some(root.update_schedule_file()), store.as_ref());
        hub.ids_start_at(next_id);
        let (display_changed, display_options) = smol::channel::unbounded::<()>();
        let display_observer = crate::platform::look::observe_display_options(move || {
            let _ = display_changed.try_send(());
        });
        crate::workspace::open_in::OpenIn::install(cx);
        cx.set_global(ParkedSettings::new(profiles.active_id().clone()));
        cx.set_global(AppState {
            root,
            profiles,
            profiles_error: profiles_error.map(|err| err.to_string()),
            parked: Default::default(),
            store,
            tips,
            hub,
            menu,
            updates,
            settings_window: None,
            studio_window: None,
            about_window: None,
            licenses_window: None,
            trust_window: None,
            windows: Vec::new(),
            front: 0,
            quitting: false,
            ending_hosts: false,
            launch_hosts,
            quit_prompt: None,
            usage,
            pr: Default::default(),
            worktrees,
            last_panes,
            welcomes: Default::default(),
            first_welcome: None,
            adopted: HashMap::new(),
            next_id,
            next_window_id,
            applied_theme: None,
            ticks: 0,
            look_override: LookOverride::default(),
            _display_options: display_observer,
        });
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
        cx.on_app_quit(|cx| {
            crate::workspace::hosting::detach_all(cx);
            save_all(cx);
            ipc::remove_socket();
            async {}
        })
        .detach();
        cx.on_window_closed(window_closed).detach();
        notifications::init(cx);
        updates::init(cx);
        crate::workspace::saved_text::init(cx);

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
            let welcome = crate::workspace::welcome::first_launch_plan(cx);
            app_mut(cx).first_welcome = welcome;
            open_workspace_window(None, None, cx);
            app_mut(cx).first_welcome = None;
            match welcome {
                Some(plan) => crate::workspace::welcome::play_first(plan, cx),
                None => onboarding::start_if_first_launch(cx),
            }
        }
        for window in saved {
            open_workspace_window(Some(window), None, cx);
        }
        crate::workspace::hosting::adopt_orphans(cx);
        app_menu::sync(cx);
    }
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
