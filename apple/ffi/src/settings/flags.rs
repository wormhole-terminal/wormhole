//! The Feature flags page and which pages the nav shows: core's rules (`features::FlagMode`, `SectionId::
//! is_visible`) over the profile's settings, so a flag turned on makes its page appear at once, and a release
//! that has the flags locked neither lists them nor reads the switches.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use wormhole_core::config::features::{self, FeatureFlag};
use wormhole_core::config::settings_index::SectionId;

use super::error::SettingsError;
use super::handle::SettingsHandle;
use crate::settings_dump::section_id;

/// One switch of the page.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SettingsFlagRow {
    pub key: String,
    pub title: String,
    pub description: String,
    pub on: bool,
    /// What the flag is with no entry in `config.json`: the row is "changed" when `on` differs.
    pub default: bool,
}

fn write(handle: &SettingsHandle, flags: &BTreeMap<String, bool>) -> Result<(), SettingsError> {
    let value: Value = json!(flags);
    handle.set("feature_flags".into(), value.to_string())
}

#[uniffi::export]
impl SettingsHandle {
    /// A row for each flag of this build.
    pub fn feature_flags(&self) -> Vec<SettingsFlagRow> {
        let state = self.shared.lock();
        let settings = state.effective();
        let mode = settings.flag_mode();
        FeatureFlag::ALL
            .iter()
            .map(|flag| SettingsFlagRow {
                key: flag.key().to_string(),
                title: flag.title().to_string(),
                description: flag.description().to_string(),
                on: settings.feature_enabled(*flag),
                default: mode.default_for(*flag),
            })
            .collect()
    }

    /// Switches a flag. Going back to what the build does takes the entry out instead of writing a redundant one.
    pub fn set_feature_flag(&self, key: String, on: bool) -> Result<(), SettingsError> {
        let flag = FeatureFlag::from_key(&key)
            .ok_or_else(|| SettingsError::Invalid { reason: format!("no feature flag `{key}`") })?;
        let (mut flags, default) = {
            let state = self.shared.lock();
            let settings = state.effective();
            (settings.feature_flags.clone(), settings.flag_mode().default_for(flag))
        };
        if on == default {
            flags.remove(&key);
        } else {
            flags.insert(key, on);
        }
        write(self, &flags)
    }

    /// Unlocks the flags in a Stable release (the page shows, the map is read), or locks them again: the
    /// unlisted ⌃⌥⌘F. Returns whether they are unlocked now.
    pub fn toggle_flags_unlocked(&self) -> Result<bool, SettingsError> {
        let mut flags = self.shared.lock().effective().feature_flags.clone();
        let unlock = !features::unlocked(&flags);
        if unlock {
            flags.insert(features::UNLOCKED_KEY.to_string(), true);
        } else {
            flags.remove(features::UNLOCKED_KEY);
        }
        write(self, &flags)?;
        Ok(unlock)
    }

    /// The schema ids of the pages the nav shows for this build and these settings.
    pub fn visible_sections(&self) -> Vec<String> {
        let state = self.shared.lock();
        let settings = state.effective();
        SectionId::ALL.into_iter().filter(|id| id.is_visible(settings)).map(section_id).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wormhole_core::storage::paths::{DataRoot, ProfileId};

    use super::super::handle::{SettingsHandle, SettingsListener};

    struct Quiet;
    impl SettingsListener for Quiet {
        fn wake(&self) {}
    }

    /// A flag turned on makes its page appear, and back at the default takes its entry out again.
    #[test]
    fn a_flag_shows_its_page_and_the_default_leaves_no_entry() {
        let dir = tempfile::tempdir().expect("scratch dir");
        let handle = SettingsHandle::open_at(
            &DataRoot::new(dir.path()),
            &ProfileId::default_profile(),
            Arc::new(Quiet),
        );
        let on = |key: &str| handle.feature_flags().into_iter().find(|row| row.key == key).map(|row| row.on);
        let shown = |id: &str| handle.visible_sections().iter().any(|s| s == id);
        assert!(shown("agents") && shown("appearance"));
        let row = handle.feature_flags().into_iter().find(|row| row.key == "probes").expect("probes flag");
        handle.set_feature_flag("probes".into(), !row.default).expect("writes");
        assert_eq!(on("probes"), Some(!row.default));
        assert_eq!(shown("extensions"), !row.default);
        handle.set_feature_flag("probes".into(), row.default).expect("writes");
        assert!(!handle.values().contains("\"probes\""), "the default is not stored");
        assert!(handle.set_feature_flag("nope".into(), true).is_err());
    }
}
