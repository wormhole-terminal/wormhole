//! Layering a project's partial `config.json` over the person's settings.

use super::Settings;

impl Settings {
    /// These settings with a partial JSON object (e.g. a project's
    /// `.wormhole/config.json`) layered on top. Unknown or mistyped keys are
    /// ignored rather than failing the whole overlay.
    pub fn with_overrides(&self, overrides: &serde_json::Map<String, serde_json::Value>) -> Settings {
        self.with_overrides_reported(overrides).0
    }

    /// [`with_overrides`](Self::with_overrides), and the keys the overlay really set (known
    /// keys whose values read), sorted: what the settings window marks as set by a project.
    pub fn with_overrides_reported(
        &self,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> (Settings, Vec<String>) {
        let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(self) else {
            return (self.clone(), Vec::new());
        };
        let mut applied = Vec::new();
        for (key, value) in overrides {
            if let Some(slot) = merged.get_mut(key) {
                let previous = std::mem::replace(slot, value.clone());
                if serde_json::from_value::<Settings>(serde_json::Value::Object(merged.clone())).is_err() {
                    merged.insert(key.clone(), previous);
                } else {
                    applied.push(key.clone());
                }
            }
        }
        // A project that sets a primary color means it, whatever the user's switch says (its
        // `theme_primary` still wins).
        if applied.iter().any(|k| k == "primary_color")
            && !overrides.contains_key("theme_primary")
            && let Some(serde_json::Value::String(color)) = overrides.get("primary_color")
        {
            merged.insert("theme_primary".into(), serde_json::Value::Bool(color.trim().is_empty()));
        }
        applied.sort();
        match serde_json::from_value::<Settings>(serde_json::Value::Object(merged)) {
            Ok(settings) => (settings.sanitized(), applied),
            Err(_) => (self.clone(), Vec::new()),
        }
    }
}
