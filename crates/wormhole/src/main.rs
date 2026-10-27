#![cfg_attr(test, allow(clippy::disallowed_methods, reason = "tests build their own files and processes"))]
#![allow(
    unreachable_pub,
    reason = "a binary crate: a `pub` is crate-wide, and nothing outside can name it anyway"
)]

mod about;
mod actions;
mod active_profile;
mod agent_access;
mod app;
mod app_state;
mod automation;
mod aux_window;
mod background;
mod command_palette;
#[cfg(feature = "devctl")]
mod devctl;
mod dialog;
mod features;
mod focus_keys;
mod fonts;
mod hotkey;
mod icons;
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
mod intro;
mod menubar;
mod notice;
mod notifications;
mod onboarding;
mod open_in;
mod palette;
mod perf;
mod platform;
mod select;
mod settings;
mod settings_window;
mod shortcut_sheet;
mod sidebar;
mod status_item;
mod studio;
mod style;
mod terminal_view;
mod theme_bridge;
mod update;
mod window;
mod window_open;
mod window_ops;

use gpui_kit::*;
use wormhole_core::ipc::Command;

actions!(wormhole, [Quit, QuitAndEndAll]);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match wormhole_core::ipc::command(&args) {
        Command::Hook(rest) => return wormhole_core::ipc::run_cli(rest),
        Command::StatusLine => return wormhole_core::ipc::run_status_line_cli(),
        // The first-run welcome, a program in the first terminal: it talks to the app that started it.
        Command::Welcome(rest) => std::process::exit(wormhole_core::onboarding::welcome::run(rest)),
        // `Wormhole list`, `send-text`, …: a round trip to the running app.
        Command::Control(rest) => std::process::exit(wormhole_core::ipc::run_control_cli(rest)),
        // Most likely a command a newer build installed into an agent's settings: a launch would
        // bring the running app to the front every time it runs.
        Command::Unknown(command) => {
            #[allow(clippy::print_stderr, reason = "a command line's answer to an unknown command")]
            {
                eprintln!("Wormhole: unknown command {command:?}");
            }
            std::process::exit(2);
        }
        Command::App => {}
    }
    // Before anything opens a file in the data folder: a version from before the rename kept it
    // under the old codename.
    let migrated = wormhole_core::storage::store::migrate_legacy_data();
    let _log_guard = app::logging::init();
    app::logging::install_panic_hook();
    match migrated {
        Ok(Some(dir)) => tracing::info!(dir = %dir.display(), "moved the data of an earlier version here"),
        Ok(None) => {}
        Err(err) => tracing::error!("couldn't move the data of an earlier version: {err:#}"),
    }
    // Files an earlier version (or the log appender) made with the umask's modes become
    // owner-only; on a thread of its own so a large folder doesn't hold up the launch.
    let _ = std::thread::Builder::new().name("private-data".into()).spawn(|| {
        wormhole_core::storage::store::make_data_dir_private(&wormhole_core::storage::paths::data_dir());
    });
    let Some(served) = wormhole_core::ipc::start_server() else {
        tracing::info!(
            "already running; asked the existing instance to activate, unless launched in one of its tabs"
        );
        return;
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");
    // Only the instance that runs: a second launch above has handed over and returned.
    if let Err(err) = wormhole_core::platform::quit_signal::catch_sigterm() {
        tracing::warn!("SIGTERM will end the app without saving: {err}");
    }
    // The `wh` command for the terminals this app starts, pointing at the binary that runs now.
    // Before any window opens a terminal.
    wormhole_core::terminal::cli_tool::keep_current_at_launch();

    let application = gpui_kit::application().with_assets(icons::Assets);
    // From the launch on: a launch to open a file sends it before the windows are up.
    let opened = app::default_terminal::listen(&application);
    application.run(move |cx| {
        // Before anything opens: a test instance in background mode never shows a Dock icon.
        background::start(cx);
        gpui_kit::init(cx);
        // Before any window: the words it draws are in this language from the first frame.
        app::language::init();
        style::menu_layer::install(cx);
        fonts::init(cx);
        // The profile Wormhole was left in: its files are the ones read from here on. With only
        // Default (no `profiles.json`) these are the data folder's own, where they have always been.
        let root = wormhole_core::storage::paths::DataRoot::current();
        let (profiles, profiles_error) = wormhole_core::config::profile::ProfileList::load_or_recover(&root);
        active_profile::set(root.profile(profiles.active_id()));
        cx.set_global(settings::SettingsStore::load());
        #[cfg(feature = "devctl")]
        devctl::install(cx);

        // ⌘Q asks first while something runs; saving happens as the app goes away, whichever way
        // it quits. Deferred: the question opens in the front window, which may be the one
        // dispatching the key.
        cx.on_action(|_: &Quit, cx| cx.defer(window::request_quit));
        // The same, and the terminals the session hosts keep running end with it.
        cx.on_action(|_: &QuitAndEndAll, cx| cx.defer(window::request_quit_ending_hosts));
        #[cfg(any(feature = "inspector", debug_assertions))]
        inspector::init(cx);
        // The keys from the action table, before the menus (they show the bound keys).
        app::handlers::init(cx);

        app::launch::start(served, root, profiles, profiles_error, cx);
        app::default_terminal::serve(opened, cx);
        background::activate(cx);
    });
}
