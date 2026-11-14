//! The Settings window's Project folders page over the same service the Project tab uses (one hub, so a trust
//! given here is what the tab shows): the defaults every project starts from, the trusted list, the projects
//! worth offering, and one project's trust and its own Default | On | Off for each feature. The rules are core's
//! (`ProjectPrefs`, `TrustStore`, `FeatureStanding`); the sentences are the GPUI page's
//! (`settings_window/sections/projects.rs`), written once here.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use wormhole_core::platform::paths;
use wormhole_core::project::{
    Feature, FeatureStanding, ProjectFolder, TrustStatus, dir_name_at, find_project, trust_applies,
};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::records::{ProjectFailure, TrustLineRecord, TrustStatusRecord};
use super::service::{ProjectService, open};

/// One feature's switch in the defaults list.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectDefaultRecord {
    pub key: String,
    pub label: String,
    pub description: String,
    pub on: bool,
}

/// A project the page can pick: its folder's name and where it is.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectChoiceRecord {
    pub root: String,
    pub name: String,
    pub detail: String,
}

/// One feature of the picked project: what it says, whether it is held back, the project's own choice and what
/// "Default" is.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectFeatureRowRecord {
    pub key: String,
    pub label: String,
    pub description: String,
    /// No such file in the project (a notes.md can still be switched on: that makes it).
    pub faint: bool,
    /// `None`: follows the default.
    pub choice: Option<bool>,
    pub default_on: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectPageRecord {
    pub root: String,
    pub status: TrustStatusRecord,
    pub status_word: String,
    pub detail: String,
    /// What it would run, while it is not trusted.
    pub commands: Vec<TrustLineRecord>,
    pub features: Vec<ProjectFeatureRowRecord>,
    pub problems: Vec<String>,
    /// Why the last approval or save failed.
    pub error: Option<String>,
    /// An approval is waiting for the owner.
    pub approving: bool,
    /// `.wormhole` or the older `.andromeda`, for "Show .wormhole in Finder".
    pub dir_name: String,
}

fn short(path: &Path) -> String {
    match paths::home() {
        Some(home) if path.starts_with(&home) => {
            format!("~/{}", path.strip_prefix(&home).unwrap_or(path).display())
        }
        _ => path.display().to_string(),
    }
}

/// `root` is handed back to name the project (`project_page`, `revoke`): a key of `names` where it is not text, so a
/// revoke from the page takes back the very project's approval, not one of a folder its name as text happens to name.
fn choice(root: &Path) -> ProjectChoiceRecord {
    ProjectChoiceRecord {
        root: crate::files_panel::names::path_key(root),
        name: root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        detail: short(root),
    }
}

/// What the Trust row says under the badge: trusting applies more than commands, so a project with none still
/// says what waits.
fn trust_detail(status: TrustStatus, has_commands: bool, needing: &[Feature]) -> String {
    let applies = trust_applies(needing);
    match (status, has_commands, needing.is_empty()) {
        (TrustStatus::Trusted, true, _) => {
            text(SETTINGS_PROJECTS_ACTIONS_THE_TERMINAL_LAYOUT_AND_ENV).to_string()
        }
        (TrustStatus::Trusted, false, false) => settings_projects_trust_applies(&applies),
        (TrustStatus::Trusted, false, true) => text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string(),
        (TrustStatus::Changed, true, _) => {
            text(SETTINGS_PROJECTS_REVIEW_THE_COMMANDS_BELOW_AND_TRUST).to_string()
        }
        (TrustStatus::Changed, false, false) => settings_projects_trust_again(&applies),
        (TrustStatus::Changed, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST_NOW).to_string()
        }
        (TrustStatus::Untrusted, true, _) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_CAN_RUN_COMMANDS).to_string()
        }
        (TrustStatus::Untrusted, false, false) => settings_projects_trust_would_apply(&applies),
        (TrustStatus::Untrusted, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string()
        }
    }
}

/// The line under a feature's name: core's words for a file that is missing or sets nothing, what the feature
/// does otherwise, with the wait for trust when something is held back.
fn feature_description(feature: Feature, standing: FeatureStanding) -> String {
    match (feature, standing) {
        (Feature::Notes, FeatureStanding::Missing) => {
            text(SETTINGS_PROJECTS_NO_NOTES_MD_YET_ON_CREATES).to_string()
        }
        (_, FeatureStanding::NeedsTrust) => settings_projects_feature_needs_trust(feature.description()),
        (_, standing) => standing.hint(feature).unwrap_or_else(|| feature.description().to_string()),
    }
}

#[uniffi::export]
impl ProjectService {
    /// A switch for each feature: whether projects use it unless they say otherwise.
    pub fn project_defaults(&self) -> Vec<ProjectDefaultRecord> {
        let hub = self.lock();
        let prefs = hub.prefs();
        Feature::ALL
            .into_iter()
            .map(|feature| ProjectDefaultRecord {
                key: feature.key().to_string(),
                label: feature.label().to_string(),
                description: if feature.requires_trust() {
                    settings_projects_feature_needs_trust(feature.description())
                } else {
                    feature.description().to_string()
                },
                on: prefs.default_enabled(feature),
            })
            .collect()
    }

    pub fn set_project_default(&self, feature: String, on: bool) -> Result<(), ProjectFailure> {
        let feature = Feature::from_key(&feature).ok_or_else(|| ProjectFailure::not_found(feature))?;
        self.lock().change_prefs(|prefs| prefs.set_default(feature, on));
        self.changed();
        Ok(())
    }

    /// Puts this project's choice for `feature` back to following the default.
    pub fn clear_feature_choice(&self, root: String, feature: String) -> Result<(), ProjectFailure> {
        let feature = Feature::from_key(&feature).ok_or_else(|| ProjectFailure::not_found(feature))?;
        let folder = open(&root).ok_or_else(|| ProjectFailure::not_found(root))?;
        self.lock().change_prefs(|prefs| prefs.clear_override(&folder.root, feature));
        self.changed();
        Ok(())
    }

    /// The projects the person trusts, whether or not they are open anywhere.
    /// Read from the file as it is now: a project may have been trusted or revoked elsewhere (`watch_trust`).
    pub fn trusted_projects(&self) -> Vec<ProjectChoiceRecord> {
        let hub = self.lock();
        hub.trust().list().into_iter().map(|trusted| choice(&trusted.root)).collect()
    }

    /// Calls `listener` once whenever the profile's approvals change on disk, until the returned watch is let go: the
    /// Project folders page hears a trust given or taken back in another window or by the command line.
    pub fn watch_trust(
        &self,
        listener: Arc<dyn crate::settings::ProjectListener>,
    ) -> Arc<crate::settings::ProjectWatch> {
        let dir = self.lock().dir().to_path_buf();
        crate::settings::project_watch::watch_approvals(&dir, listener)
    }

    /// The projects worth offering: the project of each of `folders` (the open terminals' and the recent ones),
    /// then the trusted ones, each once and only while it still has its project folder.
    pub fn known_projects(&self, folders: Vec<String>) -> Vec<ProjectChoiceRecord> {
        let mut roots: Vec<PathBuf> = Vec::new();
        let trusted: Vec<PathBuf> =
            self.lock().trust().list().into_iter().map(|trusted| trusted.root.clone()).collect();
        let candidates = folders
            .iter()
            .filter_map(|folder| {
                find_project(&crate::files_panel::names::folder(folder)).map(|found| found.root)
            })
            .chain(trusted);
        for root in candidates {
            if !roots.contains(&root) && ProjectFolder::open(&root).is_some() {
                roots.push(root);
            }
        }
        roots.iter().map(|root| choice(root)).collect()
    }

    /// One project as the page draws it; `None` when its folder has no project anymore.
    pub fn project_page(&self, root: String) -> Option<ProjectPageRecord> {
        let folder = open(&root)?;
        let mut hub = self.lock();
        let snapshot = hub.snapshot(&folder);
        let status = hub.trust().status_of(&snapshot);
        let request = snapshot.trust_request();
        let has_commands = request.as_ref().is_some_and(|r| !r.commands.is_empty());
        let needing = snapshot.needing_trust();
        let prefs = hub.prefs();
        let features = Feature::ALL
            .into_iter()
            .map(|feature| {
                let standing = snapshot.standing(feature, status);
                // Notes: with no notes.md nothing syncs unless the person switches it on, so "Default (on)"
                // would say what does not happen.
                let default_on =
                    prefs.default_enabled(feature) && (feature != Feature::Notes || snapshot.has(feature));
                ProjectFeatureRowRecord {
                    key: feature.key().to_string(),
                    label: feature.label().to_string(),
                    description: feature_description(feature, standing),
                    faint: standing == FeatureStanding::Missing && feature != Feature::Notes,
                    choice: prefs.project_override(&folder.root, feature),
                    default_on,
                }
            })
            .collect();
        let status_word = match status {
            TrustStatus::Trusted => SETTINGS_PROJECTS_TRUSTED,
            TrustStatus::Changed => SETTINGS_PROJECTS_CHANGED_SINCE_YOU_TRUSTED_IT,
            TrustStatus::Untrusted => SETTINGS_PROJECTS_NOT_TRUSTED,
        };
        Some(ProjectPageRecord {
            root: crate::files_panel::names::path_key(&folder.root),
            status: status.into(),
            status_word: text(status_word).to_string(),
            detail: trust_detail(status, has_commands, &needing),
            commands: request
                .filter(|_| status != TrustStatus::Trusted)
                .map(|r| r.commands.iter().map(TrustLineRecord::from).collect())
                .unwrap_or_default(),
            features,
            problems: snapshot.errors().map(|err| err.to_string()).collect(),
            error: hub.error.clone(),
            approving: hub.approving,
            dir_name: dir_name_at(&folder.root),
        })
    }

    /// "<folder> has no .wormhole folder anymore", for a project the page was showing that lost it.
    pub fn missing_folder_note(&self, root: String) -> String {
        settings_projects_missing_folder(&root)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::service::ProjectService;
    use super::super::trust::{OwnerAnswer, OwnerAuthenticator};

    struct Never;
    impl OwnerAuthenticator for Never {
        fn authenticate(&self, _reason: String) -> OwnerAnswer {
            panic!("a test must not raise the authentication");
        }
    }

    /// A project the person picks: its defaults and its own choice come from core's prefs, "Default" follows them,
    /// and a folder that lost its project folder is not offered.
    #[test]
    fn a_project_follows_the_defaults_until_it_has_a_choice_of_its_own() {
        let data = tempfile::tempdir().expect("data");
        let project = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(project.path().join(".wormhole")).expect("project folder");
        let service = ProjectService::in_dir(data.path(), Arc::new(Never));
        let root = std::fs::canonicalize(project.path()).expect("canonical").to_string_lossy().into_owned();
        let known = service.known_projects(vec![root.clone()]);
        assert_eq!(known.iter().map(|c| c.root.as_str()).collect::<Vec<_>>(), [root.as_str()]);

        let row = |feature: &str| {
            service
                .project_page(root.clone())
                .expect("page")
                .features
                .into_iter()
                .find(|f| f.key == feature)
                .expect("row")
        };
        assert_eq!(row("theme").choice, None);
        service.set_project_default("theme".into(), false).expect("default");
        assert!(!row("theme").default_on);
        service.set_feature(root.clone(), "theme".into(), true).expect("own choice");
        assert_eq!(row("theme").choice, Some(true));
        service.clear_feature_choice(root.clone(), "theme".into()).expect("cleared");
        assert_eq!(row("theme").choice, None);

        std::fs::remove_dir_all(project.path().join(".wormhole")).expect("gone");
        assert!(service.project_page(root).is_none());
    }
}
