//! What a project's `.wormhole/config.json` may change: how its terminals look and behave, but
//! not the app as a whole, nor the theme (that is the project's `theme.json`).

use serde_json::{Map, Value};
use wormhole_core::config::PROJECT_SETTINGS;

/// `overlay` without the keys a project may not set.
pub(super) fn project_scoped(overlay: Map<String, Value>) -> Map<String, Value> {
    overlay.into_iter().filter(|(key, _)| PROJECT_SETTINGS.contains(&key.as_str())).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::project_scoped;

    // Which settings are a project's to change is decided (and checked to cover every
    // setting) in core: `config::PROJECT_SETTINGS` / `APP_SETTINGS`.
    #[test]
    fn a_project_cannot_change_the_app_as_a_whole() {
        let overlay = json!({
            "theme": "Nord",
            "theme_mode": "light",
            "theme_light": "GitHub Light",
            "primary_color": "#a3be8c",
            "diff_wrap": true,
            "hotkey": "ctrl-`",
            "menu_bar": false,
            "auto_update": false,
            "notifications": false,
            "not_a_setting": 1
        });
        let kept = project_scoped(overlay.as_object().cloned().unwrap_or_default());
        let mut keys: Vec<&str> = kept.keys().map(String::as_str).collect();
        keys.sort_unstable();
        // The theme is the project's theme.json, never its config.json.
        assert_eq!(keys, ["diff_wrap", "primary_color"]);
    }
}
