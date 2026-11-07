//! The Settings window's backend: one profile's settings (`handle`, over `state` and core's `Config`),
//! the file watch behind its one wake (`watch`), search (`search`), shortcut checks (`shortcuts`), and
//! what the host pages list: select options and folders (`host`), the agents and their hooks
//! (`agents`), terminals to import from (`imports`) and the profiles (`profiles`). The schema the
//! rows are drawn from is `settings_dump`; the rules in all of it are `wormhole-core`'s.

mod agents;
mod error;
mod handle;
mod host;
mod imports;
mod profiles;
mod search;
mod shortcuts;
mod state;
mod values;
mod watch;

#[cfg(test)]
mod tests;

pub use agents::{SettingsAgentRow, SettingsHookState, settings_agent_rows, settings_set_agent_hooks};
pub use error::SettingsError;
pub use handle::{SettingsChangedRow, SettingsHandle, SettingsHealth, SettingsListener};
pub use host::{SettingsOptionRow, SettingsPaths};
pub use imports::{SettingsImportRow, settings_import_rows};
pub use profiles::{
    SettingsProfileRow, settings_create_profile, settings_delete_profile, settings_profiles,
    settings_switch_profile,
};
pub use search::{SettingsHit, SettingsTextSpan};
pub use shortcuts::{ShortcutVerdict, check_shortcut};

/// The settings schema (sections, groups, rows, control kinds and their words) as JSON: what the
/// window is drawn from. It changes with the language and the build's flags, so a window asks again
/// when either does.
#[uniffi::export]
pub fn settings_schema_json() -> String {
    crate::settings_dump::json()
}
