//! Escapes in `.vtt` text: `\e`, `\x1b`, `\u{1f600}`, `\n`, … and the `*` of a reply pattern.

/// Turn the escapes in `s` into bytes. `\xNN` is a raw byte (so invalid UTF-8 can be written);
/// `\u{…}` is the character's UTF-8 encoding.
pub fn unescape_bytes(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match escape(&mut chars)? {
            Escaped::Byte(b) => out.push(b),
            Escaped::Char(ch) => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    Ok(out)
}

/// Turn the escapes in `s` into text, for expected screen rows and titles. `\xNN` here is the
/// character U+00NN and only ASCII is allowed: a byte that is not a character cannot be on a
/// screen.
pub fn unescape_text(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match escape(&mut chars)? {
            Escaped::Byte(b) if b.is_ascii() => out.push(b as char),
            Escaped::Byte(b) => return Err(format!("\\x{b:02x} is not a character; use \\u{{…}} in text")),
            Escaped::Char(ch) => out.push(ch),
        }
    }
    Ok(out)
}

enum Escaped {
    Byte(u8),
    Char(char),
}

fn escape(chars: &mut std::str::Chars<'_>) -> Result<Escaped, String> {
    let Some(c) = chars.next() else {
        return Err("a backslash at the end".to_string());
    };
    Ok(match c {
        'e' => Escaped::Byte(0x1b),
        'a' => Escaped::Byte(0x07),
        'b' => Escaped::Byte(0x08),
        't' => Escaped::Byte(0x09),
        'n' => Escaped::Byte(0x0a),
        'v' => Escaped::Byte(0x0b),
        'f' => Escaped::Byte(0x0c),
        'r' => Escaped::Byte(0x0d),
        's' => Escaped::Byte(b' '),
        '0' => Escaped::Byte(0),
        '\\' => Escaped::Byte(b'\\'),
        'x' => {
            let hex: String = chars.by_ref().take(2).collect();
            let byte = (hex.len() == 2)
                .then(|| u8::from_str_radix(&hex, 16).ok())
                .flatten()
                .ok_or_else(|| format!("\\x needs two hex digits, got \\x{hex}"))?;
            Escaped::Byte(byte)
        }
        'u' => {
            if chars.next() != Some('{') {
                return Err("\\u needs braces: \\u{1f600}".to_string());
            }
            let mut hex = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(c) if c.is_ascii_hexdigit() && hex.len() < 6 => hex.push(c),
                    _ => return Err("\\u{…} needs 1 to 6 hex digits and a closing brace".to_string()),
                }
            }
            let ch = u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| format!("\\u{{{hex}}} is not a character"))?;
            Escaped::Char(ch)
        }
        other => return Err(format!("unknown escape \\{other}")),
    })
}

/// Whether `text` matches `pattern`, where `*` stands for any run of bytes (also none).
pub fn glob_match(pattern: &[u8], text: &[u8]) -> bool {
    let (mut p, mut t) = (0, 0);
    // Where the last `*` was in the pattern, and how much text it has swallowed so far.
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == b'*' {
            star = Some((p, t));
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|b| *b == b'*')
}

#[cfg(test)]
mod tests {
    use super::{glob_match, unescape_bytes, unescape_text};

    #[test]
    fn bytes_escapes() {
        assert_eq!(unescape_bytes(r"\e[1m\r\n").unwrap(), b"\x1b[1m\r\n");
        assert_eq!(unescape_bytes(r"\x1b\x9b\xff").unwrap(), [0x1b, 0x9b, 0xff]);
        assert_eq!(unescape_bytes(r"\u{e9}\u{1F600}").unwrap(), "\u{e9}\u{1f600}".as_bytes());
        assert_eq!(unescape_bytes(r"a\sb\\c").unwrap(), b"a b\\c");
        assert_eq!(unescape_bytes("日本").unwrap(), "日本".as_bytes());
    }

    #[test]
    fn bad_escapes_are_errors() {
        for bad in [r"\q", r"\x1", r"\xzz", r"\u41", r"\u{110000}", r"\u{}", "abc\\"] {
            assert!(unescape_bytes(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn text_escapes() {
        assert_eq!(unescape_text(r"e\u{301}").unwrap(), "e\u{301}");
        assert_eq!(unescape_text(r"a\x41").unwrap(), "aA");
        assert!(unescape_text(r"\xff").is_err());
    }

    #[test]
    fn globs() {
        assert!(glob_match(b"\x1b[?6*c", b"\x1b[?62;1;22c"));
        assert!(glob_match(b"\x1b[?6*c", b"\x1b[?6c"));
        assert!(glob_match(b"*", b""));
        assert!(glob_match(b"a*b*c", b"aXXbYYc"));
        assert!(!glob_match(b"a*b*c", b"aXXbYY"));
        assert!(!glob_match(b"abc", b"abcd"));
        assert!(glob_match(b"a**", b"a"));
        assert!(!glob_match(b"", b"a"));
    }
}
