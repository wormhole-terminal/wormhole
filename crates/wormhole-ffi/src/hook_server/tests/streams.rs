//! `events` and `wait`: what the bus tells a stream and a waiter about the terminals.

use std::time::Duration;

use serde_json::json;

use super::support::fixture;

#[test]
fn an_event_stream_starts_with_the_listing_and_tells_the_status_changes() {
    let f = fixture("open");
    let id = f.terminal();
    let mut stream = f.client();
    let first = stream.control(json!({"action": "events", "topics": ["status", "terminals"]}));
    assert_eq!(first["ok"], true, "{first}");
    assert_eq!(first["result"]["listing"]["windows"][0]["terminals"][0]["id"], id);
    // An agent's hook moves the state; the stream tells it, with the ids `list` uses.
    let mut hook = f.client();
    hook.send(
        &json!({"kind": "hook", "tab": id.to_string(), "agent": "claude", "event": "UserPromptSubmit"}),
    );
    let told = loop {
        let line = stream.line().expect("an event line");
        if line["event"] == "status" {
            break line;
        }
    };
    assert_eq!(
        (told["terminal"].as_i64(), told["status"].as_str(), told["was"].as_str()),
        (Some(id), Some("working"), Some("idle"))
    );
    // A terminal opening is told too.
    let opened = f.terminal();
    let line = loop {
        let line = stream.line().expect("an event line");
        if line["event"] == "terminal_opened" {
            break line;
        }
    };
    assert_eq!(line["terminal"], opened);
}

#[test]
fn wait_answers_at_once_for_a_state_that_holds_and_later_for_one_that_comes() {
    let f = fixture("open");
    let id = f.terminal();
    let target = json!({"kind": "terminal", "id": id});
    let mut client = f.client();
    let held =
        client.control(json!({"action": "wait", "target": target, "status": ["idle"], "timeout_ms": 5000}));
    assert_eq!(held["ok"], true, "{held}");
    assert_eq!(held["result"]["met"], "status");

    let mut waiter = f.client();
    waiter.send(&json!({"kind": "control", "version": 1, "action": "wait", "target": target, "status": ["needs_you"], "timeout_ms": 10000}));
    // The wait is under way before the hook comes (a state that comes first is the next test's business).
    std::thread::sleep(Duration::from_millis(300));
    let mut hook = f.client();
    hook.send(&json!({"kind": "hook", "tab": id.to_string(), "agent": "claude", "event": "PermissionRequest:Bash", "message": "ok?"}));
    let reply = waiter.line().expect("the wait's answer");
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(
        (reply["result"]["met"].as_str(), reply["result"]["status"].as_str()),
        (Some("status"), Some("needs_you"))
    );
}

#[test]
fn what_a_terminal_prints_is_told_to_a_stream_that_may_read_it_and_a_wait_can_match_it() {
    let f = fixture("open");
    let id = f.terminal();
    let target = json!({"kind": "terminal", "id": id});
    let mut stream = f.client();
    let first = stream.control(json!({"action": "events", "topics": ["output"], "terminals": [id]}));
    assert_eq!(first["ok"], true, "{first}");

    let mut waiter = f.client();
    waiter.send(&json!({"kind": "control", "version": 1, "action": "wait", "target": target, "match": "needle-[0-9]+", "timeout_ms": 10000}));
    std::thread::sleep(Duration::from_millis(300));
    f.client().control(json!({"action": "send_text", "target": target, "text": "needle-42", "submit": true}));

    let reply = waiter.line().expect("the wait's answer");
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["result"]["met"], "text");
    let told = loop {
        let line = stream.line().expect("an event line");
        if line["event"] == "output" && line.to_string().contains("needle-42") {
            break line;
        }
    };
    assert_eq!(told["terminal"], id);
}

#[test]
fn run_types_a_command_at_the_prompt_and_tells_how_it_ended() {
    let f = fixture("open");
    let id = f.shell_terminal();
    let target = json!({"kind": "terminal", "id": id});
    let mut stream = f.client();
    assert_eq!(stream.control(json!({"action": "events", "topics": ["blocks"]}))["ok"], true);
    let mut client = f.client();
    let ran = client.control(json!({"action": "run", "target": target, "command": "echo ran-here; exit_code=3; (exit 3)", "timeout_ms": 10000}));
    assert_eq!(ran["ok"], true, "{ran}");
    assert_eq!(ran["result"]["exit_code"], 3);
    assert!(ran["result"]["output"].as_str().is_some_and(|out| out.contains("ran-here")), "{ran}");
    // The same command was told to the stream as it started and as it ended.
    let mut phases = Vec::new();
    while phases.len() < 2 {
        let line = stream.line().expect("an event line");
        if line["event"] == "block" {
            phases.push(line["phase"].as_str().expect("a phase").to_string());
        }
    }
    assert_eq!(phases, ["started", "finished"]);
}
