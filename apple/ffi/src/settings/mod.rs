//! The Settings window's backend: one profile's settings (`handle`, over `state` and core's `Config`),
//! the file watch behind its one wake (`watch`), search (`search`), shortcut checks (`shortcuts`), and
//! what the host pages list: select options and folders (`host`), the agents and their hooks
//! (`agents`), terminals to import from (`imports`) and the profiles (`profiles`). The schema the
//! rows are drawn from is `settings_dump`; the rules in all of it are `wormhole-core`'s.

mod agents;
mod error;
mod flags;
mod handle;
mod host;
mod import_review;
mod imports;
mod page_text;
mod probes;
mod profile_edit;
mod profiles;
mod search;
mod shortcuts;
mod skills;
mod state;

mod values;
mod watch;

#[cfg(test)]
pub(crate) mod tests;

pub use agents::{SettingsAgentRow, SettingsHookState, settings_agent_rows, settings_set_agent_hooks};
pub use error::SettingsError;
pub use flags::SettingsFlagRow;
pub use handle::{SettingsChangedRow, SettingsHandle, SettingsHealth, SettingsListener};
pub use host::{SettingsOptionRow, SettingsPaths};
pub use import_review::{
    FontFamilyResolver, SettingsImportDone, SettingsImportItem, SettingsImportReview, SettingsImportSkip,
    settings_import_review,
};
pub use imports::{SettingsImportRow, settings_import_rows};
pub use page_text::{
    SettingsShellLine, settings_import_selected_line, settings_others_head, settings_preview_note,
    settings_reset_question, settings_update_count_wait, settings_update_user_agent,
};
pub use probes::{
    ProbeApprovalOutcome, ProbeChangeRecord, ProbeListRecord, ProbePermissionRecord, ProbeRowRecord,
    ProbeSheetRecord, ProbesDesk,
};
pub use profile_edit::{
    SettingsLoginField, SettingsProfileForm, settings_profile_form, settings_profile_save,
};
pub use profiles::{
    SettingsProfileRow, settings_create_profile, settings_delete_profile, settings_profiles,
    settings_switch_profile,
};
pub use search::{SettingsHit, SettingsTextSpan};
pub use shortcuts::{ShortcutVerdict, check_shortcut};
pub use skills::{
    SettingsSkillRow, SettingsSkillState, settings_mcp_command, settings_set_skill, settings_skill_rows,
};

/// The settings schema (sections, groups, rows, control kinds and their words) as JSON: what the
/// window is drawn from. It changes with the language and the build's flags, so a window asks again
/// when either does.
#[uniffi::export]
pub fn settings_schema_json() -> String {
    crate::settings_dump::json()
}
