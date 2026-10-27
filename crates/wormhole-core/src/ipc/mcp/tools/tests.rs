//! The tools' tables: every tool has a name a client accepts, a schema that says what it takes (checked by
//! a small validator for the keywords used here), and arguments that become exactly the request the
//! command line would send.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

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

// A small JSON Schema checker: the keywords the tools use, and a failure for any other.

const KEYWORDS: &[&str] = &[
    "type",
    "properties",
    "required",
    "additionalProperties",
    "description",
    "enum",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "uniqueItems",
    "items",
    "oneOf",
];

fn valid(schema: &Value, value: &Value) -> Result<(), String> {
    let Value::Object(schema) = schema else { return Err(format!("a schema is an object, not {schema}")) };
    if let Some(unknown) = schema.keys().find(|key| !KEYWORDS.contains(&key.as_str())) {
        return Err(format!("the checker does not know the keyword {unknown}"));
    }
    if let Some(kind) = schema.get("type").and_then(Value::as_str) {
        let ok = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            other => return Err(format!("type {other}")),
        };
        if !ok {
            return Err(format!("{value} is not a {kind}"));
        }
    }
    if let Some(words) = schema.get("enum").and_then(Value::as_array)
        && !words.contains(value)
    {
        return Err(format!("{value} is not one of {words:?}"));
    }
    if let Some(number) = value.as_i64().or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok())) {
        if schema.get("minimum").and_then(Value::as_i64).is_some_and(|min| number < min) {
            return Err(format!("{number} is below the minimum"));
        }
        if schema.get("maximum").and_then(Value::as_i64).is_some_and(|max| number > max) {
            return Err(format!("{number} is above the maximum"));
        }
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if schema.get("minLength").and_then(Value::as_u64).is_some_and(|min| length < min) {
            return Err("too short".into());
        }
        if schema.get("maxLength").and_then(Value::as_u64).is_some_and(|max| length > max) {
            return Err("too long".into());
        }
    }
    if let Value::Array(items) = value {
        let count = items.len() as u64;
        if schema.get("minItems").and_then(Value::as_u64).is_some_and(|min| count < min) {
            return Err("too few items".into());
        }
        if schema.get("maxItems").and_then(Value::as_u64).is_some_and(|max| count > max) {
            return Err("too many items".into());
        }
        if let Some(item_schema) = schema.get("items") {
            items.iter().try_for_each(|item| valid(item_schema, item))?;
        }
    }
    if let Value::Object(object) = value {
        for name in schema.get("required").and_then(Value::as_array).into_iter().flatten() {
            let name = name.as_str().expect("names");
            if !object.contains_key(name) {
                return Err(format!("{name} is required"));
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (name, member) in object {
            match properties.and_then(|properties| properties.get(name)) {
                Some(property) => valid(property, member).map_err(|why| format!("{name}: {why}"))?,
                None if schema.get("additionalProperties") == Some(&json!(false)) => {
                    return Err(format!("{name} is not a property"));
                }
                None => {}
            }
        }
    }
    if let Some(options) = schema.get("oneOf").and_then(Value::as_array) {
        let matching = options.iter().filter(|option| valid(option, value).is_ok()).count();
        if matching != 1 {
            return Err(format!("{value} matches {matching} of the options"));
        }
    }
    Ok(())
}

/// Every schema node, to see that its keywords are known (a typo such as `maxLenght` would be ignored by a
/// client, and the bound would be gone).
fn keywords_known(schema: &Value) -> Result<(), String> {
    let Value::Object(schema) = schema else { return Ok(()) };
    if let Some(unknown) = schema.keys().find(|key| !KEYWORDS.contains(&key.as_str())) {
        return Err(unknown.clone());
    }
    for properties in schema.get("properties").and_then(Value::as_object).into_iter() {
        properties.values().try_for_each(keywords_known)?;
    }
    for sub in schema
        .get("items")
        .into_iter()
        .chain(schema.get("oneOf").and_then(Value::as_array).into_iter().flatten())
    {
        keywords_known(sub)?;
    }
    Ok(())
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
    for name in ["list_terminals", "terminal_status", "read_terminal", "list_blocks", "wait", "wait_events"] {
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
