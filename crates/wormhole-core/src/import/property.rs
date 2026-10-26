//! Reading property lists (iTerm2's and Terminal.app's preferences, both XML or binary) as plain
//! values: the accessors that accept the ways a preference is stored, and how a value is shown.

use std::io::Cursor;
use std::path::Path;

use plist::{Dictionary, Value};

use super::proposal::Reason;
use super::read::{MAX_PLIST_BYTES, Reader};

/// A property list's top-level dictionary, XML or binary. A file that is not one is noted.
pub(crate) fn read_dictionary(files: &mut Reader, path: &Path) -> Option<Dictionary> {
    let bytes = files.bytes(path, MAX_PLIST_BYTES)?;
    match Value::from_reader(Cursor::new(bytes)) {
        Ok(Value::Dictionary(dictionary)) => Some(dictionary),
        _ => {
            files.note(path, Reason::Unreadable);
            None
        }
    }
}

/// A string value.
pub(crate) fn string<'a>(dict: &'a Dictionary, key: &str) -> Option<&'a str> {
    dict.get(key).and_then(Value::as_string)
}

/// A number stored as a real or as an integer.
pub(crate) fn real(dict: &Dictionary, key: &str) -> Option<f64> {
    match dict.get(key)? {
        Value::Real(real) => Some(*real),
        Value::Integer(integer) => integer.as_signed().map(|n| n as f64),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// A whole number, from an integer or a real without a fraction.
pub(crate) fn integer(dict: &Dictionary, key: &str) -> Option<i64> {
    match dict.get(key)? {
        Value::Integer(integer) => integer.as_signed(),
        Value::Real(real) if real.fract() == 0.0 => Some(*real as i64),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// A switch stored as a boolean, or as 0 and 1.
pub(crate) fn boolean(dict: &Dictionary, key: &str) -> Option<bool> {
    match dict.get(key)? {
        Value::Boolean(on) => Some(*on),
        Value::Integer(integer) => integer.as_signed().map(|n| n != 0),
        _ => None,
    }
}

/// A scalar as it reads in the preferences (`true`, `0.15`, `Monaco 12`), for the review list.
pub(crate) fn show(value: &Value) -> String {
    match value {
        Value::Boolean(on) => on.to_string(),
        Value::Integer(integer) => integer.as_signed().map_or_else(|| "?".to_string(), |n| n.to_string()),
        Value::Real(real) => real.to_string(),
        Value::String(text) => text.clone(),
        _ => String::new(),
    }
}

/// `key = value` of a dictionary's scalar, for the review list.
pub(crate) fn shown(dict: &Dictionary, key: &str) -> String {
    format!("{key} = {}", dict.get(key).map(show).unwrap_or_default())
}
