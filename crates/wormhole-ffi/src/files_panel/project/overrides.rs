//! What the project of the terminal in front sets over the person's settings (`config.json`'s `settings`),
//! for Settings to layer (`SettingsHandle::set_project_overrides`) and to mark the rows it sets. The app's
//! rule (`features/project/detect.rs`): only while the project's Settings switch is on and trust allows it
//! (core's `enabled`, the one rule every project action passes), and only when it sets something.

use wormhole_core::project::{self as proj, Feature};
use wormhole_text::keys::settings_window_set_by_project;

use super::service::ProjectService;

/// The project that governs a folder and what its `config.json` sets.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectSettingsRecord {
    /// The project's folder name on disk, for "Set by <name>/<dir>/config.json".
    pub name: String,
    /// `.wormhole` (or an older project's `.andromeda`).
    pub dir: String,
    pub root: String,
    /// What the rows it sets say under their label: "Set by <name>/<dir>/config.json while one of its terminals
    /// is in front; yours is kept", in the catalogs' words.
    pub note: String,
    /// The settings as a JSON object, to hand to `set_project_overrides` (which drops what a project may
    /// not change). `None` when it sets nothing or may not: Settings shows the person's own.
    pub json: Option<String>,
}

#[uniffi::export]
impl ProjectService {
    /// The project of `folder` (a terminal's) and its settings; `None` where there is no project.
    pub fn settings_overrides(&self, folder: String) -> Option<ProjectSettingsRecord> {
        if self.off() {
            return None;
        }
        let found = proj::find_project(&crate::files_panel::names::folder(&folder))?;
        let mut hub = self.lock();
        let snapshot = hub.snapshot(&found);
        let settings = hub
            .enabled(&snapshot, Feature::Settings)
            .then(|| snapshot.config().map(|config| config.settings.clone()))
            .flatten()
            .filter(|settings| !settings.is_empty());
        let name = found.root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = proj::dir_name_at(&found.root);
        Some(ProjectSettingsRecord {
            note: settings_window_set_by_project(&name, &dir, "config.json"),
            name,
            dir,
            root: found.root.to_string_lossy().into_owned(),
            json: settings.and_then(|map| serde_json::to_string(&map).ok()),
        })
    }
}
