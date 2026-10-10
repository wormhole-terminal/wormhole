mod a11y;
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
mod select;
mod settings;
mod style;
mod terminal_view;
mod theme_bridge;
mod traffic_lights;
mod vibrancy;
mod workspace;

use gpui_kit::*;
use wormhole_core::ipc::Command;

actions!(wormhole, [Quit]);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match wormhole_core::ipc::command(&args) {
        Command::Hook(rest) => return wormhole_core::ipc::run_cli(rest),
        Command::StatusLine => return wormhole_core::ipc::run_status_line_cli(),
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
    match migrated {
        Ok(Some(dir)) => tracing::info!(dir = %dir.display(), "moved the data of an earlier version here"),
        Ok(None) => {}
        Err(err) => tracing::error!("couldn't move the data of an earlier version: {err:#}"),
    }
    let Some(hook_events) = wormhole_core::ipc::start_server() else {
        tracing::info!(
            "already running; asked the existing instance to activate, unless launched in one of its tabs"
        );
        return;
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");

    let application = gpui_kit::application().with_assets(icons::Assets);
    // From the launch on: a launch to open a file sends it before the windows are up.
    let opened = workspace::listen_for_opened(&application);
    application.run(move |cx| {
        // Before anything opens: a test instance in background mode never shows a Dock icon.
        background::start(cx);
        gpui_kit::init(cx);
        fonts::init(cx);
        cx.set_global(settings::SettingsStore::load());

        // ⌘Q asks first while something runs; saving happens as the app goes away, whichever way
        // it quits. Deferred: the question opens in the front window, which may be the one
        // dispatching the key.
        cx.on_action(|_: &Quit, cx| cx.defer(workspace::request_quit));
        #[cfg(any(feature = "inspector", debug_assertions))]
        inspector::init(cx);
        // The keys from the action table, before the menus (they show the bound keys).
        workspace::init_actions(cx);

        workspace::AppState::start(hook_events, cx);
        workspace::serve_opened(opened, cx);
        background::activate(cx);
    });
}
