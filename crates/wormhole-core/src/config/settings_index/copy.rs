//! What a row hands to the clipboard: the config as the row's keys hold it now.
//!
//! "Copy as JSON" gives a snippet to paste into `config.json`: `{"blur": true}`. The values are the
//! person's own, as [`Values`] reads them, and a key reaching into an object is written as that
//! object, as the file has it: `motion.insert` is `{"motion": {"insert": "reveal"}}`. A row with
//! several keys gives them in one object, in the row's order, keys of one object together. A key
//! that holds nothing (a setting stored only while it differs from its default, when it does not)
//! is left out, not written as `null`, which the file would not read back as the default; a row
//! of keys that all hold nothing is `{}`.

use serde_json::{Map, Value};

use super::{Values, key_path};

impl Values {
    /// `keys` and their current values as one line of JSON, `{"key": value}`.
    pub fn json(&self, keys: &[&str]) -> String {
        let mut out = Map::new();
        for key in keys {
            if let Some(value) = key_path::get(&self.current, key) {
                key_path::insert(&mut out, key, value.clone());
            }
        }
        let mut line = String::new();
        write_object(&mut line, &out);
        line
    }
}

/// One line, with a space after each `:` and `,` (the shape of a hand-written snippet), in the
/// map's own order.
fn write_object(out: &mut String, map: &Map<String, Value>) {
    out.push('{');
    for (ix, (key, value)) in map.iter().enumerate() {
        if ix > 0 {
            out.push_str(", ");
        }
        write_string(out, key);
        out.push_str(": ");
        write_value(out, value);
    }
    out.push('}');
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Object(map) => write_object(out, map),
        Value::Array(items) => {
            out.push('[');
            for (ix, item) in items.iter().enumerate() {
                if ix > 0 {
                    out.push_str(", ");
                }
                write_value(out, item);
            }
            out.push(']');
        }
        // Numbers, booleans and null print as `serde_json` writes them.
        Value::String(text) => write_string(out, text),
        other => out.push_str(&other.to_string()),
    }
}

fn write_string(out: &mut String, text: &str) {
    // A `String` cannot fail to serialize.
    out.push_str(&serde_json::to_string(text).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use crate::config::Settings;
    use crate::config::settings_index::Values;
    use crate::view::motion::{FoldMotion, InsertMotion};

    fn json(settings: &Settings, keys: &[&str]) -> String {
        Values::of(settings).json(keys)
    }

    #[test]
    fn a_key_is_written_with_its_current_value_as_a_one_line_object() {
        let settings = Settings { blur: false, ..Settings::default() };
        assert_eq!(json(&settings, &["blur"]), r#"{"blur": false}"#);
        // A value that is still the default is copied too: the row asked for what it holds.
        assert_eq!(
            json(&Settings::default(), &["blur"]),
            format!(r#"{{"blur": {}}}"#, Settings::default().blur)
        );
    }

    #[test]
    fn a_text_value_is_quoted_and_escaped() {
        let settings = Settings { font_family: "My \"Mono\"".into(), ..Settings::default() };
        assert_eq!(json(&settings, &["font_family"]), r#"{"font_family": "My \"Mono\""}"#);
    }

    #[test]
    fn several_keys_share_one_object_in_the_rows_order() {
        let both = json(&Settings::default(), &["font_size", "blur"]);
        assert!(both.starts_with(r#"{"font_size": "#), "{both}");
        assert!(both.contains(r#", "blur": "#) && both.ends_with('}'), "{both}");
    }

    #[test]
    fn a_nested_key_is_written_inside_its_object_and_siblings_merge() {
        let mut settings = Settings::default();
        settings.motion.insert = InsertMotion::Classic;
        settings.motion.fold = FoldMotion::Drawer;
        assert_eq!(json(&settings, &["motion.insert"]), r#"{"motion": {"insert": "classic"}}"#);
        assert_eq!(
            json(&settings, &["motion.insert", "motion.fold"]),
            r#"{"motion": {"insert": "classic", "fold": "drawer"}}"#,
            "two keys of one object are one object"
        );
    }

    #[test]
    fn a_key_that_holds_nothing_is_left_out() {
        let settings = Settings::default();
        assert_eq!(json(&settings, &["motion.insert"]), "{}", "stored only while it differs");
        assert_eq!(json(&settings, &["no_such_key"]), "{}");
        assert_eq!(json(&settings, &[]), "{}");
        let mut some = Settings::default();
        some.motion.fold = FoldMotion::Drawer;
        assert_eq!(
            json(&some, &["motion.insert", "motion.fold"]),
            r#"{"motion": {"fold": "drawer"}}"#,
            "the one that holds something is still written"
        );
    }

    #[test]
    fn a_list_is_written_on_one_line() {
        let settings = Settings { files_pinned: vec!["a".into(), "b".into()], ..Settings::default() };
        assert_eq!(json(&settings, &["files_pinned"]), r#"{"files_pinned": ["a", "b"]}"#);
    }
}
