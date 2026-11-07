//! An event as a hook sees it: the fields it carries, by name. This is what a fixture's `event` table
//! is, and what the app will build from a bus event when it delivers one.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{CONTEXT, EventSpec, FieldKind, STATES};

/// A field's value: a number, or text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Number(i64),
    Text(String),
}

/// What the app knows about the terminal besides the event: which window is in front, and what the
/// terminal runs. Not payload, so a `match` can test them and a template cannot name them.
const FACTS: [&str; 2] = ["front", "kind"];

/// One event's fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct EventData(BTreeMap<String, Value>);

impl EventData {
    pub fn new(fields: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self(fields.into_iter().collect())
    }

    /// These fields and one more (a fact the app adds: `front`, `kind`, `folder_name`).
    #[must_use]
    pub fn with(mut self, name: &str, value: Value) -> Self {
        self.0.insert(name.to_string(), value);
        self
    }

    /// The fields of an `event` that are not [`FieldKind::Text`]: what a Probe that may not read what is on
    /// the terminals is told. A hook that needs one of the others does not match.
    #[must_use]
    pub fn without_text(&self, spec: &EventSpec) -> Self {
        let text = |name: &String| spec.field(name).is_some_and(|field| field.kind == FieldKind::Text);
        Self(self.0.iter().filter(|(name, _)| !text(name)).map(|(n, v)| (n.clone(), v.clone())).collect())
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    pub fn number(&self, name: &str) -> Option<i64> {
        match self.0.get(name)? {
            Value::Number(n) => Some(*n),
            Value::Text(_) => None,
        }
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        match self.0.get(name)? {
            Value::Text(text) => Some(text),
            Value::Number(_) => None,
        }
    }

    /// Whether the terminal's window is in front: `front = 1`. Absent means no.
    pub fn front(&self) -> bool {
        self.number("front") == Some(1)
    }

    /// What is wrong with these fields for `spec`: a field the event does not carry, one of the wrong
    /// kind, a state that is not one. Empty when it is a fine event.
    pub fn problems(&self, spec: &EventSpec) -> Vec<String> {
        let mut out = Vec::new();
        for (name, value) in &self.0 {
            if FACTS.contains(&name.as_str()) {
                let fine = match (name.as_str(), value) {
                    ("front", Value::Number(0 | 1)) => true,
                    ("kind", Value::Text(kind)) => matches!(kind.as_str(), "agent" | "shell"),
                    _ => false,
                };
                if !fine {
                    out.push(format!("`{name}` is `front = 0|1` or `kind = \"agent\"|\"shell\"`"));
                }
                continue;
            }
            let Some(field) = spec.fields.iter().chain(CONTEXT).find(|f| f.name == name) else {
                let names: Vec<_> = spec.fields.iter().chain(CONTEXT).map(|f| f.name).chain(FACTS).collect();
                out.push(format!("`{name}` is not a field of {}: {}", spec.event.name(), names.join(", ")));
                continue;
            };
            let fine = match (field.kind, value) {
                (FieldKind::Id | FieldKind::Number, Value::Number(_)) => true,
                (FieldKind::State, Value::Text(state)) => STATES.contains(&state.as_str()),
                (FieldKind::Name | FieldKind::Text, Value::Text(_)) => true,
                _ => false,
            };
            if !fine {
                out.push(match field.kind {
                    FieldKind::Id | FieldKind::Number => format!("`{name}` is a number"),
                    FieldKind::State => format!("`{name}` is one of {}", STATES.join(", ")),
                    FieldKind::Name | FieldKind::Text => format!("`{name}` is text"),
                });
            }
        }
        out
    }
}
