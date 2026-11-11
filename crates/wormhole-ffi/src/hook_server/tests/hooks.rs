//! The hook socket: the single-instance lock, who may connect, and what an agent's hooks do to a row.

use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use wormhole_core::view::sidebar::RowState;

use super::support::{eventually, fixture};
use crate::hook_server::{HookError, HookEvent, HookServer};

#[test]
fn an_agents_hooks_are_a_terminals_status_in_the_sidebar() {
    let f = fixture("open");
    let id = f.agent_terminal();
    // Another terminal is in front: what finishes behind it waits for a look.
    f.terminal();
    let tab = id.to_string();
    let mut hook = f.client();
    hook.send(&json!({"kind": "hook", "tab": tab, "agent": "claude", "event": "UserPromptSubmit"}));
    eventually("the agent working", || f.row(id).is_some_and(|row| row.state == RowState::Working));
    hook.send(&json!({
        "kind": "hook", "tab": tab, "agent": "claude",
        "event": "PermissionRequest:Bash", "message": "Approve cargo test?"
    }));
    eventually("the agent asking", || {
        f.row(id).is_some_and(|row| {
            row.state == RowState::NeedsYou && row.message.as_deref() == Some("Approve cargo test?")
        })
    });
    hook.send(&json!({"kind": "hook", "tab": tab, "agent": "claude", "event": "Stop"}));
    eventually("the agent done", || f.row(id).is_some_and(|row| row.state == RowState::Finished));
    // Another terminal's hook, and one for a terminal there is not, change nothing and break nothing.
    hook.send(&json!({"kind": "hook", "tab": "99", "event": "Stop"}));
    hook.send(&json!({"kind": "hook", "tab": "not a number", "event": "Stop"}));
    assert!(f.wakes.count() > 0, "Swift was woken");
}

#[test]
fn a_codex_hook_marks_its_terminal_working_with_or_without_the_agent_in_front() {
    let f = fixture("open");
    // Codex's own events (`json_hooks::CODEX`): a prompt, a question, the tool that ran after the answer, the end.
    for (label, id) in [("in front", f.named_agent_terminal("codex")), ("not seen", f.terminal())] {
        let tab = id.to_string();
        let mut hook = f.client();
        hook.send(&json!({"kind": "hook", "tab": tab, "agent": "codex", "event": "UserPromptSubmit"}));
        eventually(&format!("codex working ({label})"), || {
            f.row(id).is_some_and(|row| row.state == RowState::Working)
        });
        hook.send(&json!({
            "kind": "hook", "tab": tab, "agent": "codex", "event": "PermissionRequest", "message": "Run ls?"
        }));
        eventually(&format!("codex asking ({label})"), || {
            f.row(id).is_some_and(|row| row.state == RowState::NeedsYou)
        });
        hook.send(&json!({"kind": "hook", "tab": tab, "agent": "codex", "event": "PostToolUse"}));
        eventually(&format!("codex working again ({label})"), || {
            f.row(id).is_some_and(|row| row.state == RowState::Working)
        });
    }
}

#[test]
fn a_usage_report_reaches_the_usage_tab() {
    let f = fixture("open");
    let mut client = f.client();
    client.send(&json!({"kind": "usage", "usage": {
        "agent": "claude", "five_hour": {"used": 25.5, "resets_at": 1_800_002_040_i64},
        "seven_day": null, "updated": 1_800_000_000_i64
    }}));
    eventually("the report", || {
        f.usage.reports().iter().any(|r| r.agent == "claude" && r.limits.iter().any(|l| l.used == 25.5))
    });
}

#[test]
fn the_socket_is_private_and_a_second_instance_must_not_take_it() {
    let f = fixture("open");
    let socket = f.server.socket_path();
    let mode = std::fs::metadata(&socket).expect("the socket").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "only this user");
    // A second server on the same data folder, in the same process or another: refused, and it asked the
    // first to come forward.
    let other = crate::Workspace::new(
        crate::WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        },
        f.wakes.clone(),
    );
    let refused = HookServer::start(
        other,
        f.usage.clone(),
        f.desk.clone(),
        f.wakes.clone(),
        Some(f.dir.display().to_string()),
    );
    assert!(matches!(refused, Err(HookError::AlreadyServed)), "{:?}", refused.err());
    eventually("the first being asked to come forward", || {
        f.server.drain_events().contains(&HookEvent::Activate)
    });
    // The first still answers on the same socket.
    assert_eq!(f.client().control(json!({"action": "list"}))["ok"], true);
    assert_eq!(f.server.socket_path(), socket);
}

#[test]
fn a_server_refuses_a_folder_that_is_not_the_experiments() {
    crate::isolate_for_tests();
    let wakes = std::sync::Arc::new(super::support::Wakes::default());
    let workspace = crate::Workspace::new(
        crate::WorkspaceConfig {
            home: None,
            numbers_follow_list: true,
            start_folder: None,
            pane_header_height: 0.0,
        },
        wakes.clone(),
    );
    let usage = crate::UsageService::in_dir(&wormhole_core::platform::paths::temp(), wakes.clone());
    let desk =
        crate::AccessDesk::new(None, Some("/tmp/Wormhole-swift-x".into()), wakes.clone()).expect("desk");
    let refused = HookServer::start(workspace, usage, desk, wakes, Some("/Users/x/Wormhole".into()));
    assert!(matches!(refused, Err(HookError::NotIsolated { .. })));
}

#[test]
fn hostile_lines_are_answered_or_dropped_and_the_server_goes_on() {
    let f = fixture("open");
    let mut client = f.client();
    // Not JSON, a control request this build can't read, an action nobody knows, and a version from elsewhere.
    client.send_raw(b"this is not json\n");
    client.send_raw(b"{\"kind\":\"control\",\"version\":1,\"action\":\"send_text\"}\n");
    let unreadable = client.line().expect("an answer to the unreadable request");
    assert_eq!(unreadable["error"]["code"], "bad_request");
    client.send_raw(b"{\"kind\":\"control\",\"version\":1,\"action\":\"fly\"}\n");
    assert_eq!(client.line().expect("an answer")["error"]["code"], "unsupported");
    client.send_raw(b"{\"kind\":\"control\",\"version\":99,\"action\":\"list\"}\n");
    assert_eq!(client.line().expect("an answer")["error"]["code"], "version");
    // A line over the cap closes the connection; nothing else is harmed.
    let mut big = f.client();
    let mut huge = vec![b'x'; 200_000];
    huge.push(b'\n');
    let _ = std::io::Write::write_all(big.writer(), &huge);
    assert_eq!(f.client().control(json!({"action": "list"}))["ok"], true);
}

#[test]
fn a_terminal_started_for_its_tab_carries_the_names_its_hooks_and_wh_need() {
    crate::isolate_for_tests();
    let wakes = std::sync::Arc::new(super::support::Wakes::default());
    let terminal =
        crate::Terminal::start_for_tab(7, "/usr/bin/env".into(), Vec::new(), None, 200, 50, 8.0, 16.0, wakes)
            .expect("a terminal");
    eventually("the environment", || terminal.screen_text().contains("WORMHOLE_BIN="));
    let screen = terminal.screen_text();
    let socket = wormhole_core::ipc::socket_path();
    assert!(screen.contains("WORMHOLE_TAB_ID=7"), "{screen}");
    assert!(screen.contains(&format!("WORMHOLE_SOCKET={}", socket.display())), "{screen}");
}

#[test]
fn the_executable_does_the_command_line_before_it_is_an_app() {
    use crate::cli_exit_code;
    let args = |words: &[&str]| words.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(cli_exit_code(args(&["Wormhole"])), None, "a launch is the app's");
    assert_eq!(cli_exit_code(args(&["Wormhole", "-psn_0_12345"])), None);
    assert_eq!(
        cli_exit_code(args(&["Wormhole", "bogus-verb"])),
        Some(2),
        "an unknown command never launches"
    );
}
