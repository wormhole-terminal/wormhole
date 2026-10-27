//! The pieces the tools' JSON Schemas (draft 2020-12, MCP's default dialect) are built from, so the bounds
//! that appear in more than one tool are written once.

use serde_json::{Map, Value, json};

/// The schema of an arguments object: these properties, these required, nothing else.
pub(super) fn input(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({"type": "object", "properties": properties, "additionalProperties": false});
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

/// The schema of a result object: the properties that are always there are required, and the app may add
/// more in a later version (a client must not refuse a result for a field it does not know).
pub(super) fn output(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({"type": "object", "properties": properties});
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

pub(super) fn integer(description: &str, min: i64, max: i64) -> Value {
    json!({"type": "integer", "minimum": min, "maximum": max, "description": description})
}

pub(super) fn string(description: &str, max_length: usize) -> Value {
    json!({"type": "string", "maxLength": max_length, "description": description})
}

pub(super) fn boolean(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

pub(super) fn one_of(description: &str, words: &[&str]) -> Value {
    json!({"type": "string", "enum": words, "description": description})
}

/// A terminal's id, as `list_terminals` gives it.
pub(super) fn terminal(description: &str) -> Value {
    integer(description, 0, i64::MAX)
}

/// Properties side by side: `props(&[("terminal", …), …])`.
pub(super) fn props(entries: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    for (name, schema) in entries {
        map.insert((*name).to_string(), schema.clone());
    }
    Value::Object(map)
}

/// A result field of a plain type, described.
pub(super) fn field(kind: &str, description: &str) -> Value {
    json!({"type": kind, "description": description})
}
