//! Sentences of the settings pages that carry a number or a header: the Updates page's "What an update check
//! sends" (the user agent a check carries, as core builds it: `counted_user_agent` while the count is on,
//! `plain_user_agent` while it is off, so the example is the header this Mac would send), and the counts of the
//! Agents and Import pages, in the catalogs' plurals.

use super::handle::SettingsHandle;
use semver::Version;
use std::path::Path;
use std::time::SystemTime;
use wormhole_core::config::settings_index::SectionId;
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::terminal::shells::{self, ShellKind};
use wormhole_core::update::{
    UpdateChannel, UpdateSchedule, counted_user_agent, current_platform, macos_version, plain_user_agent,
};
use wormhole_core::view::sidebar::display_path;
use wormhole_text::keys::{
    settings_advanced_reset_question, settings_agents_others_head, settings_import_selected,
    settings_terminal_worktree_base_under, settings_updates_count_starts_in,
    settings_window_preview_feature_note,
};
use wormhole_text::whole;

/// "17 more agents Wormhole recognises".
#[uniffi::export]
pub fn settings_others_head(count: u32) -> String {
    settings_agents_others_head(whole(count))
}

/// "Reset 3 changed settings to their default? Your terminals, notes and themes are kept."
#[uniffi::export]
pub fn settings_reset_question(count: u32) -> String {
    settings_advanced_reset_question(whole(count))
}

/// What a preview section says at its top: it is behind a feature flag, which it names. `section` is the schema's id;
/// `None` for a section that always ships.
#[uniffi::export]
pub fn settings_preview_note(section: String) -> Option<String> {
    let id = SectionId::ALL.into_iter().find(|id| crate::settings_dump::section_id(*id) == section)?;
    id.feature_flag().map(|flag| settings_window_preview_feature_note(flag.key()))
}

/// A shell profile of config.json, for the Shells row's list.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsShellLine {
    pub name: String,
    pub command_line: String,
}

#[uniffi::export]
impl SettingsHandle {
    /// The shell profiles under the Shells row: name and what each runs.
    pub fn shell_profiles(&self) -> Vec<SettingsShellLine> {
        shells::choices(self.shared.lock().effective())
            .into_iter()
            .filter(|choice| choice.kind == ShellKind::Profile)
            .map(|choice| SettingsShellLine { name: choice.name, command_line: choice.command_line })
            .collect()
    }

    /// "Worktrees go under ~/work" once `worktree_base` names a folder; `None` until then.
    pub fn worktree_caption(&self, base: String) -> Option<String> {
        let base = base.trim();
        if base.is_empty() {
            return None;
        }
        let shown = display_path(Path::new(base), paths::home().as_deref());
        Some(settings_terminal_worktree_base_under(&shown))
    }
}

/// "3 of 5 selected".
#[uniffi::export]
pub fn settings_import_selected_line(selected: u32, total: u32) -> String {
    settings_import_selected(&selected.to_string(), &total.to_string())
}

/// The header of an update check. `version` is the running app's, `unstable` the channel; `None` when the
/// version does not parse.
#[uniffi::export]
pub fn settings_update_user_agent(version: String, counting: bool, unstable: bool) -> Option<String> {
    let version = Version::parse(&version).ok()?;
    Some(if counting {
        let channel = if unstable { UpdateChannel::Unstable } else { UpdateChannel::Stable };
        counted_user_agent(&version, channel, &current_platform(), macos_version().as_ref())
    } else {
        plain_user_agent(&version)
    })
}

/// "Sharing starts in about 24 hours, with your next update check." while this copy waits to be counted (the day after
/// a version that counts first ran: `UpdateSchedule::count_starts_in`), `None` once it is counted or before the wait began.
#[uniffi::export]
pub fn settings_update_count_wait() -> Option<String> {
    crate::isolation::require().ok()?;
    let file = DataRoot::current().update_schedule_file();
    let left = UpdateSchedule::load_shared(&file, || None).count_starts_in(SystemTime::now())?;
    Some(settings_updates_count_starts_in(whole(left.as_secs().div_ceil(3600))))
}

#[cfg(test)]
mod tests {
    use super::settings_update_user_agent;

    #[test]
    fn a_counted_check_says_more_than_a_plain_one() {
        let plain = settings_update_user_agent("1.6.3".into(), false, false).expect("parsed");
        assert_eq!(plain, "Wormhole-Updater/1.6.3");
        let counted = settings_update_user_agent("1.6.3".into(), true, true).expect("parsed");
        assert!(counted.starts_with("Wormhole-Updater/1.6.3") && counted.len() > plain.len(), "{counted}");
        assert_eq!(settings_update_user_agent("nope".into(), true, false), None);
    }
}
