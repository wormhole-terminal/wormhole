//! An OSC string's payload as text: what a title, a folder or a notification is made of.

/// Cut `text` to at most `max` bytes, at a character boundary.
fn cut(text: &mut String, max: usize) {
    if text.len() > max {
        let mut at = max;
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        text.truncate(at);
    }
}

pub(in crate::terminal) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Replace `text`'s contents with `bytes` (invalid UTF-8 as U+FFFD), cut to `max` bytes at a
/// character boundary. Reuses the string's buffer: a title or folder that changes with every
/// prompt costs no allocation.
pub(in crate::terminal) fn fill(text: &mut String, bytes: &[u8], max: usize) {
    text.clear();
    match std::str::from_utf8(bytes) {
        Ok(valid) => text.push_str(valid),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    cut(text, max);
}
