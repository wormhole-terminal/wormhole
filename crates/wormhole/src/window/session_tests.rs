//! A window's sessions through whole-window tests: opening with them, closing one and what goes with it, a
//! terminal's shell kept through a restart, and the window keeping its tick when Settings closes. The
//! harness is `test_harness`.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, Context, TestAppContext, Window};
use wormhole_core::git::{ChangeKind, DiffSpec, FileChange};

use crate::app_state::app;
use crate::features::git::changes::Changes;
use crate::features::git::picker::PickerHost as _;
use crate::features::notes::testing as notes;
use crate::window::Workspace;
use crate::window::test_harness::{draw, drive, isolate_in, own_window, read, read_cx, start};

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
fn open_session_panels(ws: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let spec = DiffSpec { old: "HEAD".into(), new: None };
    let (session, terminal) = (ws.active_session().id, ws.active_terminal());
    let changes = ws.feature::<Changes>().expect("in the table");
    changes.update(cx, |changes, cx| {
        changes.open_diff(
            session,
            PathBuf::from("/nonexistent"),
            some_change(),
            spec,
            Rc::default(),
            None,
            cx,
        );
        changes.picker_mut().toggle(session, &terminal, window, cx);
    });
    notes::set_focused_note(ws, Some(2), cx);
}

/// Whether the diff, the branch picker or a note focus is up.
fn has_session_panels(ws: &Workspace, cx: &App) -> bool {
    ws.covered(cx)
        || ws.feature::<Changes>().is_some_and(|c| c.read(cx).picker().is_open())
        || notes::focused_note(ws, cx).is_some()
}

/// Whether both the diff and the branch picker are up.
fn has_diff_and_picker(ws: &Workspace, cx: &App) -> bool {
    ws.covered(cx) && ws.feature::<Changes>().is_some_and(|c| c.read(cx).picker().is_open())
}

/// A view test reaches nothing of the person running it: its home (where its terminals start and
/// git reads its config), the agents' homes, a Wormhole it was started from.
#[gpui_kit::test]
fn a_view_test_keeps_to_its_scratch_dir(_: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_view_test_keeps_to_its_scratch_dir") {
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
    if isolate_in(module_path!(), "a_window_opens_with_its_sessions") {
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
    if isolate_in(module_path!(), "closing_the_only_session_starts_the_new_one_clean") {
        return;
    }
    let window = start(cx, 1);
    drive(cx, &window, open_session_panels);
    read_cx(cx, &window, |ws, cx| assert!(has_diff_and_picker(ws, cx)));

    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read_cx(cx, &window, |ws, cx| {
        assert_eq!(ws.sessions.len(), 1);
        assert_ne!(ws.sessions[0].id, 1, "a new session took its place");
        assert_eq!(ws.active, 0);
        assert!(!has_session_panels(ws, cx), "the closed session's diff, picker and note focus are gone");
    });
}

#[gpui_kit::test]
fn closing_another_session_leaves_the_one_you_are_in_alone(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "closing_another_session_leaves_the_one_you_are_in_alone") {
        return;
    }
    let window = start(cx, 3);
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    drive(cx, &window, open_session_panels);

    // A session above it: the selection moves up with its session.
    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read_cx(cx, &window, |ws, cx| {
        assert_eq!((ws.sessions.len(), ws.active, ws.sessions[ws.active].id), (2, 0, 2));
        assert!(has_diff_and_picker(ws, cx) && notes::focused_note(ws, cx) == Some(2));
    });
    // One below it.
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    read_cx(cx, &window, |ws, cx| {
        assert_eq!((ws.sessions.len(), ws.active, ws.sessions[ws.active].id), (1, 0, 2));
        assert!(has_diff_and_picker(ws, cx) && notes::focused_note(ws, cx) == Some(2));
    });
    // The selected one itself: whatever came next starts fresh.
    drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
    read_cx(cx, &window, |ws, cx| assert!(!has_session_panels(ws, cx)));
}

/// A terminal opened with a shell profile keeps its key: through a restart of its shell (the
/// terminal keeps its id too), and in what the window saves, so a restored session runs the same
/// thing.
#[gpui_kit::test]
fn a_terminal_keeps_its_shell_profile_through_a_restart_and_a_save(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_terminal_keeps_its_shell_profile_through_a_restart_and_a_save") {
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

/// Settings shows the front window's grouping (Terminals & sidebar) and project without its
/// frame reading that window's workspace, which would otherwise tick in Settings and stop when
/// Settings closed.
#[gpui_kit::test]
fn closing_settings_leaves_the_window_in_front_its_tick(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "closing_settings_leaves_the_window_in_front_its_tick") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| {
        crate::settings_window::open_settings(Some(crate::settings_window::Section::Sessions), cx)
    });
    cx.run_until_parked();
    let (settings_window, _) =
        cx.update(|cx| crate::app_state::live(&app(cx).settings_window)).expect("Settings opened");
    draw(cx, &window);
    cx.update(|cx| settings_window.update(cx, |_, window, cx| window.render_frame(cx))).expect("open");
    cx.update(|cx| settings_window.update(cx, |_, window, _| window.remove_window())).expect("open");
    cx.run_until_parked();
    assert_eq!(own_window(cx, &window), Some(window.handle.window_id()));
}
