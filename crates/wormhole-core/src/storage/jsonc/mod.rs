//! JSON with comments (`//`, `/* */`) and trailing commas, as used by theme
//! files. Comments and trailing commas are blanked out with spaces rather than
//! removed so serde_json's line/column positions still point into the
//! original text. A leading UTF-8 byte order mark (some Windows editors add
//! one) is blanked the same way.
//!
//! [`object_layout`] finds where each top-level value sits, so a file can be
//! edited in place (keeping its comments) instead of being re-serialized;
//! [`set_member`] does that for one key.

mod edit;
pub(crate) mod suggest;

use std::ops::Range;

use serde::de::DeserializeOwned;

pub use edit::set_member;

/// Parse JSONC text into `T`. Errors read `line L, column C: message`.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    let clean = strip(text)?;
    serde_json::from_str(&clean).map_err(|e| {
        let msg = e.to_string();
        // serde_json appends " at line L column C"; we put the position first
        // instead, which reads better in a settings error banner.
        let msg = msg.rfind(" at line ").map_or(msg.as_str(), |i| &msg[..i]);
        if e.line() == 0 {
            msg.to_string()
        } else {
            format!("line {}, column {}: {msg}", e.line(), e.column())
        }
    })
}

/// Replace comments and trailing commas with whitespace (newlines kept).
pub fn strip(text: &str) -> Result<String, String> {
    let without_comments = strip_comments(text)?;
    Ok(strip_trailing_commas(&without_comments))
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte].bytes().filter(|&b| b == b'\n').count() + 1
}

/// Blank a character: keep newlines (for line numbers), and keep the byte
/// length so byte-based positions of later tokens stay identical.
fn blank(out: &mut String, c: char) {
    if c == '\n' {
        out.push('\n');
    } else {
        out.extend(std::iter::repeat_n(' ', c.len_utf8()));
    }
}

fn strip_comments(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    if let Some(&(_, '\u{feff}')) = chars.peek() {
        chars.next();
        blank(&mut out, '\u{feff}');
    }
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                // Copy the string literal verbatim; `\"` must not end it.
                let mut closed = false;
                while let Some((_, s)) = chars.next() {
                    out.push(s);
                    if s == '\\' {
                        if let Some((_, esc)) = chars.next() {
                            out.push(esc);
                        }
                    } else if s == '"' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(format!("line {}: unterminated string", line_of(text, i)));
                }
            }
            '/' if matches!(chars.peek(), Some((_, '/'))) => {
                blank(&mut out, c);
                while let Some(&(_, s)) = chars.peek() {
                    if s == '\n' {
                        break;
                    }
                    blank(&mut out, s);
                    chars.next();
                }
            }
            '/' if matches!(chars.peek(), Some((_, '*'))) => {
                blank(&mut out, c);
                if let Some((_, star)) = chars.next() {
                    blank(&mut out, star);
                }
                let mut closed = false;
                while let Some((_, s)) = chars.next() {
                    blank(&mut out, s);
                    if s == '*' && matches!(chars.peek(), Some((_, '/'))) {
                        if let Some((_, slash)) = chars.next() {
                            blank(&mut out, slash);
                        }
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(format!("line {}: unterminated /* comment", line_of(text, i)));
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

/// Runs after comment stripping, so only strings need special care here.
fn strip_trailing_commas(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            out.push(c);
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            ',' => {
                let next = bytes[i + 1..].iter().find(|b| !b.is_ascii_whitespace());
                if matches!(next, Some(b'}' | b']')) {
                    out.push(' ');
                } else {
                    out.push(',');
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Where the members of a top-level JSON object sit in the original text.
#[derive(Debug, PartialEq)]
pub(crate) struct ObjectLayout {
    /// Byte offset of the opening `{`.
    pub open: usize,
    /// Each member's key and the byte range of its value, in file order.
    pub entries: Vec<(String, Range<usize>)>,
    /// Byte offset of each member's key (its opening quote), in the same order.
    pub key_starts: Vec<usize>,
    /// Byte offset of the comma after each member, if it has one (the last may or may not).
    pub commas: Vec<Option<usize>>,
    /// Byte offset of the closing `}`.
    pub close: usize,
    /// Whether the last member is followed by a (trailing) comma.
    pub trailing_comma: bool,
}

/// The layout of a JSONC document whose top level is an object. Only finds
/// spans: it checks the structure, not that every value is valid JSON.
pub(crate) fn object_layout(text: &str) -> Result<ObjectLayout, String> {
    let clean = strip_comments(text)?;
    let b = clean.as_bytes();
    let err = |at: usize, what: &str| format!("line {}: expected {what}", line_of(text, at.min(text.len())));
    let skip_ws = |mut i: usize| {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        i
    };
    let mut i = skip_ws(0);
    if b.get(i) != Some(&b'{') {
        return Err(err(i, "`{`"));
    }
    let open = i;
    let mut entries = Vec::new();
    let mut key_starts = Vec::new();
    let mut commas = Vec::new();
    let mut trailing_comma = false;
    i = skip_ws(i + 1);
    loop {
        match b.get(i) {
            Some(b'}') => break,
            Some(b'"') => {}
            _ => return Err(err(i, "a key or `}`")),
        }
        key_starts.push(i);
        let key_end = string_end(b, i).ok_or_else(|| err(i, "a closing `\"`"))?;
        let key: String = serde_json::from_str(&clean[i..key_end]).map_err(|e| e.to_string())?;
        i = skip_ws(key_end);
        if b.get(i) != Some(&b':') {
            return Err(err(i, "`:`"));
        }
        let start = skip_ws(i + 1);
        let end = value_end(b, start).ok_or_else(|| err(start, "a value"))?;
        entries.push((key, start..end));
        i = skip_ws(end);
        trailing_comma = false;
        match b.get(i) {
            Some(b',') => {
                trailing_comma = true;
                commas.push(Some(i));
                i = skip_ws(i + 1);
            }
            Some(b'}') => {
                commas.push(None);
                break;
            }
            _ => return Err(err(i, "`,` or `}`")),
        }
    }
    let close = i;
    if skip_ws(close + 1) != b.len() {
        return Err(err(close + 1, "the end of the file after `}`"));
    }
    Ok(ObjectLayout { open, entries, key_starts, commas, close, trailing_comma })
}

/// The offset just past the string literal starting at `start` (a `"`).
fn string_end(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// The offset just past the JSON value starting at `start`.
fn value_end(b: &[u8], start: usize) -> Option<usize> {
    match b.get(start)? {
        b'"' => string_end(b, start),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut i = start;
            while i < b.len() {
                match b[i] {
                    b'"' => {
                        i = string_end(b, i)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        b',' | b'}' | b']' | b':' => None,
        _ => {
            let len = b[start..].iter().position(|c| c.is_ascii_whitespace() || b",}]".contains(c));
            Some(start + len.unwrap_or(b.len() - start))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn p(s: &str) -> Result<Value, String> {
        parse(s)
    }

    #[test]
    fn plain_json() {
        assert_eq!(p(r#"{"a": [1, 2, {"b": null}]}"#).unwrap(), json!({"a": [1, 2, {"b": null}]}));
    }

    #[test]
    fn comments() {
        let src = "// leading\n{\n  \"a\": 1, // trailing\n  /* block\n  spanning */ \"b\": /* inline */ 2\n}\n// end";
        assert_eq!(p(src).unwrap(), json!({"a": 1, "b": 2}));
        assert_eq!(p("{\"a\":1/**/}").unwrap(), json!({"a": 1}));
        assert_eq!(p("{\"a\":1/***/}").unwrap(), json!({"a": 1}));
        assert_eq!(p("{\"a\":1}//").unwrap(), json!({"a": 1}));
    }

    #[test]
    fn comment_markers_inside_strings_are_kept() {
        let src = r#"{"url": "http://x//y", "c": "/* not */", "q": "a\"//b", "bs": "\\", "z": 1}"#;
        assert_eq!(
            p(src).unwrap(),
            json!({"url": "http://x//y", "c": "/* not */", "q": "a\"//b", "bs": "\\", "z": 1})
        );
    }

    #[test]
    fn trailing_commas() {
        assert_eq!(p("{\"a\": [1, 2, ], \"b\": {\"c\": 1,},}").unwrap(), json!({"a": [1, 2], "b": {"c": 1}}));
        assert_eq!(p("[1, // x\n /* y */ ]").unwrap(), json!([1]));
        // Commas inside strings are data.
        assert_eq!(p(r#"{"a": ",}"}"#).unwrap(), json!({"a": ",}"}));
        assert_eq!(p(r#"{"a": "\\",}"#).unwrap(), json!({"a": "\\"}));
    }

    #[test]
    fn unicode_is_preserved_and_blanked() {
        let src = "{\n // ünïcødé 🎨\n \"é\": \"🎨\", /* ☃ */\n}";
        assert_eq!(p(src).unwrap(), json!({"é": "🎨"}));
        // Blanking keeps byte offsets, so the stripped text has the same length.
        assert_eq!(strip(src).unwrap().len(), src.len());
    }

    #[test]
    fn errors_have_line_numbers() {
        let e = p("{\n  \"a\": 1,\n  \"b\": oops\n}").unwrap_err();
        assert!(e.starts_with("line 3, column"), "{e}");
        let e = p("{\n\n/* never closed").unwrap_err();
        assert_eq!(e, "line 3: unterminated /* comment");
        let e = p("{\n\"abc").unwrap_err();
        assert_eq!(e, "line 2: unterminated string");
        // Comment stripping preserves line numbers for later errors.
        let e = p("/* one\ntwo\nthree */ {\"a\": }").unwrap_err();
        assert!(e.starts_with("line 3,"), "{e}");
        // Type errors from serde are positioned too.
        let e = parse::<Vec<u32>>("[1,\n\"x\"]").unwrap_err();
        assert!(e.starts_with("line 2,") && e.contains("expected u32"), "{e}");
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(p("").is_err());
        assert!(p("// only a comment").is_err());
    }

    #[test]
    fn a_byte_order_mark_is_ignored() {
        let src = "\u{feff}{\n  \"a\": 1\n}";
        assert_eq!(p(src).unwrap(), json!({"a": 1}));
        assert_eq!(strip(src).unwrap().len(), src.len());
        // Positions after it still count bytes from the start of the file.
        let e = p("\u{feff}{\"a\": x}").unwrap_err();
        assert!(e.starts_with("line 1, column 10"), "{e}");
        // Only a leading one; elsewhere it is an error, as in JSON.
        assert!(p("{\"a\": \u{feff}1}").is_err());
    }

    #[test]
    fn object_layout_finds_value_spans() {
        let src = "\u{feff}// lead\n{\n  \"a\": 1, // one\n  \"b\" : \"x,}\\\"\" /* two */,\n  \"c\": {\"d\": [1, {\"e\": \"]\"}]},\n  \"\\u0066\": true,\n}\n// end\n";
        let layout = object_layout(src).unwrap();
        let values: Vec<(&str, &str)> =
            layout.entries.iter().map(|(k, r)| (k.as_str(), &src[r.clone()])).collect();
        assert_eq!(
            values,
            [("a", "1"), ("b", "\"x,}\\\"\""), ("c", "{\"d\": [1, {\"e\": \"]\"}]}"), ("f", "true")]
        );
        assert_eq!(&src[layout.close..layout.close + 1], "}");
        assert_eq!(&src[layout.open..layout.open + 1], "{");
        assert!(layout.trailing_comma);
        // Each member's key starts at its opening quote, and its comma is the one after its value
        // (past a comment, if one sits between).
        let keys: Vec<&str> = layout.key_starts.iter().map(|at| &src[*at..*at + 2]).collect();
        assert_eq!(keys, ["\"a", "\"b", "\"c", "\"\\"]);
        let commas: Vec<Option<char>> =
            layout.commas.iter().map(|at| at.map(|at| char::from(src.as_bytes()[at]))).collect();
        assert_eq!(commas, [Some(','); 4]);
        assert_eq!(object_layout("{\"a\": 1, \"b\": 2}").unwrap().commas, [Some(7), None]);
        let tight = object_layout("{\"a\":-1.5e3}").unwrap();
        assert_eq!(tight.entries, [("a".to_string(), 5..11)]);
        assert!(!tight.trailing_comma);
        assert_eq!(
            object_layout(" {} ").unwrap(),
            ObjectLayout {
                open: 1,
                entries: vec![],
                key_starts: vec![],
                commas: vec![],
                close: 2,
                trailing_comma: false
            }
        );
        for bad in ["[1]", "{\"a\" 1}", "{\"a\": 1} x", "{\"a\": }", "{\"a\": 1 \"b\": 2}", "{", ""] {
            assert!(object_layout(bad).is_err(), "{bad}");
        }
    }
}
