//! What a setting is by default, in plain words: the line the Changed page puts beside a changed row
//! ("Default: 10,000 lines"). A row whose control is a switch or a menu says it in that control's own
//! words (the page knows them); every other row falls back to this, read from the defaults of the keys the
//! row writes.

use serde_json::Value;
use wormhole_text::keys::{SETTINGS_CHANGED_DEFAULT_EMPTY, SETTINGS_CHANGED_OFF, SETTINGS_CHANGED_ON};
use wormhole_text::text;

use super::{Values, key_path};

impl Values {
    /// The defaults of `keys` as plain words, one after another: a text as it is, a number as written, a
    /// switch as On or Off, a list as its items. Empty ones are left out; where nothing is left, "none".
    pub fn default_words(&self, keys: &[&str]) -> String {
        let words: Vec<String> = keys
            .iter()
            .filter_map(|key| key_path::get(self.defaults, key))
            .map(plain)
            .filter(|words| !words.is_empty())
            .collect();
        if words.is_empty() { text(SETTINGS_CHANGED_DEFAULT_EMPTY).to_string() } else { words.join(", ") }
    }
}

fn plain(value: &Value) -> String {
    match value {
        Value::Bool(true) => text(SETTINGS_CHANGED_ON).to_string(),
        Value::Bool(false) => text(SETTINGS_CHANGED_OFF).to_string(),
        Value::String(words) => words.clone(),
        Value::Number(number) => number.to_string(),
        Value::Array(items) => {
            items.iter().map(plain).filter(|item| !item.is_empty()).collect::<Vec<_>>().join(", ")
        }
        Value::Object(_) => value.to_string(),
        Value::Null => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Settings;
    use crate::config::settings_index::Values;

    fn words(keys: &[&str]) -> String {
        Values::of(&Settings::default()).default_words(keys)
    }

    #[test]
    fn a_default_is_said_in_plain_words() {
        let defaults = Settings::default();
        assert_eq!(words(&["blur"]), if defaults.blur { "On" } else { "Off" });
        assert_eq!(words(&["tab_width"]), defaults.tab_width.to_string());
        assert_eq!(words(&["primary_color"]), "none", "an empty text has nothing to say");
        assert_eq!(words(&["no_such_key"]), "none");
        assert_eq!(words(&[]), "none");
    }

    #[test]
    fn several_keys_are_said_one_after_another() {
        let defaults = Settings::default();
        assert_eq!(
            words(&["tab_width", "primary_color", "diff_wrap"]),
            format!("{}, {}", defaults.tab_width, if defaults.diff_wrap { "On" } else { "Off" })
        );
    }
}
