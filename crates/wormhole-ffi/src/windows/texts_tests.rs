//! Restore terminal text after restart for terminals in the app's own process, a program's notification in its own
//! words and a failed command for the notification rules, and a focus report's redraw, all over real programs in real
//! PTYs and a database in a temp folder.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_core::view::sidebar::RowState;
use wormhole_core::workspace::Action;

use super::Windows;
use crate::notify::NoticeSourceRecord;
use crate::{Terminal, TerminalListener, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};

struct Quiet;

impl WorkspaceListener for Quiet {
    fn wake(&self) {}
}

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

fn window(windows: &Arc<Windows>, id: i64) -> Arc<Workspace> {
    let config = WorkspaceConfig {
        home: None,
        numbers_follow_list: true,
        start_folder: None,
        pane_header_height: 0.0,
    };
    Workspace::in_window(windows.clone(), id, config, Arc::new(Quiet))
}

fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A new terminal of `ws` running `/bin/sh -c script`.
fn run(ws: &Workspace, script: &str) -> (i64, Arc<Terminal>) {
    let events = ws.apply(Action::NewTerminal);
    let id = events
        .iter()
        .find_map(|event| match event {
            WorkspaceEvent::Opened { id, .. } => Some(*id),
            _ => None,
        })
        .expect("opened");
    let args = vec!["-c".to_string(), script.to_string()];
    let terminal =
        Terminal::start("/bin/sh".into(), args, 80, 24, 8.0, 16.0, Arc::new(Quiet)).expect("starts");
    ws.register_terminal(id, terminal.clone());
    (id, terminal)
}

fn saved_text(dir: &Path, window: i64, id: i64) -> Option<String> {
    let reopened = Windows::in_dir(dir).expect("opens");
    let ws = self::window(&reopened, window);
    ws.restore();
    ws.launch_facts(id).saved_text
}

#[test]
fn a_local_terminal_s_text_is_saved_restored_and_erased_when_cleared_or_not_to_be_kept() {
    crate::isolate_for_tests();
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = Windows::in_dir(dir.path()).expect("opens");
    let ws = window(&windows, 1);
    let (kept, first) = run(&ws, "echo kept-across-restart; sleep 30");
    let (cleared, second) = run(&ws, "echo cleared-with-cmd-k; sleep 30");
    let (private, third) = run(&ws, "echo not-to-be-kept; sleep 30");
    eventually("the output", || {
        first.screen_text().contains("kept-across")
            && second.screen_text().contains("cleared-with")
            && third.screen_text().contains("not-to-be")
    });
    assert!(windows.save(1, ws.clone(), None), "the window's rows are there to hold the texts");
    assert_eq!(ws.save_texts(), 3);
    assert!(saved_text(dir.path(), 1, kept).is_some_and(|text| text.contains("kept-across-restart")));

    // ⌘K erases what was saved at once; "Don't keep this terminal's text" too, and nothing of it is saved again.
    second.clear_history();
    assert_eq!(saved_text(dir.path(), 1, cleared), None);
    ws.apply(Action::SetForgetText { id: private, forget: true });
    assert!(windows.save(1, ws.clone(), None));
    assert_eq!(saved_text(dir.path(), 1, private), None);
    ws.save_texts();
    assert_eq!(saved_text(dir.path(), 1, private), None, "a terminal whose text is not kept is not saved");

    // Turning the setting off erases every text, and nothing is saved while it is off.
    assert_eq!(windows.erase_saved_text_if_off(), 0, "on: nothing goes");
    std::fs::write(dir.path().join("config.json"), r#"{"restore_terminal_text": false}"#).expect("config");
    assert!(windows.erase_saved_text_if_off() > 0);
    assert_eq!(saved_text(dir.path(), 1, kept), None);
    assert_eq!(ws.save_texts(), 0);
}

#[test]
fn a_programs_notification_and_a_failed_command_reach_the_notification_rules() {
    crate::isolate_for_tests();
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = Windows::in_dir(dir.path()).expect("opens");
    let ws = window(&windows, 1);
    // OSC 777 with a title, then the shell's marks for a command that ran and failed with 2.
    let script = r"printf '\033]777;notify;Build;all 42 tests passed\007'; printf '\033]133;C\007'; sleep 1; printf 'boom\n\033]133;D;2\007'; sleep 30";
    let (id, _terminal) = run(&ws, script);
    let facts = |ws: &Workspace| ws.notice_facts(1_000_000).into_iter().find(|facts| facts.id == id);
    eventually("the notification's words", || facts(&ws).is_some_and(|facts| facts.notice.is_some()));
    let notice = facts(&ws).and_then(|facts| facts.notice).expect("a notice");
    assert_eq!(
        (notice.serial, notice.title.as_str(), notice.body.as_str()),
        (1, "Build", "all 42 tests passed")
    );
    assert_eq!(ws.terminals().terminals.iter().find(|t| t.id == id).map(|t| t.notice), Some(1));
    eventually("the failure", || facts(&ws).is_some_and(|facts| facts.failure.is_some()));
    let found = facts(&ws).expect("facts");
    let failure = found.failure.expect("a failure");
    assert_eq!((failure.serial, failure.code), (1, 2));
    assert!(failure.at_ms <= 1_000_000, "on the caller's clock");
    assert_eq!((found.source, found.who), (NoticeSourceRecord::Plain, None));
}

/// A program that redraws when it hears the window went behind (DECSET 1004) is answering, not working: its row stays
/// as it was instead of turning Working and then "Done".
#[test]
fn a_redraw_after_a_focus_report_is_not_work() {
    crate::isolate_for_tests();
    let dir = tempfile::tempdir().expect("temp dir");
    let windows = Windows::in_dir(dir.path()).expect("opens");
    let ws = window(&windows, 1);
    let script = r"stty raw -echo; printf '\033[?1004hready\r\n'; while :; do dd bs=3 count=1 >/dev/null 2>&1; printf 'redraw\r\n'; done";
    let (id, terminal) = run(&ws, script);
    eventually("the program to be ready", || terminal.screen_text().contains("ready"));
    let state = |ws: &Workspace| {
        ws.sidebar().sections.into_iter().flat_map(|s| s.rows).find(|row| row.id == id).map(|row| row.state)
    };
    eventually("it to settle", || state(&ws) == Some(RowState::Idle));
    terminal.focus_changed(false);
    eventually("the redraw", || terminal.screen_text().contains("redraw"));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(state(&ws), Some(RowState::Idle), "the redraw answered the focus report");
}
