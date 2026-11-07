//! The control line over several windows: `new-window` plans a window Swift opens, and what is typed to,
//! read from or hooked for a terminal of another window reaches it.

use serde_json::{Value, json};
use wormhole_core::workspace::Action;

use super::support::{eventually, fixture, fixture_of_windows};
use crate::hook_server::HookEvent;
use crate::{Terminal, Workspace, WorkspaceConfig, WorkspaceEvent};

fn ok(reply: &Value) -> &Value {
    assert_eq!(reply["ok"], true, "{reply}");
    &reply["result"]
}

#[test]
fn a_workspace_that_is_not_among_windows_has_no_new_window() {
    let f = fixture("open");
    let reply = f.client().control(json!({"action": "new_window"}));
    assert_eq!(reply["error"]["code"], "unsupported");
}

#[test]
fn new_window_plans_the_ids_and_the_control_line_then_reaches_the_window_swift_opens() {
    let f = fixture_of_windows("open");
    let windows = f.windows.clone().expect("windows");
    let first = f.terminal();
    let mut client = f.client();
    let reply = client.control(json!({"action": "new_window", "folder": "/tmp", "activate": true}));
    let opened = ok(&reply);
    let (window, terminal) =
        (opened["window"].as_i64().expect("window"), opened["terminal"].as_i64().expect("terminal"));
    assert!(window > 1, "the first window took 1");
    assert_ne!(terminal, first);
    assert_eq!(opened["folder"], "/tmp");
    assert!(f.events().contains(&HookEvent::NewWindow {
        window,
        terminal,
        folder: Some("/tmp".into()),
        activate: true
    }));

    // What Swift does with that event: a workspace for the window, filled from the plan, its program started.
    let wakes = f.wakes.clone();
    let second = Workspace::in_window(
        windows,
        window,
        WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        },
        wakes.clone(),
    );
    let events = second.restore();
    assert!(events.contains(&WorkspaceEvent::Opened { id: terminal, cwd: Some("/tmp".into()) }));
    let shell =
        Terminal::start_for_tab(terminal, "/bin/cat".into(), Vec::new(), None, 80, 24, 8.0, 16.0, wakes)
            .expect("a terminal");
    second.register_terminal(terminal, shell);

    // The first window's server types into the second window's terminal.
    let target = json!({"kind": "terminal", "id": terminal});
    let typed = client
        .control(json!({"action": "send_text", "target": target, "text": "over there", "submit": true}));
    assert_eq!(ok(&typed)["tab"], terminal);
    let screen = second.terminal(terminal).expect("registered");
    eventually("the text on the other window's screen", || screen.screen_text().contains("over there"));
    // ... and its hooks reach that window's row.
    second.apply(Action::Select(terminal));
    assert!(second.sidebar().sections.iter().flat_map(|s| &s.rows).any(|row| row.id == terminal));
}

#[test]
fn new_window_runs_its_command_once_the_terminal_has_a_shell() {
    let f = fixture_of_windows("open");
    let windows = f.windows.clone().expect("windows");
    f.terminal();
    let reply = f.client().control(json!({"action": "new_window", "command": "echo hi"}));
    let terminal = ok(&reply)["terminal"].as_i64().expect("terminal");
    let wakes = f.wakes.clone();
    let second = Workspace::in_window(
        windows,
        ok(&reply)["window"].as_i64().expect("window"),
        WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        },
        wakes.clone(),
    );
    second.restore();
    let shell =
        Terminal::start_for_tab(terminal, "/bin/cat".into(), Vec::new(), None, 80, 24, 8.0, 16.0, wakes)
            .expect("a terminal");
    second.register_terminal(terminal, shell);
    let screen = second.terminal(terminal).expect("registered");
    eventually("the command typed", || screen.screen_text().contains("echo hi"));
}
