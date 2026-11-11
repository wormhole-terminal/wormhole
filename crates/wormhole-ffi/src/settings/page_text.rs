//! Sentences of the settings pages that carry a number or a header: the Updates page's "What an update check
//! sends" (the user agent a check carries, as core builds it, so the example is the header this Mac sends), and
//! the counts of the Agents and Import pages, in the catalogs' plurals.

use super::handle::SettingsHandle;
use semver::Version;
use std::path::Path;
use wormhole_core::config::settings_index::SectionId;
use wormhole_core::platform::paths;
use wormhole_core::terminal::shells::{self, ShellKind};
use wormhole_core::update::user_agent;
use wormhole_core::view::sidebar::display_path;
use wormhole_text::keys::{
    settings_advanced_reset_question, settings_agents_others_head, settings_changed_summary, settings_import_selected,
    settings_terminal_worktree_base_under,
    settings_window_preview_feature_note, settings_window_profiles_unreadable,
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

/// "7 settings changed", the Changed page's summary.
#[uniffi::export]
pub fn settings_changed_summary_line(count: u32) -> String {
    settings_changed_summary(whole(count))
}

/// "The list of profiles couldn't be read, so it was set aside and rebuilt: <why>": the Profile page's banner while
/// `profiles.json` was recovered at launch; `None` when it read.
#[uniffi::export]
pub fn settings_profiles_problem() -> Option<String> {
    crate::profile::recovered().map(|message| settings_window_profiles_unreadable(&message))
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

/// The one header of an update check that says anything: the app's name and `version`, as core sends it. `None`
/// when the version does not parse.
#[uniffi::export]
pub fn settings_update_user_agent(version: String) -> Option<String> {
    Some(user_agent(&Version::parse(&version).ok()?))
}

#[cfg(test)]
mod tests {
    use super::settings_update_user_agent;

    #[test]
    fn a_check_names_the_app_and_its_version() {
        assert_eq!(settings_update_user_agent("1.6.3".into()).as_deref(), Some("Wormhole-Updater/1.6.3"));
        assert_eq!(settings_update_user_agent("nope".into()), None);
    }
}
