//! Reading a TOML table by dotted path, the way the TOML-based sources (Alacritty, Warp) are read.

use toml::{Table, Value};

/// The value at a dotted path (`font.normal.family`).
pub(crate) fn at<'a>(table: &'a Table, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let mut value = table.get(parts.next()?)?;
    for part in parts {
        value = value.as_table()?.get(part)?;
    }
    Some(value)
}

/// A number, whichever way the file wrote it.
pub(crate) fn float(value: &Value) -> Option<f32> {
    match value {
        Value::Float(f) => Some(*f as f32),
        Value::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

/// `path = value`, for the review list.
pub(crate) fn shown(path: &str, value: &Value) -> String {
    format!("{path} = {value}")
}
