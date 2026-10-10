//! Setting or removing one top-level key of a JSONC document, in place.
//!
//! A project's `config.json` is written by hand, lives in a repository and has comments and a layout
//! of its own. Saving it again from a parsed copy would lose all of that, so an edit replaces,
//! inserts or cuts out only the text of the one member, and every other byte stays as it was.

use std::ops::Range;

use serde_json::{Map, Value};

use super::{ObjectLayout, object_layout, parse};

/// `text` with the top-level `key` set to `value`, or taken out for `None`. The key's value is
/// replaced where it stands, a new key goes on the line above the closing `}` (after the last member,
/// which gets its comma), and a removed key leaves nothing behind but the comments around it. A
/// key that is in the file more than once (the last one wins when it is read) is set or removed
/// everywhere, so the file reads the same whichever one counts. Only the object's own members are
/// looked at: a key of the same name deeper down, inside a string or in a comment is not one.
///
/// Fails, and gives nothing to write, when `text` is not a JSONC object that reads in full: saving
/// over a file we could not read would lose what we did not understand. The result is read back
/// before it is returned, so it is always the same object as `text` with that one key changed.
pub fn set_member(text: &str, key: &str, value: Option<&Value>) -> Result<String, String> {
    let before: Map<String, Value> = parse(text)?;
    let layout = object_layout(text)?;
    let edited = match value {
        Some(value) => {
            let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
            put(text, &layout, key, &json)
        }
        None => cut_all(text, layout, key)?,
    };
    let mut expected = before;
    match value {
        Some(value) => {
            expected.insert(key.to_string(), value.clone());
        }
        None => {
            expected.remove(key);
        }
    }
    match parse::<Map<String, Value>>(&edited) {
        Ok(after) if after == expected => Ok(edited),
        Ok(_) => Err("the edited file does not read back as intended".to_string()),
        Err(e) => Err(format!("the edited file does not read back: {e}")),
    }
}

/// Replace the value of `key` wherever it is, or add the member when it is not there.
fn put(text: &str, layout: &ObjectLayout, key: &str, json: &str) -> String {
    let spans: Vec<(Range<usize>, String)> = layout
        .entries
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, span)| (span.clone(), json.to_string()))
        .collect();
    if spans.is_empty() { insert(text, layout, key, json) } else { apply(text, spans) }
}

/// Add `"key": json` as the last member, laid out like the first one.
fn insert(text: &str, layout: &ObjectLayout, key: &str, json: &str) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let indent = layout.key_starts.first().map_or_else(|| "  ".to_string(), |at| indent_before(text, *at));
    let key = serde_json::to_string(key).unwrap_or_else(|_| String::from("\"\""));
    let member = format!("{indent}{key}: {json}");

    // The member before it needs a comma, unless the file already ends its members with one.
    let comma = layout.entries.last().filter(|_| !layout.trailing_comma).map(|(_, last)| last.end);
    let mut ops = Vec::new();
    let line_start = text[..layout.close].rfind('\n').map_or(0, |at| at + 1);
    if line_start > 0 && text[line_start..layout.close].trim().is_empty() {
        // The `}` is on a line of its own: the member goes on the line above it. When all that
        // is between the braces is comments, a blank line keeps it from reading as their subject.
        let only_comments =
            layout.entries.is_empty() && !text[layout.open + 1..layout.close].trim().is_empty();
        let blank_above = text[..line_start].lines().next_back().is_some_and(|line| line.trim().is_empty());
        let gap = if only_comments && !blank_above { nl } else { "" };
        ops.push((line_start..line_start, format!("{gap}{member}{nl}")));
        ops.extend(comma.map(|at| (at..at, ",".to_string())));
    } else {
        // `}` shares its line with something (`{}`, `{ "a": 1 }`): break the line before it. The
        // comma is part of the same edit when the value runs right up to the brace.
        let at = text[..layout.close].trim_end_matches([' ', '\t']).len();
        let lead = if comma == Some(at) { "," } else { "" };
        ops.push((at..layout.close, format!("{lead}{nl}{member}{nl}")));
        ops.extend(comma.filter(|comma| *comma != at).map(|at| (at..at, ",".to_string())));
    }
    apply(text, ops)
}

/// `text` without any member called `key`.
fn cut_all(text: &str, mut layout: ObjectLayout, key: &str) -> Result<String, String> {
    let mut text = text.to_string();
    // One at a time from the end, reading the layout again each time: a cut moves what follows it
    // (and may take the comma of the member before it), and a repeated key is rare enough that
    // simple beats clever.
    while let Some(at) = layout.entries.iter().rposition(|(k, _)| k == key) {
        text = cut(&text, &layout, at);
        layout = object_layout(&text)?;
    }
    Ok(text)
}

/// `text` without the `at`th member: its key, value and comma, and its line when nothing else is on
/// it. A comment on the same line stays (it is the file's, not ours); the last member also takes
/// the comma of the one before it, so no comma is left dangling before the `}`.
fn cut(text: &str, layout: &ObjectLayout, at: usize) -> String {
    let start = layout.key_starts[at];
    let mut end = layout.commas[at].map_or(layout.entries[at].1.end, |comma| comma + 1);
    end += text[end..].bytes().take_while(|b| matches!(b, b' ' | b'\t')).count();
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let alone = text[line_start..start].trim().is_empty() && text[end..line_end].trim().is_empty();
    let mut ops = vec![if alone {
        (line_start..(line_end + 1).min(text.len()), String::new())
    } else {
        (start..end, String::new())
    }];
    if layout.commas[at].is_none()
        && at > 0
        && let Some(comma) = layout.commas[at - 1]
    {
        ops.push((comma..comma + 1, String::new()));
    }
    apply(text, ops)
}

/// The whitespace in front of the key at `at` when it starts its line, else two spaces.
fn indent_before(text: &str, at: usize) -> String {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let before = &text[line_start..at];
    if before.chars().all(|c| c == ' ' || c == '\t') { before.to_string() } else { "  ".to_string() }
}

/// `text` with every range replaced, applied from the end so the offsets of the others hold. Of
/// two edits at the same place the longer goes first, so an insertion lands in front of it.
fn apply(text: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(range, _)| std::cmp::Reverse((range.start, range.end)));
    let mut out = text.to_string();
    for (range, with) in edits {
        out.replace_range(range, &with);
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::set_member;

    fn set(text: &str, value: Value) -> String {
        set_member(text, "project_color", Some(&value)).unwrap_or_else(|e| panic!("{text:?}: {e}"))
    }

    fn unset(text: &str) -> String {
        set_member(text, "project_color", None).unwrap_or_else(|e| panic!("{text:?}: {e}"))
    }

    // ---- replacing -------------------------------------------------------------------------------

    #[test]
    fn a_value_is_replaced_where_it_stands_and_nothing_else_moves() {
        let text = "// the project\n{\n  \"font_size\": 15, // big\n  \"project_color\"  :  \"#2bb3a3\" /* teal */ ,\n  \"shell\": \"/bin/zsh\"\n}\n";
        assert_eq!(
            set(text, json!("teal")),
            "// the project\n{\n  \"font_size\": 15, // big\n  \"project_color\"  :  \"teal\" /* teal */ ,\n  \"shell\": \"/bin/zsh\"\n}\n"
        );
        // The same value again is the same text.
        let again = set(text, json!("#2bb3a3"));
        assert_eq!(again, text);
        // Whatever kind of value stood there.
        for old in ["1", "true", "null", "[1, {\"a\": \"]\"}]", "{\"x\": [\"}\"]}", "\"a,b\""] {
            let text = format!("{{ \"project_color\": {old}, \"b\": 2 }}");
            assert_eq!(set(&text, json!("teal")), "{ \"project_color\": \"teal\", \"b\": 2 }", "{old}");
        }
    }

    #[test]
    fn a_key_written_with_escapes_is_the_same_key() {
        let text = "{ \"project\\u005fcolor\": \"red\" }";
        assert_eq!(set(text, json!("teal")), "{ \"project\\u005fcolor\": \"teal\" }");
        assert_eq!(unset(text), "{ }");
    }

    #[test]
    fn a_key_in_the_file_twice_is_set_and_removed_everywhere() {
        let text = "{\n  \"project_color\": \"red\",\n  \"a\": 1,\n  \"project_color\": \"blue\"\n}\n";
        assert_eq!(
            set(text, json!("teal")),
            "{\n  \"project_color\": \"teal\",\n  \"a\": 1,\n  \"project_color\": \"teal\"\n}\n"
        );
        assert_eq!(unset(text), "{\n  \"a\": 1\n}\n");
        assert_eq!(unset("{\"project_color\": 1, \"project_color\": 2}"), "{}");
    }

    // ---- adding ----------------------------------------------------------------------------------

    #[test]
    fn a_new_key_goes_last_with_the_layout_of_the_first() {
        let cases = [
            // The member before gets its comma; the new one has none.
            ("{\n  \"a\": 1\n}\n", "{\n  \"a\": 1,\n  \"project_color\": \"teal\"\n}\n"),
            // The file's own trailing comma is kept as it is.
            ("{\n  \"a\": 1,\n}\n", "{\n  \"a\": 1,\n  \"project_color\": \"teal\"\n}\n"),
            // Tabs, four spaces, none.
            ("{\n\t\"a\": 1\n}", "{\n\t\"a\": 1,\n\t\"project_color\": \"teal\"\n}"),
            ("{\n    \"a\": 1\n}", "{\n    \"a\": 1,\n    \"project_color\": \"teal\"\n}"),
            ("{\n\"a\": 1\n}", "{\n\"a\": 1,\n\"project_color\": \"teal\"\n}"),
            // A comment after the last member stays on its line, and the comma goes before it.
            ("{\n  \"a\": 1 // one\n}\n", "{\n  \"a\": 1, // one\n  \"project_color\": \"teal\"\n}\n"),
            ("{\n  \"a\": 1, // one\n}\n", "{\n  \"a\": 1, // one\n  \"project_color\": \"teal\"\n}\n"),
            // Comments after the last member, on lines of their own, stay above the new one.
            (
                "{\n  \"a\": 1\n  // \"b\": 2\n}\n",
                "{\n  \"a\": 1,\n  // \"b\": 2\n  \"project_color\": \"teal\"\n}\n",
            ),
            // Not on a line of its own: the line is broken before the brace.
            ("{ \"a\": 1 }", "{ \"a\": 1,\n  \"project_color\": \"teal\"\n}"),
            ("{\"a\":1}", "{\"a\":1,\n  \"project_color\": \"teal\"\n}"),
            ("{ \"a\": 1, }", "{ \"a\": 1,\n  \"project_color\": \"teal\"\n}"),
            ("{\n  \"a\": 1 }", "{\n  \"a\": 1,\n  \"project_color\": \"teal\"\n}"),
            // Nothing to follow.
            ("{}", "{\n  \"project_color\": \"teal\"\n}"),
            ("{ }", "{\n  \"project_color\": \"teal\"\n}"),
            ("{\n}\n", "{\n  \"project_color\": \"teal\"\n}\n"),
            ("  \n{}\n\n", "  \n{\n  \"project_color\": \"teal\"\n}\n\n"),
        ];
        for (text, want) in cases {
            assert_eq!(set(text, json!("teal")), want, "{text:?}");
        }
    }

    #[test]
    fn a_file_of_only_comments_and_braces_gets_the_key_below_them() {
        let text = "// Settings.\n// Uncomment what you want.\n{\n  // \"font_size\": 15,\n  // \"shell\": \"/bin/zsh\"\n}\n";
        assert_eq!(
            set(text, json!("teal")),
            "// Settings.\n// Uncomment what you want.\n{\n  // \"font_size\": 15,\n  // \"shell\": \"/bin/zsh\"\n\n  \"project_color\": \"teal\"\n}\n"
        );
        // A blank line already there is not doubled.
        let spaced = "{\n  // \"font_size\": 15\n\n}\n";
        assert_eq!(
            set(spaced, json!("teal")),
            "{\n  // \"font_size\": 15\n\n  \"project_color\": \"teal\"\n}\n"
        );
        // And taking it out again gives the comments back, with the blank line they were given.
        let with = set(text, json!("teal"));
        assert_eq!(
            unset(&with),
            "// Settings.\n// Uncomment what you want.\n{\n  // \"font_size\": 15,\n  // \"shell\": \"/bin/zsh\"\n\n}\n"
        );
    }

    /// A commented-out `"project_color": …` is a comment, not the key: it stays, and the real key is
    /// added (or, to remove, there is nothing to remove).
    #[test]
    fn a_commented_out_key_is_not_the_key() {
        let text = "{\n  // \"project_color\": \"#2bb3a3\",\n  /* \"project_color\": 1, */\n  \"a\": 1\n}\n";
        assert_eq!(
            set(text, json!("teal")),
            "{\n  // \"project_color\": \"#2bb3a3\",\n  /* \"project_color\": 1, */\n  \"a\": 1,\n  \"project_color\": \"teal\"\n}\n"
        );
        assert_eq!(unset(text), text);
    }

    #[test]
    fn a_key_deeper_down_or_inside_a_string_is_not_the_key() {
        let text = "{\n  \"env\": { \"project_color\": \"x\" },\n  \"note\": \"\\\"project_color\\\": 1\",\n  \"list\": [{\"project_color\": 2}]\n}\n";
        let added = set(text, json!("teal"));
        assert_eq!(
            added,
            format!(
                "{},\n  \"project_color\": \"teal\"\n}}\n",
                text.trim_end().strip_suffix("\n}").unwrap_or("")
            )
        );
        let parsed: Value = crate::jsonc::parse(&added).expect("reads");
        assert_eq!(parsed["env"], json!({"project_color": "x"}), "the nested one is untouched");
        assert_eq!(parsed["list"], json!([{"project_color": 2}]));
        assert_eq!(parsed["project_color"], json!("teal"));
        assert_eq!(unset(text), text, "nothing at the top level to remove");
        assert_eq!(unset(&added), text);
        // Replacing changes the top-level one only.
        let replaced = set(&added, json!("red"));
        assert_eq!(crate::jsonc::parse::<Value>(&replaced).expect("reads")["project_color"], json!("red"));
        assert_eq!(
            crate::jsonc::parse::<Value>(&replaced).expect("reads")["env"],
            json!({"project_color": "x"})
        );
    }

    #[test]
    fn line_endings_and_a_byte_order_mark_are_kept() {
        let text = "\u{feff}{\r\n  \"a\": 1\r\n}\r\n";
        let added = set(text, json!("teal"));
        assert_eq!(added, "\u{feff}{\r\n  \"a\": 1,\r\n  \"project_color\": \"teal\"\r\n}\r\n");
        assert_eq!(unset(&added), text);
        assert_eq!(set("{\r\n}\r\n", json!(true)), "{\r\n  \"project_color\": true\r\n}\r\n");
        // Only a file that uses CRLF gets it.
        assert!(!set("{\n  \"a\": 1\n}\n", json!(1)).contains('\r'));
    }

    #[test]
    fn any_json_value_can_be_written() {
        for (value, written) in [
            (json!("a \"quoted\" \\ name\n"), r#""a \"quoted\" \\ name\n""#),
            (json!(true), "true"),
            (json!(null), "null"),
            (json!(1.5), "1.5"),
            (json!({"a": [1, 2]}), r#"{"a":[1,2]}"#),
        ] {
            assert_eq!(set("{}", value), format!("{{\n  \"project_color\": {written}\n}}"));
        }
    }

    // ---- removing --------------------------------------------------------------------------------

    #[test]
    fn a_removed_key_takes_its_line_and_comma_and_nothing_else() {
        let cases = [
            // The only member.
            ("{\n  \"project_color\": \"teal\"\n}\n", "{\n}\n"),
            ("{ \"project_color\": \"teal\" }", "{ }"),
            ("{\"project_color\":\"teal\"}", "{}"),
            // First, middle, last.
            (
                "{\n  \"project_color\": \"teal\",\n  \"a\": 1,\n  \"b\": 2\n}\n",
                "{\n  \"a\": 1,\n  \"b\": 2\n}\n",
            ),
            (
                "{\n  \"a\": 1,\n  \"project_color\": \"teal\",\n  \"b\": 2\n}\n",
                "{\n  \"a\": 1,\n  \"b\": 2\n}\n",
            ),
            (
                "{\n  \"a\": 1,\n  \"b\": 2,\n  \"project_color\": \"teal\"\n}\n",
                "{\n  \"a\": 1,\n  \"b\": 2\n}\n",
            ),
            // The file's trailing comma stays a trailing comma.
            ("{\n  \"a\": 1,\n  \"project_color\": \"teal\",\n}\n", "{\n  \"a\": 1,\n}\n"),
            // Several to a line.
            ("{ \"a\": 1, \"project_color\": \"teal\", \"b\": 2 }", "{ \"a\": 1, \"b\": 2 }"),
            ("{ \"a\": 1, \"project_color\": \"teal\" }", "{ \"a\": 1 }"),
            ("{\"project_color\":\"teal\",\"a\":1}", "{\"a\":1}"),
            ("{\"a\":1,\"project_color\":\"teal\"}", "{\"a\":1}"),
            // A comment on the same line is the file's: it stays.
            (
                "{\n  \"a\": 1,\n  \"project_color\": \"teal\", // the tint\n  \"b\": 2\n}\n",
                "{\n  \"a\": 1,\n  // the tint\n  \"b\": 2\n}\n",
            ),
            (
                "{\n  \"a\": 1, // one\n  \"project_color\": \"teal\" // the tint\n}\n",
                "{\n  \"a\": 1 // one\n  // the tint\n}\n",
            ),
            // So do the comments above and below it.
            (
                "{\n  // colour\n  \"project_color\": \"teal\",\n  // size\n  \"a\": 1\n}\n",
                "{\n  // colour\n  // size\n  \"a\": 1\n}\n",
            ),
            // A value over several lines.
            (
                "{\n  \"a\": 1,\n  \"project_color\": {\n    \"x\": [1,\n 2]\n  },\n  \"b\": 2\n}\n",
                "{\n  \"a\": 1,\n  \"b\": 2\n}\n",
            ),
            // Not there: the same text, byte for byte.
            ("{\n  \"a\": 1 // x\n}\n", "{\n  \"a\": 1 // x\n}\n"),
            ("{}", "{}"),
        ];
        for (text, want) in cases {
            assert_eq!(unset(text), want, "{text:?}");
        }
    }

    // ---- refusing --------------------------------------------------------------------------------

    #[test]
    fn what_is_not_an_object_that_reads_in_full_is_refused() {
        for text in [
            "",
            "   ",
            "// only a comment\n",
            "[]",
            "[1, 2]",
            "\"project_color\"",
            "null",
            "1",
            "{",
            "{\"a\": }",
            "{\"a\" 1}",
            "{\"a\": 1} x",
            "{\"a\": 1}{\"b\": 2}",
            "{\"a\": \"unterminated}",
            "{\"a\": 1 /* never closed }",
            "{a: 1}",
            "{\"a\": 1,, \"b\": 2}",
        ] {
            for value in [Some(json!("teal")), None] {
                let err = set_member(text, "project_color", value.as_ref());
                assert!(err.is_err(), "{text:?} was edited into {err:?}");
            }
        }
    }

    // ---- the whole matrix ------------------------------------------------------------------------

    /// Every combination of layout, position of the key and what is around it: the edit reads back as
    /// the object it started from with only that key changed, and every comment is still there.
    #[test]
    fn every_layout_reads_back_with_only_the_key_changed_and_keeps_its_comments() {
        // Separators between members (also the end), as the layouts a person might write.
        let layouts: [(&str, &str, &str, &str); 6] = [
            // (open, between, last separator, close)
            ("{", ",", "", "}"),
            ("{ ", ", ", " ", "}"),
            ("{\n  ", ",\n  ", "\n", "}\n"),
            ("{\n  ", ",\n  ", ",\n", "}\n"),
            ("{\r\n\t", ",\r\n\t", "\r\n", "}\r\n"),
            ("// head\n{ // brace\n  ", ", // next\n  ", " // tail\n", "}\n// end\n"),
        ];
        let members = [
            "\"a\": 1",
            "\"project_color\": \"red\"",
            "\"b\": [1, {\"project_color\": 2}]",
            "\"c\": \"x, // not a comment\"",
        ];
        let mut count = 0;
        for (open, between, last, close) in layouts {
            for mask in 0u32..(1 << members.len()) {
                let picked: Vec<&str> =
                    (0..members.len()).filter(|i| mask & (1 << i) != 0).map(|i| members[i]).collect();
                // (A trailing comma needs a member to follow.)
                let tail = if picked.is_empty() { last.trim_start_matches(',') } else { last };
                let text = format!("{open}{}{tail}{close}", picked.join(between));
                let before: Value = crate::jsonc::parse(&text).expect("a valid document");
                let comments = text.matches("// ").count() - text.matches("// not a comment").count();
                for value in [Some(json!("teal")), Some(json!(false)), None] {
                    let after = set_member(&text, "project_color", value.as_ref())
                        .unwrap_or_else(|e| panic!("{text:?} {value:?}: {e}"));
                    let mut want = before.clone();
                    match &value {
                        Some(value) => {
                            want["project_color"] = value.clone();
                        }
                        None => {
                            want.as_object_mut().map(|o| o.remove("project_color"));
                        }
                    }
                    assert_eq!(
                        crate::jsonc::parse::<Value>(&after).expect("reads"),
                        want,
                        "{text:?} -> {after:?}"
                    );
                    let kept = after.matches("// ").count() - after.matches("// not a comment").count();
                    assert_eq!(kept, comments, "{text:?} -> {after:?}");
                    count += 1;
                }
            }
        }
        assert_eq!(count, 6 * 16 * 3);
    }
}
