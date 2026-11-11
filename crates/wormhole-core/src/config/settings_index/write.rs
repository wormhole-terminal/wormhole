//! Writing one setting from a value the settings window holds (JSON, as `config.json` spells it):
//! the one place that decides what a write is allowed to be, so every front end of the settings
//! (the GPUI window, the native one, a script) refuses the same values with the same words.

use serde_json::Value;

use super::{as_map, default_map, key_path};
use crate::config::Settings;

/// `settings` with `key` (a `config.json` key, a path for one inside an object: `motion.insert`) set
/// to `value`, then sanitised as a save would: what comes back is what sticks.
///
/// Refused, and nothing changes, for a key that is no setting and for a value its setting does not
/// read (a word that is no option, a string where a number goes). The error is a sentence to show
/// beside the control.
pub fn set(settings: &Settings, key: &str, value: Value) -> Result<Settings, String> {
    if !default_map().contains_key(key_path::top(key)) {
        return Err(format!("unknown setting `{key}`"));
    }
    let mut map = as_map(settings);
    key_path::insert(&mut map, key, value);
    serde_json::from_value::<Settings>(Value::Object(map))
        .map(Settings::sanitized)
        .map_err(|err| format!("`{key}`: {err}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::set;
    use crate::config::Settings;

    #[test]
    fn a_value_that_reads_is_set_and_clamped() {
        let defaults = Settings::default();
        let dimmer = set(&defaults, "opacity", json!(0.5)).expect("reads");
        assert_eq!(dimmer.opacity, 0.5);
        let clamped = set(&defaults, "opacity", json!(7.0)).expect("reads");
        assert_eq!(clamped.opacity, *crate::config::OPACITY.end(), "sticks as the clamp leaves it");
    }

    #[test]
    fn a_value_that_does_not_read_changes_nothing() {
        let defaults = Settings::default();
        let wrong_type = set(&defaults, "opacity", json!("dim")).expect_err("a string is no opacity");
        assert!(wrong_type.starts_with("`opacity`: "), "{wrong_type}");
        let unknown = set(&defaults, "not_a_setting", json!(1)).expect_err("no such key");
        assert_eq!(unknown, "unknown setting `not_a_setting`");
    }

    #[test]
    fn a_choice_inside_an_object_is_its_own_key() {
        let defaults = Settings::default();
        let changed = set(&defaults, "motion.insert", json!("classic")).expect("an option");
        assert_eq!(serde_json::to_value(changed.motion).expect("serialises")["insert"], json!("classic"));
        // The motion choices read a word that is no option as the default (core's rule for the object),
        // so that is no refusal.
        let unknown = set(&changed, "motion.insert", json!("no-such-motion")).expect("reads as the default");
        assert_eq!(unknown.motion, defaults.motion);
    }
}
