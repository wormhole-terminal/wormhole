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

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn a_text_is_cut_back_to_the_character_before_the_limit_not_through_one() {
        let mut text = String::new();
        // "é" is two bytes: a limit of 2 falls in the middle of it, so it goes.
        fill(&mut text, "aé".as_bytes(), 2);
        assert_eq!(text, "a");
        fill(&mut text, "aé".as_bytes(), 3);
        assert_eq!(text, "aé", "it fits exactly");
        // Two bytes into a three-byte character; and a text that is not UTF-8 is cut as what it
        // becomes (U+FFFD is three bytes as well).
        fill(&mut text, "日本".as_bytes(), 4);
        assert_eq!(text, "日");
        fill(&mut text, b"\xffabc", 4);
        assert_eq!(text, "\u{fffd}a");
        fill(&mut text, "日本".as_bytes(), 2);
        assert_eq!(text, "", "not even the first character fits");
    }
}
