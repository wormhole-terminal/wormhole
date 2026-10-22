//! The text `config.json` is saved as. An existing file is edited in place:
//! only the values that changed are replaced and missing keys are appended, so
//! the user's comments, `$schema`, unknown keys and key order survive a save.

use serde::Serialize;
use serde_json::{Map, Value};

use super::settings::Settings;
use crate::storage::jsonc;

/// A new file: every setting, in declaration order.
pub(super) fn fresh_text(settings: &Settings) -> Result<String, String> {
    serde_json::to_string_pretty(settings).map(|text| text + "\n").map_err(|e| e.to_string())
}

/// `old` (the file as it is on disk) with `settings` saved into it. Fails when
/// `old` is not a settings file we can read in full: overwriting it would
/// lose what we could not read.
pub(super) fn updated_text(old: &str, settings: &Settings) -> Result<String, String> {
    let current: Settings = jsonc::parse(old)?;
    let patched = patch(old, &current, settings);
    match patched {
        Ok(text) if saves(&text, old, settings) => Ok(text),
        // Never expected; the full rewrite still keeps everything but comments.
        outcome => {
            let reason = outcome.err().unwrap_or_else(|| "the edited file did not read back".into());
            tracing::warn!("config.json: rewriting in full instead of editing in place: {reason}");
            rewritten_text(old, settings)
        }
    }
}

/// Replace changed values and append missing keys, leaving everything else
/// byte for byte.
fn patch(old: &str, current: &Settings, settings: &Settings) -> Result<String, String> {
    let layout = jsonc::object_layout(old)?;
    let new_values = as_map(settings)?;
    let old_values = as_map(current)?;
    let mut replacements = Vec::new();
    let mut missing = Vec::new();
    for key in declaration_order(settings)? {
        let Some(value) = new_values.get(&key) else { continue };
        // Strict parsing rejects duplicate settings, so a key is in the file at most once.
        match layout.entries.iter().find(|(k, _)| *k == key) {
            Some((_, span)) if old_values.get(&key) != Some(value) => {
                replacements.push((span.clone(), serde_json::to_string(value).map_err(|e| e.to_string())?));
            }
            Some(_) => {}
            None => missing.push((key, value)),
        }
    }

    let mut text = old.to_string();
    if !missing.is_empty() {
        let indent = layout.entries.first().map_or("  ".to_string(), |(_, span)| indent_of(old, span.start));
        let mut members = Vec::new();
        for (key, value) in &missing {
            let key = serde_json::to_string(key).map_err(|e| e.to_string())?;
            let value = serde_json::to_string(value).map_err(|e| e.to_string())?;
            members.push(format!("{indent}{key}: {value}"));
        }
        let mut members = members.join(",\n");
        let line_start = old[..layout.close].rfind('\n').map_or(0, |i| i + 1);
        if old[line_start..layout.close].trim().is_empty() && line_start > 0 {
            // `}` on a line of its own: new members go on the lines above it.
            members.push('\n');
            text.insert_str(line_start, &members);
        } else {
            text.insert_str(layout.close, &format!("\n{members}\n"));
        }
        if let Some((_, last)) = layout.entries.last()
            && !layout.trailing_comma
        {
            replacements.push((last.end..last.end, ",".into()));
        }
    }
    // Back to front, so earlier offsets stay valid.
    replacements.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    for (span, value) in replacements {
        text.replace_range(span, &value);
    }
    Ok(text)
}

/// Whether `text` holds exactly `settings` and every other key of `old`.
fn saves(text: &str, old: &str, settings: &Settings) -> bool {
    let others = |text: &str| -> Option<Map<String, Value>> {
        let mut map: Map<String, Value> = jsonc::parse(text).ok()?;
        let known = as_map(settings).ok()?;
        map.retain(|k, _| !known.contains_key(k));
        Some(map)
    };
    jsonc::parse::<Settings>(text).is_ok_and(|saved| saved == *settings)
        && others(text).is_some()
        && others(text) == others(old)
}

/// The whole file again: `$schema` first, then every setting, then the keys
/// we don't know (kept for whatever reads them). Comments are lost.
fn rewritten_text(old: &str, settings: &Settings) -> Result<String, String> {
    #[derive(Serialize)]
    struct OnDisk<'a> {
        #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
        schema: Option<&'a Value>,
        #[serde(flatten)]
        settings: &'a Settings,
        #[serde(flatten)]
        unknown: &'a Map<String, Value>,
    }
    let mut unknown: Map<String, Value> = jsonc::parse(old)?;
    let known = as_map(settings)?;
    unknown.retain(|k, _| !known.contains_key(k));
    let schema = unknown.remove("$schema");
    let file = OnDisk { schema: schema.as_ref(), settings, unknown: &unknown };
    serde_json::to_string_pretty(&file).map(|text| text + "\n").map_err(|e| e.to_string())
}

fn as_map(settings: &Settings) -> Result<Map<String, Value>, String> {
    match serde_json::to_value(settings) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("settings did not serialize to an object".into()),
        Err(e) => Err(e.to_string()),
    }
}

/// Setting keys in the order `Settings` declares them (a `Map` sorts them).
fn declaration_order(settings: &Settings) -> Result<Vec<String>, String> {
    let text = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    Ok(jsonc::object_layout(&text)?.entries.into_iter().map(|(key, _)| key).collect())
}

/// The whitespace before the member whose value starts at `value_start`,
/// when that member begins its line; otherwise two spaces.
fn indent_of(text: &str, value_start: usize) -> String {
    let line_start = text[..value_start].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[line_start..value_start];
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    if line[indent.len()..].starts_with('"') { indent } else { "  ".into() }
}

#[cfg(test)]
mod tests {
    use super::{fresh_text, rewritten_text, updated_text};
    use crate::config::Settings;
    use crate::storage::jsonc;

    fn changed(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    #[test]
    fn a_fresh_file_has_every_setting_in_order() {
        let text = fresh_text(&Settings::default()).unwrap();
        assert!(text.starts_with("{\n  \"theme\": \"Wormhole Dark\",\n  \"primary_color\": \"\","), "{text}");
        assert!(text.ends_with("}\n"));
        assert_eq!(jsonc::parse::<Settings>(&text).unwrap(), Settings::default());
    }

    #[test]
    fn saving_edits_values_in_place_and_keeps_the_rest() {
        let old = "\u{feff}{\n  \"$schema\": \"https://example.com/config.json\",\n  // My theme.\n  \"theme\": \"Nord\", /* keep */\n  \"x-mine\": { \"a\": [1, 2] },\n  \"font_size\": 14\n}\n// end\n";
        let settings = changed(|s| {
            s.theme = "Dracula".into();
            s.diff_wrap = true;
        });
        let new = updated_text(old, &settings).unwrap();
        // Untouched text is byte for byte the same; the changed value is replaced.
        assert!(new.starts_with("\u{feff}{\n  \"$schema\": \"https://example.com/config.json\",\n  // My theme.\n  \"theme\": \"Dracula\", /* keep */\n  \"x-mine\": { \"a\": [1, 2] },\n  \"font_size\": 14,\n"), "{new}");
        assert!(new.ends_with("  \"worktree_base\": \"\",\n  \"feature_flags\": {}\n}\n// end\n"), "{new}");
        // `14` reads back as the same setting, so it is not rewritten as `14.0`.
        assert!(!new.contains("14.0"));
        assert!(new.contains("\n  \"diff_wrap\": true,\n"));
        assert_eq!(jsonc::parse::<Settings>(&new).unwrap(), settings);
        // Saving the same settings again changes nothing.
        assert_eq!(updated_text(&new, &settings).unwrap(), new);
    }

    #[test]
    fn appending_works_with_any_layout() {
        let settings = changed(|s| s.font_size = 16.0);
        for old in [
            "{}",
            "{ }\n",
            "{\"theme\":\"Nord\"}",
            "{\n\t\"theme\": \"Nord\",\n}",
            "{\n  \"theme\": \"Nord\" // last\n  // trailing\n}",
            "  {\"theme\": \"Nord\", \"font_size\": 12}  ",
        ] {
            let new = updated_text(old, &settings).unwrap();
            let read: Settings = jsonc::parse(&new).unwrap_or_else(|e| panic!("{old:?} -> {new:?}: {e}"));
            assert_eq!(read, settings, "{old:?} -> {new}");
        }
        let tabs = updated_text("{\n\t\"theme\": \"Nord\",\n}", &settings).unwrap();
        assert!(tabs.contains("\n\t\"font_size\": 16.0,\n"), "{tabs}");
        let comment = updated_text("{\n  \"theme\": \"Nord\" // last\n}", &settings).unwrap();
        assert!(
            comment.starts_with("{\n  \"theme\": \"Wormhole Dark\", // last\n  \"primary_color\": \"\","),
            "{comment}"
        );
    }

    #[test]
    fn a_file_that_does_not_read_in_full_is_not_saved_over() {
        let settings = Settings::default();
        assert!(updated_text("{ \"font_size\": \"16\" }", &settings).is_err());
        assert!(updated_text("{ nope", &settings).is_err());
        assert!(updated_text("[]", &settings).is_err());
    }

    #[test]
    fn the_full_rewrite_keeps_schema_and_unknown_keys() {
        let old = "{ \"x-mine\": 1, \"$schema\": \"s\", \"theme\": \"Nord\" }";
        let text = rewritten_text(old, &Settings::default()).unwrap();
        assert!(text.starts_with("{\n  \"$schema\": \"s\",\n  \"theme\": \"Wormhole Dark\","), "{text}");
        assert!(text.ends_with("  \"x-mine\": 1\n}\n"), "{text}");
        assert_eq!(jsonc::parse::<Settings>(&text).unwrap(), Settings::default());
    }
}
