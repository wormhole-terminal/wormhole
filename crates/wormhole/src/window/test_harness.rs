//! The harness of the whole-window tests: a real `Workspace` (with real shells) in GPUI's test platform,
//! opened through the same `open_workspace_window` the app uses.
//!
//! Opening one reads and writes the data dir and starts shells, so each test runs in a child
//! process of the test binary with a scratch data dir and `/bin/cat` for a shell (see
//! [`isolate_in`]): the environment can't be changed from inside a test process, and a test must
//! never touch the user's real data. The cases are in the `*_tests` files beside it.

use std::process::Command;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, Context, Keystroke, TestAppContext, Window};
use wormhole_core::agents::AgentId;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::sidebar::SidebarNumbers;

use crate::app_state::{AppState, OpenWindow, app};
use crate::notifications;
use crate::settings::SettingsStore;
use crate::window::Workspace;
use crate::window::window_lifecycle::open_workspace_window;

const CHILD: &str = "WORMHOLE_VIEW_TEST_CHILD";

/// [`isolate`] for a test of another module (`module_path!()` there).
pub(crate) fn isolate_in(module_path: &str, name: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    // The test's path as the harness names it: without the crate's name.
    let module = module_path.split_once("::").map_or("", |(_, rest)| rest);
    let test = format!("{module}::{name}");
    let data = std::env::temp_dir().join(format!("wormhole-view-test-{}-{name}", std::process::id()));
    // A home of its own too: terminals start there, git reads its config from there, and Settings
    // looks for the agents' configs under it.
    let home = data.join("home");
    std::fs::create_dir_all(&home).expect("creates the scratch data dir");
    let exe = std::env::current_exe().expect("finds the test binary");
    let mut command = Command::new(exe);
    command
        .args([&test, "--exact", "--test-threads=1"])
        .current_dir(&data)
        .env(CHILD, "1")
        .env("HOME", &home)
        .env("WORMHOLE_DATA_DIR", &data)
        .env("WORMHOLE_SHELL", "/bin/cat")
        .env("WORMHOLE_AGENT_HOME", data.join("agent-home"))
        // Where Settings → Command line tool would put `wh`, and what it reads to say whether it is
        // there: never /usr/local/bin, never the person's shell.
        .env("WORMHOLE_CLI_DIR", data.join("cli-bin"));
    // Run from a Wormhole tab (or its dev instance), the tests would otherwise find its socket,
    // its devctl folder (and take the commands meant for it), its scripted answers and where it
    // starts terminals.
    for var in [
        "WORMHOLE_START_DIR",
        "WORMHOLE_DEVCTL",
        "WORMHOLE_SOCKET",
        "WORMHOLE_TAB_ID",
        "WORMHOLE_BIN",
        // A tab's variables under their old names, which the app reads when the new ones are missing.
        "ANDROMEDA_SOCKET",
        "ANDROMEDA_TAB_ID",
        "ANDROMEDA_BIN",
        "WORMHOLE_AUTH",
        "WORMHOLE_NOTIFY",
    ] {
        command.env_remove(var);
    }
    let output = command.output().expect("runs the test binary");
    std::fs::remove_dir_all(&data).ok();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // A filter that matched nothing also exits successfully: make sure the test really ran.
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{test} failed in its isolated process:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// An app with one window of `sessions` sessions (terminals), ids 1..
pub(crate) fn start(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let record = WindowRecord {
        id: 1,
        sessions: (1..=sessions).map(|id| SessionRecord::new(id, None, None)).collect(),
        ..Default::default()
    };
    start_with(cx, record)
}

/// The globals every window reads besides the settings: the shared state, the updater and the feature
/// table, all of which `app::launch` sets. A test that opens a window sets them here, so none can lack
/// one and a test window has the features the app's has ([`start_with_table`] swaps the table).
pub(crate) fn set_shared_state(cx: &mut App) {
    cx.set_global(AppState::for_test(100));
    cx.set_global(crate::update::Updates::new(None, None));
    crate::features::install(cx);
    crate::window::feature::FeatureTable::profile_shown(cx);
}

/// [`start`] with the window's sessions as `record` has them (a terminal's scrollback, say).
pub(crate) fn start_with(cx: &mut TestAppContext, record: WindowRecord) -> OpenWindow {
    start_with_table(cx, record, crate::features::TABLE)
}

/// [`start_with`] for an app whose feature table is `table`: what the window copies from the global
/// the app installs at launch.
pub(crate) fn start_with_table(
    cx: &mut TestAppContext,
    record: WindowRecord,
    table: &'static [crate::window::feature::FeatureDef],
) -> OpenWindow {
    // The terminals run real programs, whose reader threads wake the terminal's event task when the
    // program prints (an action's command typed into a new terminal is echoed at once). GPUI's test
    // scheduler reports a wake from another thread as a non-deterministic test, unless the test allowed
    // parking once; forbidding it again keeps the clock deterministic (idle waits jump to the next
    // timer) and leaves only the thread check off. The reader is the one thread that can wake it.
    cx.executor().allow_parking();
    cx.executor().forbid_parking();
    cx.update(|cx| {
        gpui_kit::init(cx);
        // The test platform delivers notifications only to an app with an identity.
        cx.set_app_identity("app.getwormhole.terminal.test", "Wormhole");
        // Layout tests measure rows and then press them: a list that slides to a new order for 180 ms
        // of the wall clock (not the test clock) would be read halfway, and pressed somewhere else.
        cx.set_reduce_motion(true);
        cx.set_global(SettingsStore::load());
        set_shared_state(cx);
        crate::window::feature::FeatureTable::install(table, cx);
        // These tests were written for the numbers that follow the list; the ones that stay with the
        // terminal have their own (`stable_list_tests`), which switch back with `use_stable`.
        crate::settings::update_settings(cx, |settings| {
            settings.sidebar_numbers = SidebarNumbers::List;
        });
        crate::app::handlers::init(cx);
        notifications::init(cx);
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    cx.update(|cx| app(cx).windows.first().cloned()).expect("the window opened")
}

/// The numbers that stay with a terminal (the default).
pub(super) fn use_stable(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.sidebar_numbers = SidebarNumbers::Terminal;
        });
    });
}

/// Run `f` on the window's workspace, then let everything it set in motion finish.
pub(crate) fn drive<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> R {
    let out = cx.update(|cx| AppState::with_workspace(window, cx, f)).expect("the window is open");
    cx.run_until_parked();
    out
}

/// Draw the window once, as a frame would (not from inside an update of the workspace).
pub(crate) fn draw(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.render_frame(cx)))
        .expect("the window is open");
}

/// Draw the window once as the display would: views GPUI keeps are replayed unless they were notified.
/// [`draw`] drops every cache first, which is what most tests want and a test of what a frame leaves
/// alone cannot have.
pub(crate) fn draw_replaying(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)))
        .expect("the window is open");
}

/// Read the window's sidebar (its list's state), which is an entity of its own.
pub(crate) fn sidebar_of<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&crate::sidebar::Sidebar) -> R,
) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| f(ws.sidebar.read(cx))))
        .expect("the window is open")
}

/// [`read`] with the app too, for what lives in an entity beside the workspace.
pub(crate) fn read_cx<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&Workspace, &App) -> R,
) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, cx| f(ws, cx))).expect("the window is open")
}

pub(crate) fn read<R>(cx: &mut TestAppContext, window: &OpenWindow, f: impl FnOnce(&Workspace) -> R) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, _| f(ws))).expect("the window is open")
}

/// Run `f` on the window itself (not its workspace).
pub(crate) fn with_window<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| f(window, cx))).expect("the window is open")
}

/// An agent's hook for session 2 (a background session), as the hook socket delivers it.
pub(crate) fn hook(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    tab: i64,
    event: &str,
    message: Option<&str>,
) {
    drive(cx, window, |ws, window, cx| {
        ws.handle_hook(&tab.to_string(), AgentId::Claude, event, message.map(str::to_string), window, cx)
    });
}

/// Press a key combination in the window, as the keyboard would (`cmd-1`).
pub(crate) fn press(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx)))
        .expect("the window is open");
    cx.run_until_parked();
}

/// Which window GPUI runs `window`'s workspace in (its tick, deferred work and subscriptions).
pub(crate) fn own_window(cx: &mut TestAppContext, window: &OpenWindow) -> Option<gpui_kit::WindowId> {
    let workspace = window.workspace.upgrade().expect("the window is open");
    cx.update(|cx| cx.with_window(workspace.entity_id(), |window, _| window.window_handle().window_id()))
}
