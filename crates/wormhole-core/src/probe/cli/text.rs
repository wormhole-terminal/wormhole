//! What may be printed: text a Probe or a folder chose, cleaned, and the JSON that carries it.

use std::borrow::Cow;
use std::path::Path;

use serde::Serialize;

use crate::probe::hidden::{is_hidden, reveal};

/// `text` with every control, bidi and invisible character written out.
pub(super) fn clean(text: &str) -> Cow<'_, str> {
    reveal(text)
}

/// A path as it is printed.
pub(super) fn path(path: &Path) -> String {
    clean(&path.to_string_lossy()).into_owned()
}

/// `value` as one line of JSON, safe to print on a terminal: a character [`is_hidden`] names is written
/// as `\uXXXX` (as a surrogate pair above the first plane), which a JSON parser reads back as the
/// character. serde_json itself escapes only what JSON requires, which leaves an escape sequence's
/// C1 form and a right-to-left override to the terminal.
pub(super) fn json_line<T: Serialize>(value: &T) -> String {
    let raw = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut out = String::with_capacity(raw.len() + 1);
    for c in raw.chars() {
        if is_hidden(c) {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        } else {
            out.push(c);
        }
    }
    out.push('\n');
    out
}

/// The text of an output being built, one cleaned line at a time.
#[derive(Debug, Default)]
pub(super) struct Out(String);

impl Out {
    pub(super) fn line(&mut self, text: &str) {
        self.0.push_str(&clean(text));
        self.0.push('\n');
    }

    pub(super) fn blank(&mut self) {
        self.0.push('\n');
    }

    pub(super) fn finish(self) -> String {
        self.0
    }
}

/// `n` of something: `1 error`, `2 errors`.
pub(super) fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
