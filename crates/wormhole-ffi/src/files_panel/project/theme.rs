//! A project's `.wormhole/theme.json`: the theme text that colours its windows while one of its terminals is in
//! front (the GPUI app's `detect.rs` and the Studio's "Use in this project"), under the same gate every project
//! action passes (`Hub::enabled`: the file is there, the person has it on, and trust where the feature needs it).

use std::path::Path;

use wormhole_core::project::{self as proj, Feature, TrustStatus};

use super::records::ProjectFailure;
use super::service::ProjectService;

#[uniffi::export]
impl ProjectService {
    /// The text of the theme of the project that governs `folder`, when its theme is in effect (the file is
    /// there and the person has it on; a theme is harmless, so core asks no trust); `None` otherwise. Hand it to
    /// `resolve_theme_json`.
    pub fn theme_overlay(&self, folder: String) -> Option<String> {
        if self.off() {
            return None;
        }
        let found = proj::find_project(Path::new(&folder))?;
        let mut hub = self.lock();
        hub.reload_trust();
        let snapshot = hub.snapshot(&found);
        if !hub.enabled(&snapshot, Feature::Theme) {
            return None;
        }
        snapshot.theme_json().map(ToString::to_string)
    }

    /// Whether "Use in this project" may write to the project that governs `folder`, and the folder's root:
    /// the project is trusted and its theme is not switched off. The reason otherwise, in the person's words.
    pub fn theme_target(&self, folder: String) -> Result<String, ProjectFailure> {
        let found = proj::find_project(Path::new(&folder))
            .ok_or_else(|| ProjectFailure::not_found(folder.clone()))?;
        let mut hub = self.lock();
        hub.reload_trust();
        let snapshot = hub.snapshot(&found);
        if hub.trust.status_of(&snapshot) != TrustStatus::Trusted {
            return Err(ProjectFailure::refused(
                "Trust this project first (Project tab), then use the theme in it.",
            ));
        }
        if !hub.prefs.enabled(&found.root, Feature::Theme) {
            return Err(ProjectFailure::refused("This project's theme is switched off (Project tab)."));
        }
        Ok(found.root.to_string_lossy().into_owned())
    }

    /// Writes `json` (a theme as Export writes it) as the `theme.json` of the project that governs `folder`,
    /// under the same gate as `theme_target`; the path it went to.
    pub fn write_theme(&self, folder: String, json: String) -> Result<String, ProjectFailure> {
        let root = self.theme_target(folder)?;
        let project = super::service::open(&root).ok_or_else(|| ProjectFailure::not_found(root))?;
        let path = project.write_theme(&json).map_err(ProjectFailure::failed)?;
        self.changed();
        Ok(path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::files_panel::project::trust::{OwnerAnswer, OwnerAuthenticator};

    struct Never;
    impl OwnerAuthenticator for Never {
        fn authenticate(&self, _reason: String) -> OwnerAnswer {
            panic!("a test must not raise the authentication");
        }
    }

    /// A project's theme colours its windows only while its theme is on, and may be written only into a project
    /// that is trusted: an untrusted folder's `theme.json` is never applied and never replaced.
    #[test]
    fn a_projects_theme_needs_its_switch_to_apply_and_its_trust_to_be_written() {
        let data = tempfile::tempdir().expect("data");
        let project = tempfile::tempdir().expect("project");
        let dot = project.path().join(".wormhole");
        std::fs::create_dir_all(&dot).expect("project folder");
        std::fs::write(dot.join("theme.json"), r##"{"name": "App", "extends": "Nord"}"##).expect("theme");
        let service = ProjectService::in_dir(data.path(), Arc::new(Never));
        let folder = project.path().to_string_lossy().into_owned();
        let root = std::fs::canonicalize(project.path()).expect("canonical").to_string_lossy().into_owned();

        // Not trusted: the theme is in effect all the same (the switch is on by default), but nothing may be written.
        assert_eq!(service.theme_overlay(folder.clone()).as_deref(), Some(r#"{"name":"App","extends":"Nord"}"#));
        assert!(matches!(service.theme_target(folder.clone()), Err(ProjectFailure::Refused { .. })));
        assert!(service.write_theme(folder.clone(), "{}".into()).is_err());
        assert_eq!(
            std::fs::read_to_string(dot.join("theme.json")).expect("read"),
            r##"{"name": "App", "extends": "Nord"}"##,
            "an untrusted project's file is left as it is"
        );
        service.set_feature(root, "theme".into(), false).expect("off");
        assert_eq!(service.theme_overlay(folder), None);
    }
}
