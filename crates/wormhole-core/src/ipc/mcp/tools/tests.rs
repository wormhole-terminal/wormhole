//! The tools' tables: every tool has a name a client accepts, a schema that says what it takes (checked by
//! a small validator for the keywords used here), and arguments that become exactly the request the
//! command line would send.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

use super::schema_check::{keywords_known, valid};
use super::{Call, all, find};
use crate::ipc::control::{Request, TerminalState};
use crate::ipc::exchange::Caller;
use crate::ipc::mcp::exec::Endpoint;
use crate::ipc::mcp::version::Version;

fn endpoint() -> Endpoint {
    Endpoint {
        caller: Caller { socket: PathBuf::from("/nonexistent.sock"), from: Some("7".into()) },
        cwd: PathBuf::from("/work"),
        home: Some(PathBuf::from("/home/me")),
        automation: || true,
    }
}

/// The request a tool's arguments make, as the wire has it.
fn request(tool: &str, arguments: Value) -> Result<Value, String> {
    let tool = find(tool).unwrap_or_else(|| panic!("no tool {tool}"));
    match (tool.build)(arguments, &endpoint())? {
        Call::Control(action) => Ok(serde_json::to_value(Request::new(action, None)).expect("serializes")),
        Call::Events(_) => panic!("{} is not a single request", tool.name),
    }
}

fn wormhole_core_doctor_facts() -> crate::ipc::control::doctor::Facts {
    use crate::ipc::control::doctor::{Caller, Facts};
    Facts {
        app_version: "1.3.0".into(),
        protocol: 1,
        automation: true,
        control_write: false,
        control_read: false,
        caller: Some(Caller { terminal: 3, verified: true, shell_integration: Some(true) }),
        access: None,
    }
}

#[test]
fn every_tool_has_a_name_a_client_accepts_a_description_and_known_schema_keywords() {
    let tools: Vec<_> = all().collect();
    assert_eq!(
        tools.iter().map(|tool| tool.name).collect::<Vec<_>>(),
        [
            "list_terminals",
            "terminal_status",
            "read_terminal",
            "list_blocks",
            "notify",
            "doctor",
            "send_text",
            "send_keys",
            "mouse",
            "new_terminal",
            "new_window",
            "split",
            "focus",
            "rename",
            "close",
            "wait",
            "run",
            "wait_events"
        ]
    );
    for tool in tools {
        let name_ok = !tool.name.is_empty()
            && tool.name.len() <= 128
            && tool.name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.');
        assert!(name_ok, "{}", tool.name);
        assert!(tool.description.len() > 60, "{} needs a description a model can use", tool.name);
        assert!(!tool.title.is_empty());
        for (kind, schema) in [("input", (tool.input)()), ("output", (tool.output)())] {
            assert_eq!(schema["type"], json!("object"), "{} {kind}", tool.name);
            keywords_known(&schema)
                .unwrap_or_else(|word| panic!("{} {kind} schema: unknown keyword {word}", tool.name));
        }
        assert_eq!(
            (tool.input)()["additionalProperties"],
            json!(false),
            "{} refuses what it does not know",
            tool.name
        );
        // Annotations are not contradictory: a tool that only looks changes nothing.
        let hints = tool.hints;
        assert!(!(hints.read_only && hints.destructive), "{}", tool.name);
    }
}

#[test]
fn what_a_tool_may_do_is_said_truthfully() {
    let hints = |name: &str| find(name).expect("tool").hints;
    for name in
        ["list_terminals", "terminal_status", "read_terminal", "list_blocks", "doctor", "wait", "wait_events"]
    {
        assert!(hints(name).read_only && !hints(name).open_world, "{name} only looks");
    }
    // What types into programs or starts them can do anything the user can.
    for name in ["send_text", "send_keys", "mouse", "run", "new_terminal", "new_window", "split"] {
        let h = hints(name);
        assert!(!h.read_only && h.destructive && h.open_world && !h.idempotent, "{name}");
    }
    assert!(hints("close").destructive, "closing a terminal ends its shell, if the person says yes");
    assert!(hints("focus").idempotent && hints("rename").idempotent);
    assert!(!hints("notify").read_only && !hints("notify").idempotent);
}

/// The definitions as each revision shows them: later revisions add titles, output schemas and annotations.
#[test]
fn a_tool_is_shown_with_the_fields_its_revision_has() {
    let tool = find("wait").expect("tool");
    let old = tool.definition(Version::V2024_11_05);
    assert!(
        old.get("annotations").is_none() && old.get("outputSchema").is_none() && old.get("title").is_none()
    );
    let march = tool.definition(Version::V2025_03_26);
    assert!(march.get("annotations").is_some() && march.get("outputSchema").is_none());
    let new = tool.definition(Version::V2026_07_28);
    assert_eq!(new["title"], json!("Wait for a terminal"));
    assert_eq!(new["annotations"]["readOnlyHint"], json!(true));
    assert_eq!(new["annotations"]["openWorldHint"], json!(false));
    assert!(new["outputSchema"]["properties"]["met"].is_object());
}

/// Arguments → the request the command line would send. Folders resolve like the command line's: `~` and
/// relative ones against this process's home and working directory.
#[test]
fn arguments_become_the_golden_requests() {
    let target = json!({"kind": "terminal", "id": 3});
    let cases: Vec<(&str, Value, Value)> = vec![
        ("list_terminals", json!({}), json!({"version": 1, "action": "list"})),
        ("doctor", json!({}), json!({"version": 1, "action": "doctor"})),
        ("terminal_status", json!({}), json!({"version": 1, "action": "status"})),
        (
            "terminal_status",
            json!({"terminal": 3}),
            json!({"version": 1, "action": "status", "target": target}),
        ),
        ("read_terminal", json!({}), json!({"version": 1, "action": "read", "range": {"kind": "screen"}})),
        (
            "read_terminal",
            json!({"terminal": 3, "last_lines": 200, "join": true}),
            json!({"version": 1, "action": "read", "target": target, "range": {"kind": "last", "lines": 200}, "join": true}),
        ),
        (
            "read_terminal",
            json!({"terminal": 3, "from_line": -10, "to_line": 5, "styles": true}),
            json!({"version": 1, "action": "read", "target": target, "range": {"kind": "lines", "from": -10, "to": 5}, "styles": true}),
        ),
        (
            "read_terminal",
            json!({"terminal": 3, "block": "last"}),
            json!({"version": 1, "action": "read", "target": target, "range": {"kind": "block", "id": "last"}}),
        ),
        (
            "read_terminal",
            json!({"terminal": 3, "block": 57}),
            json!({"version": 1, "action": "read", "target": target, "range": {"kind": "block", "id": 57}}),
        ),
        (
            "list_blocks",
            json!({"terminal": 3, "last": 5, "include_output": true}),
            json!({"version": 1, "action": "blocks", "target": target, "last": 5, "output": true}),
        ),
        (
            "notify",
            json!({"terminal": 3, "title": "Done", "body": "tests pass"}),
            json!({"version": 1, "action": "notify", "target": target, "title": "Done", "body": "tests pass"}),
        ),
        (
            "send_text",
            json!({"terminal": 3, "text": "ls", "submit": true}),
            json!({"version": 1, "action": "send_text", "target": target, "text": "ls", "submit": true}),
        ),
        (
            "send_keys",
            json!({"terminal": 3, "keys": ["ctrl-c", {"text": "ls -la"}, {"text": "a\nb", "paste": true}, {"key": "enter"}]}),
            json!({"version": 1, "action": "send_keys", "target": target, "keys": [
                {"key": "ctrl-c"}, {"text": "ls -la"}, {"text": "a\nb", "paste": true}, {"key": "enter"}]}),
        ),
        (
            "mouse",
            json!({"terminal": 3, "action": "click", "row": 4, "col": 10, "count": 2, "shift": true}),
            json!({"version": 1, "action": "mouse", "target": target,
                   "op": {"kind": "click", "row": 4, "col": 10, "count": 2, "shift": true}}),
        ),
        (
            "mouse",
            json!({"terminal": 3, "action": "scroll", "row": 4, "col": 10, "direction": "down", "lines": 3}),
            json!({"version": 1, "action": "mouse", "target": target,
                   "op": {"kind": "scroll", "row": 4, "col": 10, "direction": "down", "lines": 3}}),
        ),
        (
            "new_terminal",
            json!({"folder": "~/src", "command": "npm run dev", "window": 2}),
            json!({"version": 1, "action": "new_terminal", "folder": "/home/me/src", "command": "npm run dev", "window": 2}),
        ),
        (
            "new_window",
            json!({"folder": "api", "activate": true}),
            json!({"version": 1, "action": "new_window", "folder": "/work/api", "activate": true}),
        ),
        (
            "split",
            json!({"terminal": 3, "direction": "down", "size": 40, "command": "make watch", "select": true}),
            json!({"version": 1, "action": "split", "target": target, "direction": "down", "size": 40,
                   "command": "make watch", "select": true}),
        ),
        (
            "split",
            json!({"terminal": 3}),
            json!({"version": 1, "action": "split", "target": target, "direction": "right"}),
        ),
        (
            "focus",
            json!({"terminal": 3, "activate": true}),
            json!({"version": 1, "action": "focus", "target": target, "activate": true}),
        ),
        (
            "rename",
            json!({"terminal": 3, "name": "api"}),
            json!({"version": 1, "action": "rename", "target": target, "name": "api"}),
        ),
        ("rename", json!({"terminal": 3}), json!({"version": 1, "action": "rename", "target": target})),
        ("close", json!({"terminal": 3}), json!({"version": 1, "action": "close", "target": target})),
        (
            "wait",
            json!({"terminal": 3, "status": ["needs_you", "failed"], "match": "\\$$", "stable_ms": 500,
                   "block_done": true, "after_block": 9, "timeout_seconds": 90}),
            json!({"version": 1, "action": "wait", "target": target, "status": ["needs_you", "failed"],
                   "match": "\\$$", "stable_ms": 500, "block_done": true, "after": 9, "timeout_ms": 90000}),
        ),
        (
            "wait",
            json!({"terminal": 3, "gone": "Compiling"}),
            json!({"version": 1, "action": "wait", "target": target, "gone": "Compiling", "timeout_ms": 30000}),
        ),
        (
            "run",
            json!({"terminal": 3, "command": "cargo test"}),
            json!({"version": 1, "action": "run", "target": target, "command": "cargo test", "timeout_ms": 60000}),
        ),
        (
            "run",
            json!({"terminal": 3, "command": "make", "timeout_seconds": 120}),
            json!({"version": 1, "action": "run", "target": target, "command": "make", "timeout_ms": 120000}),
        ),
    ];
    for (tool, arguments, expected) in cases {
        let schema = (find(tool).expect("tool").input)();
        valid(&schema, &arguments)
            .unwrap_or_else(|why| panic!("{tool} {arguments}: the schema refuses it: {why}"));
        assert_eq!(request(tool, arguments.clone()).expect("builds"), expected, "{tool} {arguments}");
    }
}

#[test]
fn the_events_tool_builds_a_query_with_its_defaults_and_its_filter() {
    let tool = find("wait_events").expect("tool");
    let Call::Events(first) = (tool.build)(json!({}), &endpoint()).expect("builds") else { panic!("events") };
    assert_eq!(
        (first.since, first.timeout, first.max_events, first.output),
        (None, Duration::from_secs(10), 100, false)
    );
    let Call::Events(next) = (tool.build)(
        json!({"since_seq": 40, "boot": "b1", "timeout_seconds": 0, "topics": ["status", "blocks"], "terminals": [3], "max_events": 5}),
        &endpoint(),
    )
    .expect("builds") else {
        panic!("events")
    };
    assert_eq!(
        (next.since, next.boot.as_deref(), next.timeout, next.max_events),
        (Some(40), Some("b1"), Duration::ZERO, 5)
    );
    assert_eq!(next.filter.topics.names(), ["status", "blocks"]);
    assert_eq!(next.filter.terminals, [3]);
}

/// Arguments the schema refuses, and the builder with them: each is told in words the model can act on.
#[test]
fn wrong_arguments_are_told_in_words_and_the_schema_says_no_too() {
    let cases: Vec<(&str, Value, &str)> = vec![
        ("list_terminals", json!({"terminal": 3}), "unknown field"),
        ("send_text", json!({"text": "ls"}), "terminal"),
        ("send_text", json!({"terminal": "3", "text": "ls"}), "invalid type"),
        ("send_keys", json!({"terminal": 3, "keys": [5]}), "did not match"),
        ("read_terminal", json!({"last_lines": 5, "block": "last"}), "one of"),
        ("read_terminal", json!({"from_line": 5}), "go together"),
        ("read_terminal", json!({"block": "newest"}), "did not match"),
        ("mouse", json!({"terminal": 3, "action": "scroll", "row": 0, "col": 0}), "direction"),
        ("mouse", json!({"terminal": 3, "action": "wiggle", "row": 0, "col": 0}), "unknown variant"),
        ("wait", json!({"terminal": 3, "timeout_seconds": 0}), "at least 1"),
        ("wait", json!({"terminal": 3, "timeout_seconds": 121}), "at most 120"),
        ("run", json!({"terminal": 3, "command": "x", "timeout_seconds": 9999}), "at most 120"),
        ("split", json!({"terminal": 3, "direction": "diagonal"}), "unknown variant"),
        ("wait_events", json!({"topics": ["nope"]}), "unknown topic"),
        ("wait_events", json!({"timeout_seconds": 31}), "at most 30"),
        ("wait_events", json!({"max_events": 0}), "max_events"),
    ];
    for (tool, arguments, expected) in cases {
        let words = match (find(tool).expect("tool").build)(arguments.clone(), &endpoint()) {
            Err(words) => words,
            Ok(_) => panic!("{tool} {arguments} was accepted"),
        };
        assert!(words.contains(expected), "{tool} {arguments}: {words}");
    }
    let schema_refuses: Vec<(&str, Value)> = vec![
        ("send_text", json!({"text": "ls"})),
        ("send_text", json!({"terminal": 3, "text": "x".repeat(16 * 1024 + 1)})),
        ("send_keys", json!({"terminal": 3, "keys": []})),
        ("send_keys", json!({"terminal": 3, "keys": vec!["a"; 65]})),
        ("mouse", json!({"terminal": 3, "action": "click", "row": -1, "col": 0})),
        ("mouse", json!({"terminal": 3, "action": "click", "row": 0, "col": 0, "count": 4})),
        ("split", json!({"terminal": 3, "size": 100})),
        ("wait", json!({"terminal": 3, "timeout_seconds": 121})),
        ("wait", json!({"terminal": 3, "stable_ms": 60_001})),
        ("read_terminal", json!({"last_lines": 5001})),
        ("list_blocks", json!({"last": 101})),
        ("notify", json!({"body": "x".repeat(401)})),
        ("rename", json!({"terminal": 3, "name": "x".repeat(65)})),
        ("wait_events", json!({"max_events": 501})),
    ];
    for (tool, arguments) in schema_refuses {
        let schema = (find(tool).expect("tool").input)();
        assert!(valid(&schema, &arguments).is_err(), "{tool} {arguments}");
    }
}

#[test]
fn every_state_a_wait_names_is_one_the_app_knows() {
    let schema = (find("wait").expect("tool").input)();
    let words = schema["properties"]["status"]["items"]["enum"].as_array().expect("enum").clone();
    for word in words {
        let state: TerminalState = serde_json::from_value(word.clone()).expect("reads");
        assert_ne!(state, TerminalState::Other, "{word}");
    }
}

/// What each tool says it returns is what the app's reply types serialize to: every result type, as the app
/// makes it (the always-present fields and the optional ones), against its tool's `outputSchema`.
#[test]
fn every_reply_type_fits_the_output_schema_of_its_tool() {
    use crate::ipc::control::read::{BlockInfo, Blocks, CursorInfo, ModesInfo, Read, Size};
    use crate::ipc::control::{
        Asked, CloseAsked, Focused, KeysSent, Listing, Met, Opened, PaneInfo, Ran, Reached, Renamed,
        SplitInfo, SplitMade, StatusInfo, Summary, TabInfo, TerminalInfo, Waited, WindowInfo, WindowOpened,
    };
    use wormhole_vt::Modes;

    let tab = TabInfo {
        id: 3,
        name: "api".into(),
        status: TerminalState::Working,
        agent: Some("claude".into()),
        message: None,
        folder: Some("/work".into()),
        shell: "zsh".into(),
        command: None,
        since_seconds: 4,
        selected: true,
        panes: vec![PaneInfo { id: 3 }],
    };
    let terminal = TerminalInfo {
        id: 3,
        name: "api".into(),
        status: TerminalState::Working,
        agent: None,
        folder: None,
        branch: None,
        changed: None,
        pinned: false,
        profile: None,
        selected: true,
        split: Some(1),
        tabs: vec![tab],
    };
    let read = |block: Option<i64>| Read {
        terminal: 3,
        title: None,
        screen: "main".into(),
        size: Size { rows: 24, cols: 80 },
        history: 10,
        cursor: CursorInfo { row: 2, col: 4, visible: true, shape: Some("block".into()) },
        modes: ModesInfo::of(Modes::default()),
        first_line: 0,
        lines: vec!["$ ".into()],
        styles: None,
        truncated: false,
        revision: 9,
        block,
    };
    let samples: Vec<(&str, Value)> = vec![
        (
            "list_terminals",
            serde_json::to_value(Listing {
                windows: vec![WindowInfo {
                    id: 1,
                    front: true,
                    terminals: vec![terminal.clone()],
                    splits: vec![SplitInfo { id: 1, terminals: vec![3], focused: 3 }],
                }],
            })
            .expect("serializes"),
        ),
        ("list_terminals", serde_json::to_value(Listing::default()).expect("serializes")),
        (
            "terminal_status",
            serde_json::to_value(StatusInfo { window: 1, terminal: terminal.clone(), tab: 3 })
                .expect("serializes"),
        ),
        ("terminal_status", serde_json::to_value(Summary::of(1, [TerminalState::Idle])).expect("serializes")),
        ("read_terminal", serde_json::to_value(read(None)).expect("serializes")),
        ("read_terminal", serde_json::to_value(read(Some(57))).expect("serializes")),
        (
            "list_blocks",
            serde_json::to_value(Blocks {
                terminal: 3,
                integration: true,
                blocks: vec![BlockInfo {
                    id: 57,
                    command: "make".into(),
                    state: "finished".into(),
                    exit_code: Some(2),
                    took_ms: Some(10),
                    first_line: -3,
                    last_line: -1,
                    output: Some("error".into()),
                    truncated: false,
                }],
            })
            .expect("serializes"),
        ),
        ("doctor", serde_json::to_value(wormhole_core_doctor_facts()).expect("serializes")),
        ("notify", serde_json::to_value(Reached { tab: 3 }).expect("serializes")),
        ("send_text", serde_json::to_value(Reached { tab: 3 }).expect("serializes")),
        ("mouse", serde_json::to_value(Reached { tab: 3 }).expect("serializes")),
        ("send_keys", serde_json::to_value(KeysSent { tab: 3, keys: 2 }).expect("serializes")),
        (
            "new_terminal",
            serde_json::to_value(Opened {
                window: 1,
                terminal: 4,
                tab: 4,
                pane: 4,
                folder: None,
                worktree: None,
            })
            .expect("serializes"),
        ),
        (
            "new_terminal",
            serde_json::to_value(Asked { asked: true, window: 1, branch: "x".into(), folder: "/w".into() })
                .expect("serializes"),
        ),
        (
            "new_window",
            serde_json::to_value(WindowOpened { window: 2, terminal: 5, folder: Some("/w".into()) })
                .expect("serializes"),
        ),
        (
            "split",
            serde_json::to_value(SplitMade {
                window: 1,
                split: 2,
                terminal: 6,
                folder: None,
                size: Some(40),
            })
            .expect("serializes"),
        ),
        ("focus", serde_json::to_value(Focused { window: 1, terminal: 3 }).expect("serializes")),
        (
            "rename",
            serde_json::to_value(Renamed { terminal: 3, name: "api".into(), custom: true })
                .expect("serializes"),
        ),
        (
            "close",
            serde_json::to_value(CloseAsked { asked: true, window: 1, terminal: 3 }).expect("serializes"),
        ),
        (
            "wait",
            serde_json::to_value(Waited {
                terminal: 3,
                met: Met::Text,
                after_ms: 30,
                status: None,
                matched: Some("$".into()),
                block: None,
                exit_code: None,
                revision: Some(4),
            })
            .expect("serializes"),
        ),
        (
            "run",
            serde_json::to_value(Ran {
                terminal: 3,
                block: 58,
                exit_code: Some(0),
                duration_ms: 1200,
                output: Some("ok".into()),
                truncated: false,
                output_hidden: None,
            })
            .expect("serializes"),
        ),
        (
            "run",
            serde_json::to_value(Ran {
                terminal: 3,
                block: 58,
                exit_code: None,
                duration_ms: 1,
                output: None,
                truncated: false,
                output_hidden: Some("control_read is off".into()),
            })
            .expect("serializes"),
        ),
    ];
    for (tool, sample) in samples {
        let schema = (find(tool).expect("tool").output)();
        valid(&schema, &sample).unwrap_or_else(|why| panic!("{tool}: {why}: {sample}"));
    }
    // And the schema is not empty talk: a result that lacks what the tool promises is refused.
    let schema = (find("run").expect("tool").output)();
    assert!(valid(&schema, &json!({"terminal": 3})).is_err());
}
