//! Bytes and text as a person reads them.

use std::fmt::Write as _;

/// `text` cut to `max` characters in the middle.
pub(crate) fn shortened(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let head = max * 2 / 3;
    let tail = max - head;
    format!(
        "{}…{}",
        chars[..head].iter().collect::<String>(),
        chars[chars.len() - tail..].iter().collect::<String>()
    )
}

/// Bytes as a person can read them: `\e`, `\r`, `\n`, `\xNN`; valid UTF-8 as itself.
pub(crate) fn escaped(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let (valid, bad) = match std::str::from_utf8(rest) {
            Ok(text) => (text, 0),
            Err(err) => {
                let (good, _) = rest.split_at(err.valid_up_to());
                // `good` is valid by the error's own word.
                (std::str::from_utf8(good).unwrap_or(""), err.error_len().unwrap_or(rest.len() - good.len()))
            }
        };
        for c in valid.chars() {
            match c {
                '\x1b' => out.push_str("\\e"),
                '\r' => out.push_str("\\r"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\x07' => out.push_str("\\a"),
                '\\' => out.push_str("\\\\"),
                c if c.is_control() => {
                    let _ = write!(out, "\\x{:02x}", c as u32);
                }
                c => out.push(c),
            }
        }
        let used = valid.len();
        for b in &rest[used..used + bad] {
            let _ = write!(out, "\\x{b:02x}");
        }
        rest = &rest[used + bad..];
    }
    out
}

pub(crate) fn text_name(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    format!("text \"{}\" ({} chars)", escaped(shortened(&text, 40).as_bytes()), text.chars().count())
}
