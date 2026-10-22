mod a11y;
mod active_profile;
mod auth;
mod background;
#[cfg(feature = "devctl")]
mod devctl;
mod dock;
mod fonts;
mod hotkey;
mod icons;
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
mod intro;
mod logging;
mod menubar;
mod palette;
mod perf;
mod platform;
mod scroller;
mod select;
mod settings;
mod style;
mod terminal_view;
mod theme_bridge;
mod traffic_lights;
mod vibrancy;
mod window_ops;
mod workspace;

use gpui_kit::*;
use wormhole_core::ipc::Command;

actions!(wormhole, [Quit, QuitAndEndAll]);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match wormhole_core::ipc::command(&args) {
        Command::Hook(rest) => return wormhole_core::ipc::run_cli(rest),
        Command::StatusLine => return wormhole_core::ipc::run_status_line_cli(),
        // The first-run welcome, a program in the first terminal: it talks to the app that started it.
        Command::Welcome(rest) => std::process::exit(wormhole_core::welcome::run(rest)),
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
    let migrated = wormhole_core::store::migrate_legacy_data();
    let _log_guard = logging::init();
    logging::install_panic_hook();
    match migrated {
        Ok(Some(dir)) => tracing::info!(dir = %dir.display(), "moved the data of an earlier version here"),
        Ok(None) => {}
        Err(err) => tracing::error!("couldn't move the data of an earlier version: {err:#}"),
    }
    // Files an earlier version (or the log appender) made with the umask's modes become
    // owner-only; on a thread of its own so a large folder doesn't hold up the launch.
    let _ = std::thread::Builder::new().name("private-data".into()).spawn(|| {
        wormhole_core::store::make_data_dir_private(&wormhole_core::store::data_dir());
    });
    let Some(hook_events) = wormhole_core::ipc::start_server() else {
        tracing::info!(
            "already running; asked the existing instance to activate, unless launched in one of its tabs"
        );
        return;
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");
    // Only the instance that runs: a second launch above has handed over and returned.
    if let Err(err) = wormhole_core::quit_signal::catch_sigterm() {
        tracing::warn!("SIGTERM will end the app without saving: {err}");
    }
    // The `wh` command for the terminals this app starts, pointing at the binary that runs now.
    // Before any window opens a terminal.
    wormhole_core::cli_tool::keep_current_at_launch();

    let application = gpui_kit::application().with_assets(icons::Assets);
    // From the launch on: a launch to open a file sends it before the windows are up.
    let opened = workspace::listen_for_opened(&application);
    application.run(move |cx| {
        // Before anything opens: a test instance in background mode never shows a Dock icon.
        background::start(cx);
        gpui_kit::init(cx);
        fonts::init(cx);
        // The profile Wormhole was left in: its files are the ones read from here on. With only
        // Default (no `profiles.json`) these are the data folder's own, where they have always been.
        let root = wormhole_core::profile::DataRoot::current();
        let (profiles, profiles_error) = wormhole_core::profile::ProfileList::load_or_recover(&root);
        active_profile::set(root.profile(profiles.active_id()));
        cx.set_global(settings::SettingsStore::load());

        // ⌘Q asks first while something runs; saving happens as the app goes away, whichever way
        // it quits. Deferred: the question opens in the front window, which may be the one
        // dispatching the key.
        cx.on_action(|_: &Quit, cx| cx.defer(workspace::request_quit));
        // The same, and the terminals the session hosts keep running end with it.
        cx.on_action(|_: &QuitAndEndAll, cx| cx.defer(workspace::request_quit_ending_hosts));
        #[cfg(any(feature = "inspector", debug_assertions))]
        inspector::init(cx);
        // The keys from the action table, before the menus (they show the bound keys).
        workspace::init_actions(cx);

        workspace::AppState::start(hook_events, root, profiles, profiles_error, cx);
        workspace::serve_opened(opened, cx);
        background::activate(cx);
    });
}
