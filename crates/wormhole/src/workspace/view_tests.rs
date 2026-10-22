//! Whole-window tests: a real `Workspace` (with real shells) in GPUI's test platform, opened
//! through the same `open_workspace_window` the app uses.
//!
//! Opening one reads and writes the data dir and starts shells, so each test runs in a child
//! process of the test binary with a scratch data dir and `/bin/cat` for a shell (see
//! [`isolate`]): the environment can't be changed from inside a test process, and a test must
//! never touch the user's real data.
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    App, Context, Focusable as _, Keystroke, SystemNotificationResponse, TestAppContext, Window, px, rems,
    size,
};
use wormhole_core::agents::AgentId;
use wormhole_core::config::InFront;
use wormhole_core::git::{ChangeKind, DiffSpec, FileChange};
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::sidebar::{Entry, SidebarNumbers, WindowSidebar};

use crate::settings::SettingsStore;
use crate::terminal_view::Status;
use crate::workspace::app::{AppState, OpenWindow, app};
use crate::workspace::picker::Picker;
use crate::workspace::status_bar::StatusItem;
use crate::workspace::window_lifecycle::open_workspace_window;
use crate::workspace::{RightTab, Workspace, notifications};

const CHILD: &str = "WORMHOLE_VIEW_TEST_CHILD";

/// Runs test `name` again in a child process with a scratch data dir and a harmless shell, and
/// returns `true` (the caller then just returns) once the child has passed. In the child it
/// returns `false` and the test body runs.
fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// [`isolate`] for a test of another module (`module_path!()` there).
pub(super) fn isolate_in(module_path: &str, name: &str) -> bool {
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
pub(super) fn start(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let record = WindowRecord {
        id: 1,
        sessions: (1..=sessions).map(|id| SessionRecord::new(id, None, None)).collect(),
        ..Default::default()
    };
    start_with(cx, record)
}

/// [`start`] with the window's sessions as `record` has them (a terminal's scrollback, say).
pub(super) fn start_with(cx: &mut TestAppContext, record: WindowRecord) -> OpenWindow {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // The test platform delivers notifications only to an app with an identity.
        cx.set_app_identity("app.getwormhole.terminal.test", "Wormhole");
        cx.set_global(SettingsStore::load());
        cx.set_global(AppState::for_test(100));
        // These tests were written for the numbers that follow the list; the ones that stay with the
        // terminal have their own (`stable_list_tests`), which switch back with `use_stable`.
        crate::settings::update_settings(cx, |settings| {
            settings.sidebar_numbers = SidebarNumbers::List;
        });
        crate::workspace::app_handlers::init(cx);
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
pub(super) fn drive<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> R {
    let out = cx.update(|cx| AppState::with_workspace(window, cx, f)).expect("the window is open");
    cx.run_until_parked();
    out
}

/// Draw the window once, as a frame would (not from inside an update of the workspace).
pub(super) fn draw(cx: &mut TestAppContext, window: &OpenWindow) {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.render_frame(cx)))
        .expect("the window is open");
}

pub(super) fn read<R>(cx: &mut TestAppContext, window: &OpenWindow, f: impl FnOnce(&Workspace) -> R) -> R {
    cx.update(|cx| window.workspace.read_with(cx, |ws, _| f(ws))).expect("the window is open")
}

/// Run `f` on the window itself (not its workspace).
fn with_window<R>(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    cx.update(|cx| window.handle.update(cx, |_, window, cx| f(window, cx))).expect("the window is open")
}

fn some_change() -> FileChange {
    FileChange {
        path: "src/lib.rs".into(),
        original_path: None,
        kind: ChangeKind::Modified,
        added: Some(1),
        removed: Some(0),
        lfs: false,
        submodule: None,
    }
}

/// Give the active session the panels that belong to it: an open diff, a branch picker and a
/// focused note.
fn open_session_panels(ws: &mut Workspace, cx: &mut Context<Workspace>) {
    let spec = DiffSpec { old: "HEAD".into(), new: None };
    ws.open_diff(PathBuf::from("/nonexistent"), some_change(), spec, Rc::default(), None, cx);
    ws.picker = Some(Picker::Compare);
    ws.focused_note = Some(2);
}

fn has_session_panels(ws: &Workspace) -> bool {
    ws.diff_view.is_some() || ws.picker.is_some() || ws.focused_note.is_some()
}

/// A view test reaches nothing of the person running it: its home (where its terminals start and
/// git reads its config), the agents' homes, a Wormhole it was started from.
#[gpui_kit::test]
fn a_view_test_keeps_to_its_scratch_dir(_: &mut TestAppContext) {
    if isolate("a_view_test_keeps_to_its_scratch_dir") {
        return;
    }
    let data = PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir"));
    let inside = |var: &str| std::env::var_os(var).map(PathBuf::from).is_some_and(|p| p.starts_with(&data));
    // HOME is also where a terminal without a folder starts.
    for var in ["HOME", "WORMHOLE_AGENT_HOME", "WORMHOLE_CLI_DIR"] {
        assert!(inside(var), "{var} is in the scratch dir");
    }
    assert!(wormhole_core::agents::integrations::home().starts_with(&data));
    for var in ["WORMHOLE_START_DIR", "WORMHOLE_DEVCTL", "WORMHOLE_SOCKET", "WORMHOLE_AUTH"] {
        assert!(std::env::var_os(var).is_none(), "{var} is not inherited");
    }
    // The working directory comes back with links resolved (/var is /private/var on a Mac).
    let real = data.canonicalize().expect("the scratch dir exists");
    assert!(std::env::current_dir().is_ok_and(|dir| dir.starts_with(&real)), "nor is the checkout");
}

#[gpui_kit::test]
fn a_window_opens_with_its_sessions(cx: &mut TestAppContext) {
    if isolate("a_window_opens_with_its_sessions") {
        return;
    }
    let window = start(cx, 2);
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 2);
        assert_eq!(ws.active, 0);
    });
}

#[gpui_kit::test]
fn closing_the_only_session_starts_the_new_one_clean(cx: &mut TestAppContext) {
    if isolate("closing_the_only_session_starts_the_new_one_clean") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| open_session_panels(ws, cx));
    read(cx, &window, |ws| assert!(ws.diff_view.is_some() && ws.picker.is_some()));

    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 1);
        assert_ne!(ws.sessions[0].id, 1, "a new session took its place");
        assert_eq!(ws.active, 0);
        assert!(!has_session_panels(ws), "the closed session's diff, picker and note focus are gone");
    });
}

#[gpui_kit::test]
fn closing_another_session_leaves_the_one_you_are_in_alone(cx: &mut TestAppContext) {
    if isolate("closing_another_session_leaves_the_one_you_are_in_alone") {
        return;
    }
    let window = start(cx, 3);
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    drive(cx, &window, |ws, _, cx| open_session_panels(ws, cx));

    // A session above it: the selection moves up with its session.
    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.active, ws.sessions[ws.active].id), (2, 0, 2));
        assert!(ws.diff_view.is_some() && ws.picker.is_some() && ws.focused_note == Some(2));
    });
    // One below it.
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    read(cx, &window, |ws| {
        assert_eq!((ws.sessions.len(), ws.active, ws.sessions[ws.active].id), (1, 0, 2));
        assert!(ws.diff_view.is_some() && ws.picker.is_some() && ws.focused_note == Some(2));
    });
    // The selected one itself: whatever came next starts fresh.
    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read(cx, &window, |ws| assert!(!has_session_panels(ws)));
}

#[gpui_kit::test]
fn cmd_e_shows_the_notes_tab_before_focusing_a_note(cx: &mut TestAppContext) {
    if isolate("cmd_e_shows_the_notes_tab_before_focusing_a_note") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| {
        ws.add_note(0, Some("remember this".into()), false, window, cx);
        ws.show_tab(RightTab::Changes, cx);
        ws.sessions[0].notes_collapsed = true;
        ws.focus_terminal(window, cx);
        assert!(ws.active_terminal().focus_handle(cx).is_focused(window), "the terminal has the keyboard");
    });

    drive(cx, &window, |ws, window, cx| ws.toggle_focus(window, cx));
    drive(cx, &window, |ws, window, cx| {
        assert!(ws.right_tab == RightTab::Notes, "the panel switched to the Notes tab");
        assert!(!ws.sessions[0].notes_collapsed, "and unfolded the section");
        assert_eq!(ws.focused_note, Some(0));
        let focus = ws.sessions[0].notes[0].state.read(cx).focus_handle(cx);
        assert!(focus.is_focused(window), "the note has the keyboard");
    });
}

#[gpui_kit::test]
fn a_new_note_and_a_clicked_note_are_shown_and_focused(cx: &mut TestAppContext) {
    if isolate("a_new_note_and_a_clicked_note_are_shown_and_focused") {
        return;
    }
    let window = start(cx, 1);
    // ⌥⌘N with History showing and the Notes section folded away.
    drive(cx, &window, |ws, _, cx| {
        ws.show_tab(RightTab::History, cx);
        ws.sessions[0].notes_collapsed = true;
    });
    drive(cx, &window, |ws, window, cx| ws.new_note(Some("first".into()), window, cx));
    drive(cx, &window, |ws, window, cx| {
        assert!(ws.right_tab == RightTab::Notes);
        assert!(!ws.sessions[0].notes_collapsed);
        assert_eq!((ws.sessions[0].notes.len(), ws.focused_note), (1, Some(0)));
        assert!(ws.sessions[0].notes[0].state.read(cx).focus_handle(cx).is_focused(window));
    });

    // Clicking the title of a folded note unfolds it and puts the cursor at the end.
    drive(cx, &window, |ws, window, cx| {
        ws.sessions[0].notes[0].collapsed = true;
        ws.focus_terminal(window, cx);
        ws.open_note(0, window, cx);
    });
    drive(cx, &window, |ws, window, cx| {
        let note = &ws.sessions[0].notes[0];
        assert!(!note.collapsed);
        let state = note.state.read(cx);
        assert!(state.focus_handle(cx).is_focused(window));
        assert_eq!(state.cursor(), "first".len(), "at the end of the text");
    });
}

#[gpui_kit::test]
fn clicking_a_notification_brings_its_terminal_forward(cx: &mut TestAppContext) {
    if isolate("clicking_a_notification_brings_its_terminal_forward") {
        return;
    }
    let window = start(cx, 2);
    // Terminal 2 is the one in front.
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));

    let click = |cx: &mut TestAppContext, tag: Click| {
        let tag = match tag {
            Click::Tab(tab) => notifications::tag(tab),
            Click::Foreign(tag) => tag.into(),
        };
        cx.simulate_system_notification_response(SystemNotificationResponse { tag, action_id: None });
        cx.run_until_parked();
    };
    // Someone else's notification: nothing moves.
    click(cx, Click::Foreign("gpui-component/notification/abc"));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));
    // The first terminal.
    click(cx, Click::Tab(1));
    read(cx, &window, |ws| assert_eq!(ws.active, 0));
    // A terminal that has been closed since: the app comes forward, nothing else happens.
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    click(cx, Click::Tab(999));
    read(cx, &window, |ws| assert_eq!(ws.active, 1));
}

/// What a notification was posted for.
enum Click {
    Tab(i64),
    /// A tag of someone else's (GPUI Component's own notifications, say).
    Foreign(&'static str),
}

/// An agent's hook for session 2 (a background session), as the hook socket delivers it.
pub(super) fn hook(
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

/// Banners while Wormhole is in front (the test window always is): the default there is nothing.
fn banners_in_front(cx: &mut TestAppContext) {
    cx.update(|cx| SettingsStore::update(cx, |s| s.notify_in_front = InFront::Banner));
}

#[gpui_kit::test]
fn a_background_sessions_notification_follows_its_agent(cx: &mut TestAppContext) {
    if isolate("a_background_sessions_notification_follows_its_agent") {
        return;
    }
    let window = start(cx, 2);
    banners_in_front(cx);
    let notices = |cx: &mut TestAppContext| -> Vec<(String, String, String)> {
        cx.delivered_system_notifications()
            .into_iter()
            .map(|n| (n.tag.to_string(), n.title.to_string(), n.body.to_string()))
            .collect()
    };
    let tag = notifications::tag(2).to_string();
    // Read when a notice is checked: an unnamed terminal is named after what runs in it,
    // so the first hook, which names the agent, renames it.
    let name = |cx: &mut TestAppContext| {
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sessions[1].name(cx).to_string()))
            .expect("the window is open")
    };

    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    let needs_you = format!("{} needs you", name(cx));
    assert_eq!(notices(cx), [(tag.clone(), needs_you, "Allow Bash?".to_string())]);
    // The agent finishes instead: the same tab's notification is replaced, not added to.
    hook(cx, &window, 2, "Stop", None);
    let finished = format!("{} finished", name(cx));
    assert_eq!(notices(cx), [(tag.clone(), finished, "Finished".to_string())]);
    // It starts on something else: "finished" is history.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    assert!(notices(cx).is_empty());
    assert_eq!(cx.dismissed_system_notifications(), [notifications::tag(2)]);

    // Looking at the session takes a pending notice down too.
    hook(cx, &window, 2, "Stop", None);
    assert_eq!(notices(cx).len(), 1);
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert!(notices(cx).is_empty());

    // The session in front, in a window that is: nothing is announced while you are looking at it.
    drive(cx, &window, |ws, window, cx| ws.activate(0, window, cx));
    assert!(drive(cx, &window, |_, window, _| window.is_window_active()), "the test window is in front");
    hook(cx, &window, 1, "Notification", Some("Allow?"));
    assert!(notices(cx).is_empty());
}

#[gpui_kit::test]
fn closing_a_terminal_takes_its_notification_down(cx: &mut TestAppContext) {
    if isolate("closing_a_terminal_takes_its_notification_down") {
        return;
    }
    let window = start(cx, 1);
    banners_in_front(cx);
    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    // The new terminal is the selected one; the hook is for the first, now in the back.
    hook(cx, &window, 1, "Stop", None);
    assert_eq!(cx.delivered_system_notifications().len(), 1);
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    assert!(cx.delivered_system_notifications().is_empty());
}

/// In front, nothing shows by default: the sidebar, the Dock badge, the menu bar item and ⌥⌘J
/// carry the status. A banner comes only when asked for.
#[gpui_kit::test]
fn in_front_a_background_terminal_posts_nothing_unless_banners_are_asked_for(cx: &mut TestAppContext) {
    if isolate("in_front_a_background_terminal_posts_nothing_unless_banners_are_asked_for") {
        return;
    }
    let window = start(cx, 2);
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    assert!(cx.delivered_system_notifications().is_empty(), "Wormhole is in front: nothing is posted");
    hook(cx, &window, 2, "Stop", None);
    assert!(cx.delivered_system_notifications().is_empty(), "finishing is not told either");
    banners_in_front(cx);
    hook(cx, &window, 2, "Notification", Some("Allow Bash?"));
    assert_eq!(cx.delivered_system_notifications().len(), 1, "asked for: the same event is a banner");
}

#[gpui_kit::test]
fn jump_goes_to_the_terminal_waiting_longest_then_the_next(cx: &mut TestAppContext) {
    if isolate("jump_goes_to_the_terminal_waiting_longest_then_the_next") {
        return;
    }
    let window = start(cx, 3);
    hook(cx, &window, 3, "Notification", Some("first"));
    std::thread::sleep(std::time::Duration::from_millis(5));
    hook(cx, &window, 2, "Notification", Some("second"));
    let jump = |cx: &mut TestAppContext| {
        cx.update(crate::workspace::ambient::jump_to_waiting);
        cx.run_until_parked();
        read(cx, &window, |ws| ws.sessions[ws.active].id)
    };
    assert_eq!(jump(cx), 3, "the one that has waited longest");
    assert_eq!(jump(cx), 2, "pressed again: the next one");
    assert_eq!(jump(cx), 3, "and round again");
}

/// The sidebar's list as text: bands as `[band]`, rows as their session id and ⌘ number.
fn listed(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    cx.update(|cx| {
        window.workspace.read_with(cx, |ws, cx| {
            let listing = ws.session_listing(cx);
            listing
                .entries
                .iter()
                .map(|entry| match *entry {
                    Entry::Band { .. } => "[band]".to_string(),
                    Entry::Row { session, number, .. } => {
                        let id = ws.sessions[session].id;
                        number.map_or_else(|| id.to_string(), |n| format!("{id}:{n}"))
                    }
                })
                .collect()
        })
    })
    .expect("the window is open")
}

#[gpui_kit::test]
fn the_sidebar_lists_groups_pins_and_folds(cx: &mut TestAppContext) {
    if isolate("the_sidebar_lists_groups_pins_and_folds") {
        return;
    }
    let window = start(cx, 3);
    draw(cx, &window);
    // A window that was never saved has no groups: the terminals are a card each, numbered top to
    // bottom.
    assert_eq!(listed(cx, &window), ["1:1", "2:2", "3:3"]);

    // A pinned terminal goes first, and ⌘1 follows it.
    drive(cx, &window, |ws, _, cx| ws.toggle_pin(2, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["3:1", "1:2", "2:3"]);
    drive(cx, &window, |ws, _, cx| ws.toggle_pin(2, cx));

    // Two terminals grouped stand where the first stood, under a head of their own.
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2, 3], cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]", "2:2", "3:3"]);

    // A folded group keeps its terminals out of the list and out of the numbering.
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]"]);
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["1:1", "[band]", "2:2", "3:3"]);

    let saved = read(cx, &window, |ws| ws.sidebar.view.clone());
    // What is saved with the window brings all of it back.
    let record = cx
        .update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx)))
        .expect("the window is open");
    let layout: serde_json::Value =
        serde_json::from_str(record.layout.as_deref().expect("a layout")).expect("layout JSON");
    let restored: WindowSidebar =
        serde_json::from_value(layout["sidebar"].clone()).expect("the sidebar's state");
    assert_eq!(restored, saved);
    assert_eq!(record.sessions.iter().map(|s| s.pinned).collect::<Vec<_>>(), [false, false, false]);
}

/// Press a key combination in the window, as the keyboard would (`cmd-1`).
pub(super) fn press(cx: &mut TestAppContext, window: &OpenWindow, keys: &str) {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    cx.update(|cx| window.handle.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx)))
        .expect("the window is open");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn command_digits_and_stepping_follow_what_the_sidebar_shows(cx: &mut TestAppContext) {
    if isolate("command_digits_and_stepping_follow_what_the_sidebar_shows") {
        return;
    }
    let window = start(cx, 4);
    let active = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions[ws.active].id);
    // The first two are a group, folded: its terminals are skipped, so ⌘1 is the third.
    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[1, 2], cx));
    drive(cx, &window, |ws, _, cx| ws.toggle_group_band(group, cx));
    draw(cx, &window);
    assert_eq!(listed(cx, &window), ["[band]", "3:1", "4:2"]);
    press(cx, &window, "cmd-1");
    assert_eq!(active(cx), 3, "⌘1 is the terminal its keycap is on");

    // Stepping goes between what is shown, past the folded ones, and wraps.
    press(cx, &window, "ctrl-cmd-down");
    assert_eq!(active(cx), 4);
    draw(cx, &window);
    press(cx, &window, "ctrl-cmd-down");
    assert_eq!(active(cx), 3, "the folded terminals are skipped, and it wraps");
}

#[gpui_kit::test]
fn a_hidden_sidebar_never_redraws_the_window_for_its_times(cx: &mut TestAppContext) {
    if isolate("a_hidden_sidebar_never_redraws_the_window_for_its_times") {
        return;
    }
    let window = start(cx, 2);
    drive(cx, &window, |ws, _, cx| ws.toggle_sidebar(cx));
    draw(cx, &window);
    // A finished agent's row would read "Finished · just now", then "· 1m ago", …
    hook(cx, &window, 2, "Stop", None);
    let workspace = window.workspace.upgrade().expect("the window is open");
    let redraws = Rc::new(std::cell::Cell::new(0));
    let _watch = cx.update(|cx| {
        let redraws = redraws.clone();
        cx.observe(&workspace, move |_, _| redraws.set(redraws.get() + 1))
    });
    for _ in 0..3 {
        drive(cx, &window, |ws, _, cx| ws.sidebar_tick(true, cx));
    }
    assert_eq!(redraws.get(), 0, "nothing on screen shows those times");
}

/// The status bar counts the agents of every window without its frame reading the others: GPUI
/// makes an entity read while a window draws that window's, so another window's workspace would
/// run its tick there, and lose it when that window closed.
#[gpui_kit::test]
fn the_status_bar_counts_other_windows_without_taking_them_over(cx: &mut TestAppContext) {
    if isolate("the_status_bar_counts_other_windows_without_taking_them_over") {
        return;
    }
    let first = start(cx, 1);
    cx.update(|cx| SettingsStore::update(cx, |s| s.status_bar = true));
    cx.update(|cx| {
        let record =
            WindowRecord { id: 2, sessions: vec![SessionRecord::new(9, None, None)], ..Default::default() };
        open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    let second =
        cx.update(|cx| app(cx).windows.iter().find(|w| w.id == 2).cloned()).expect("the second window");
    hook(cx, &second, 9, "UserPromptSubmit", None);
    drive(cx, &first, |ws, _, cx| ws.refresh_status_bar(cx));
    draw(cx, &second);
    draw(cx, &first);
    read(cx, &first, |ws| {
        assert!(ws.status_items().contains(&StatusItem::Working(9)), "the other window's agent is counted")
    });

    // The window that drew last goes; the other keeps its own window (and so its tick).
    cx.update(|cx| first.handle.update(cx, |_, window, _| window.remove_window()))
        .expect("the window is open");
    cx.run_until_parked();
    assert_eq!(own_window(cx, &second), Some(second.handle.window_id()));
}

/// A terminal opened with a shell profile keeps its key: through a restart of its shell (the
/// terminal keeps its id too), and in what the window saves, so a restored session runs the same
/// thing.
#[gpui_kit::test]
fn a_terminal_keeps_its_shell_profile_through_a_restart_and_a_save(cx: &mut TestAppContext) {
    if isolate("a_terminal_keeps_its_shell_profile_through_a_restart_and_a_save") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, |ws, window, cx| ws.new_session_with(Some("fish".into()), window, cx));
    let id = read(cx, &window, |ws| {
        let keys: Vec<Option<&str>> =
            ws.sessions.iter().map(|session| session.pane.shell.as_deref()).collect();
        assert_eq!(keys, [None, Some("fish")]);
        ws.active_session().id
    });
    drive(cx, &window, |ws, window, cx| {
        ws.on_terminal_event(id, crate::terminal_view::TerminalEvent::Restart, window, cx)
    });
    read(cx, &window, |ws| {
        let session = ws.active_session();
        assert_eq!(
            (session.id, session.pane.shell.as_deref()),
            (id, Some("fish")),
            "restarted with the same profile"
        );
    });
    let record = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.window_record(cx))).expect("open");
    let saved: Vec<Option<&str>> = record.sessions.iter().map(|session| session.shell.as_deref()).collect();
    assert_eq!(saved, [None, Some("fish")], "the key is what the store gets");

    drive(cx, &window, |ws, window, cx| ws.new_session_with(Some("/bin/bash".into()), window, cx));
    read(cx, &window, |ws| {
        assert_eq!(ws.sessions.len(), 3);
        assert_eq!(ws.active_session().pane.shell.as_deref(), Some("/bin/bash"));
    });
}

/// There are no tabs: each terminal is one row of the sidebar and the title row has its breadcrumb
/// and the split buttons, nothing else. Closing a terminal takes its row away.
#[gpui_kit::test]
fn each_terminal_gets_a_sidebar_row_and_no_tab_strip(cx: &mut TestAppContext) {
    if isolate("each_terminal_gets_a_sidebar_row_and_no_tab_strip") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let rows = |cx: &mut TestAppContext| {
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.session_listing(cx)))
            .expect("open")
            .entries
            .iter()
            .filter(|entry| matches!(entry, Entry::Row { .. }))
            .count()
    };
    assert_eq!(rows(cx), 1);
    drive(cx, &window, |ws, window, cx| {
        ws.new_session(window, cx);
        ws.new_session(window, cx);
    });
    draw(cx, &window);
    assert_eq!(rows(cx), 3, "⌘T twice: three rows");
    let mut drawn = gpui_kit::VisualTestContext::from_window(window.handle, cx);
    for selector in ["tab-label-0", "sidebar-tab-0-0"] {
        assert!(drawn.debug_bounds(selector).is_none(), "{selector}: tabs are gone");
    }
    drive(cx, &window, |ws, window, cx| ws.request_close_session(1, window, cx));
    assert_eq!(rows(cx), 2);
}

/// Which window GPUI runs `window`'s workspace in (its tick, deferred work and subscriptions).
fn own_window(cx: &mut TestAppContext, window: &OpenWindow) -> Option<gpui_kit::WindowId> {
    let workspace = window.workspace.upgrade().expect("the window is open");
    cx.update(|cx| cx.with_window(workspace.entity_id(), |window, _| window.window_handle().window_id()))
}

/// Settings shows the front window's grouping (Terminals & sidebar) and project without its
/// frame reading that window's workspace, which would otherwise tick in Settings and stop when
/// Settings closed.
#[gpui_kit::test]
fn closing_settings_leaves_the_window_in_front_its_tick(cx: &mut TestAppContext) {
    if isolate("closing_settings_leaves_the_window_in_front_its_tick") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| {
        crate::workspace::settings_page::open_settings(
            Some(crate::workspace::settings_page::Section::Sessions),
            cx,
        )
    });
    cx.run_until_parked();
    let (settings_window, _) = cx.update(|cx| app(cx).settings_window.clone()).expect("Settings opened");
    draw(cx, &window);
    cx.update(|cx| settings_window.update(cx, |_, window, cx| window.render_frame(cx))).expect("open");
    cx.update(|cx| settings_window.update(cx, |_, window, _| window.remove_window())).expect("open");
    cx.run_until_parked();
    assert_eq!(own_window(cx, &window), Some(window.handle.window_id()));
}

/// A devctl batch with `wait-idle` stops there until the window has settled, holds back the next
/// batch meanwhile, and ends with its `sync` tokens after everything else it printed.
#[cfg(feature = "devctl")]
#[gpui_kit::test]
fn a_devctl_batch_waits_for_idle_and_ends_with_its_sync_tokens(cx: &mut TestAppContext) {
    if isolate("a_devctl_batch_waits_for_idle_and_ends_with_its_sync_tokens") {
        return;
    }
    let window = start(cx, 2);
    let dir = PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir")).join("ctl");
    std::fs::create_dir_all(&dir).expect("creates the control dir");
    let (cmd, out) = (dir.join("cmd"), dir.join("out.txt"));
    let poll = |cx: &mut TestAppContext| {
        let dir = dir.clone();
        drive(cx, &window, move |ws, window, cx| crate::devctl::poll_dir(dir, ws, window, cx));
    };
    let script = "sync first\nreduce-motion on\nhook 2 Notification Allow Bash?\nhook 9 Stop\nwait-idle 5000\n\
                  reduce-motion\nsync second\n";
    std::fs::write(&cmd, script).expect("writes the batch");
    poll(cx);
    assert!(!out.exists(), "the batch waits for the window to settle");
    assert!(cx.update(|cx| cx.reduce_motion()), "what came before the wait ran");
    std::fs::write(&cmd, "state\n").expect("writes the next batch");
    poll(cx);
    assert!(cmd.exists(), "the next batch is not taken while one waits");

    for _ in 0..100 {
        if out.exists() {
            break;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(50));
        cx.run_until_parked();
    }
    let printed = std::fs::read_to_string(&out).expect("the batch finished");
    assert_eq!(
        printed,
        "hook failed: no terminal 9 in this window (`state` lists them)\nidle\nreduce-motion on\nsync first\nsync second\n"
    );
    let status = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sessions[1].status(cx)));
    assert_eq!(status.expect("the window is open"), Status::Attention, "the hook arrived");
    poll(cx);
    assert!(!cmd.exists(), "the next batch runs once the first is done");
}

/// `scroll` turns the wheel over a point the way a trackpad does, a frame between the steps, so
/// a long virtual list scrolls past the rows it had never measured, and back.
#[cfg(feature = "devctl")]
#[gpui_kit::test]
fn a_devctl_scroll_turns_the_wheel_over_a_point(cx: &mut TestAppContext) {
    if isolate("a_devctl_scroll_turns_the_wheel_over_a_point") {
        return;
    }
    let window = start(cx, 1);
    let dir = PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir")).join("ctl");
    std::fs::create_dir_all(&dir).expect("creates the control dir");
    let cmd = dir.join("cmd");
    let poll = |cx: &mut TestAppContext| {
        let dir = dir.clone();
        drive(cx, &window, move |ws, window, cx| crate::devctl::poll_dir(dir, ws, window, cx));
    };
    std::fs::write(&cmd, "task-seed 80 0\npanel-tab notes\n").expect("writes the batch");
    poll(cx);
    draw(cx, &window);
    let top = |cx: &mut TestAppContext| {
        let top = read(cx, &window, |ws| ws.devctl_notes_list().logical_scroll_top());
        (top.item_ix, f32::from(top.offset_in_item))
    };
    assert_eq!(top(cx), (0, 0.0));

    // Over the right panel, which ends at the window's edge.
    let x = f32::from(with_window(cx, &window, |window, _| window.viewport_size().width)) - 180.0;
    std::fs::write(&cmd, format!("scroll {x} 400 1500\n")).expect("writes the batch");
    poll(cx);
    let (row, _) = top(cx);
    assert!(row > 0, "the list scrolled, past the rows it had measured");
    std::fs::write(&cmd, format!("scroll {x} 400 -5000\n")).expect("writes the batch");
    poll(cx);
    assert_eq!(top(cx), (0, 0.0), "and back to the top");
}

/// With the debug Inspector open, GPUI lays the workspace out in the window less the Inspector's
/// panel: the right panel's edge, its handle and its fit follow that column, not the window.
#[cfg(any(feature = "inspector", debug_assertions))]
#[gpui_kit::test]
fn the_panels_edge_follows_the_workspaces_column_beside_the_inspector(cx: &mut TestAppContext) {
    if isolate("the_panels_edge_follows_the_workspaces_column_beside_the_inspector") {
        return;
    }
    let window = start(cx, 1);
    // Room for the panel next to the Inspector.
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    draw(cx, &window);
    let column = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.window_width);
    let mut drawn = gpui_kit::VisualTestContext::from_window(window.handle, cx);
    let mut handle_x =
        || f32::from(drawn.debug_bounds("panel-resize").expect("the handle is drawn").origin.x);
    let at_edge = 1800.0 - wormhole_core::view::layout::PANEL_DEFAULT - crate::style::RESIZE_HANDLE_W / 2.0;
    assert_eq!((column(cx), handle_x()), (1800.0, at_edge));

    with_window(cx, &window, |window, cx| window.toggle_inspector(cx));
    let inspector = with_window(cx, &window, |window, _| f32::from(rems(30.0).to_pixels(window.rem_size())));
    // The first frame measures the column; the next one is laid out with it.
    draw(cx, &window);
    draw(cx, &window);
    assert_eq!(column(cx), 1800.0 - inspector);
    assert_eq!(handle_x(), at_edge - inspector, "the handle sits on the panel's edge, not the Inspector's");

    with_window(cx, &window, |window, cx| window.toggle_inspector(cx));
    draw(cx, &window);
    draw(cx, &window);
    assert_eq!((column(cx), handle_x()), (1800.0, at_edge), "closing it gives the window back");
}

/// ⌘F opens the bar with the keyboard in its field, typing finds every match and picks the
/// newest, ⌘G / ⇧⌘G go round, and Esc closes it and gives the terminal the keyboard back.
#[gpui_kit::test]
fn find_opens_counts_steps_and_closes(cx: &mut TestAppContext) {
    if isolate("find_opens_counts_steps_and_closes") {
        return;
    }
    // A terminal restored with text in it: the test can't wait for a shell's output.
    let mut session = SessionRecord::new(1, None, None);
    session.scrollback = Some("one split, two split\nnothing\nthree split\nsplits".into());
    let window = start_with(cx, WindowRecord { id: 1, sessions: vec![session], ..Default::default() });
    draw(cx, &window);

    let find = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| ws.active_terminal()).read_with(cx, |t, _| t.find_summary())
    };
    press(cx, &window, "cmd-f");
    assert_eq!(find(cx), Some("\"\" ".to_string()), "open, nothing typed yet");
    let terminal_focused = |cx: &mut TestAppContext| {
        let terminal = read(cx, &window, |ws| ws.active_terminal());
        with_window(cx, &window, |window, cx| terminal.read(cx).focus_handle.is_focused(window))
    };
    assert!(!terminal_focused(cx), "the field has the keyboard");
    for key in ["s", "p", "l", "i", "t"] {
        press(cx, &window, key);
    }
    assert_eq!(find(cx), Some("\"split\" 4 of 4".to_string()), "the newest match first");
    press(cx, &window, "cmd-g");
    assert_eq!(find(cx), Some("\"split\" 1 of 4".to_string()), "round the end");
    press(cx, &window, "shift-cmd-g");
    press(cx, &window, "shift-cmd-g");
    assert_eq!(find(cx), Some("\"split\" 3 of 4".to_string()));
    draw(cx, &window);
    press(cx, &window, "escape");
    assert_eq!(find(cx), None, "closed");
    assert!(terminal_focused(cx), "the terminal has the keyboard again");
}

#[gpui_kit::test]
fn answering_the_agents_question_by_key_ends_needs_you(cx: &mut TestAppContext) {
    if isolate("answering_the_agents_question_by_key_ends_needs_you") {
        return;
    }
    let window = start(cx, 1);
    draw(cx, &window);
    let now = |cx: &mut TestAppContext| {
        let summary = read(cx, &window, |ws| ws.active_terminal()).read_with(cx, |t, _| t.summary());
        (summary.status, summary.agent_message)
    };
    // The key as the terminal reads it, without sending it to the shell (whose echo the test
    // scheduler can't wait for).
    let key = |cx: &mut TestAppContext, key: &str| {
        let terminal = read(cx, &window, |ws| ws.active_terminal());
        cx.update(|cx| terminal.update(cx, |t, _| t.answered(key, false)));
    };
    let asked = (Status::Attention, Some("Approve cargo test?".to_string()));

    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    assert_eq!(now(cx), asked);
    // The notification that the same prompt still waits keeps the prompt's own words.
    hook(cx, &window, 1, "Notification:permission_prompt", Some("Claude needs your permission to use Bash"));
    assert_eq!(now(cx), asked);
    // Moving through the choices answers nothing; Return does, and the tool runs.
    key(cx, "down");
    assert_eq!(now(cx), asked);
    key(cx, "enter");
    assert_eq!(now(cx), (Status::Busy, None));

    // Esc turns it down: the agent stops at its prompt, still reporting through its hooks.
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    key(cx, "escape");
    assert_eq!(now(cx).0, Status::Idle);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    assert_eq!(now(cx).0, Status::Busy);
}
