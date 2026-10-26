//! Reading property lists (iTerm2's and Terminal.app's preferences, both XML or binary) as plain
//! values: the accessors that accept the ways a preference is stored, and how a value is shown.
//!
//! Every property list of the domain is read through [`parse`], which refuses one that nests deeper
//! than any preferences file does: a hostile file could otherwise build a value whose drop or clone
//! overflows the stack.

use std::io::Cursor;
use std::path::Path;

use plist::stream::{Event, Reader as Events};
use plist::{Dictionary, Value};

use super::proposal::Reason;
use super::read::{MAX_PLIST_BYTES, Reader};

/// The deepest nesting of arrays and dictionaries read. iTerm2's own preferences are about eight
/// deep; an archive is flat.
pub(crate) const MAX_NESTING: usize = 64;

/// A property list's value, XML or binary, or `None` when it is not one or nests deeper than
/// [`MAX_NESTING`]. The events are walked first, with no value built, so how deep it goes is known
/// before anything recursive (building, cloning, dropping a [`Value`]) happens.
pub(crate) fn parse(bytes: &[u8]) -> Option<Value> {
    let mut depth = 0usize;
    for event in Events::new(Cursor::new(bytes)) {
        match event.ok()? {
            Event::StartArray(_) | Event::StartDictionary(_) => {
                depth += 1;
                if depth > MAX_NESTING {
                    return None;
                }
            }
            Event::EndCollection => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Value::from_reader(Cursor::new(bytes)).ok()
}

/// A property list's top-level dictionary, a source's own file. A file that is not one is noted.
pub(crate) fn read_dictionary(files: &mut Reader, path: &Path) -> Option<Dictionary> {
    let bytes = files.bytes(path, MAX_PLIST_BYTES)?;
    dictionary(files, path, &bytes)
}

/// [`read_dictionary`] of a file the settings name: inside the home folder only.
pub(crate) fn read_dictionary_in_home(files: &mut Reader, path: &Path) -> Option<Dictionary> {
    let bytes = files.bytes_in_home(path, MAX_PLIST_BYTES)?;
    dictionary(files, path, &bytes)
}

fn dictionary(files: &mut Reader, path: &Path, bytes: &[u8]) -> Option<Dictionary> {
    match parse(bytes) {
        Some(Value::Dictionary(dictionary)) => Some(dictionary),
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

/// A number stored as a real or as an integer; never `NaN` or infinite, however the file spells it.
pub(crate) fn real(dict: &Dictionary, key: &str) -> Option<f64> {
    let number = match dict.get(key)? {
        Value::Real(real) => *real,
        Value::Integer(integer) => integer.as_signed().map(|n| n as f64)?,
        Value::String(text) => text.trim().parse().ok()?,
        _ => return None,
    };
    number.is_finite().then_some(number)
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
