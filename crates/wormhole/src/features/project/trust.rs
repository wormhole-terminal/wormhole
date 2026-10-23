//! Trust and the per-project switches: Touch ID approval of exactly what was shown, revoking,
//! and the feature toggles, for the Project tab and Settings.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::project::{Feature, ProjectFolder, TrustRequest};
use wormhole_core::storage::paths::ProfileId;

use super::{ProjectHub, hub_of, hub_of_mut, load_project, projects_changed};
use crate::platform::auth;

impl ProjectHub {
    /// The answer to an approval dialog for `request` (`None`: it went away without one).
    fn finish_approval(&mut self, request: &TrustRequest, answer: Option<Result<(), auth::AuthError>>) {
        self.approving = false;
        match answer {
            Some(Ok(())) => {
                // Exactly what was shown: if the files changed since, the project reads "changed".
                self.trust.trust_request(request);
                tracing::info!(root = %request.root.display(), "project trusted");
            }
            Some(Err(auth::AuthError::Cancelled)) => {}
            Some(Err(err)) => self.error = Some(format!("Not trusted: {err}").into()),
            None => self.error = Some("Not trusted: the approval dialog closed unexpectedly".into()),
        }
    }
}

/// Ask for Touch ID (or the password), then trust the project at `root` for `profile` exactly as it was
/// shown: the hub's snapshot of it, which the Project tab and Settings display. The approval is the
/// profile's own, whichever profile is shown.
pub(crate) fn approve_project(profile: &ProfileId, root: &Path, cx: &mut App) {
    if hub_of(profile, cx).approving {
        return;
    }
    let Some(hub) = hub_of_mut(profile, cx) else { return };
    if !hub.projects.contains_key(root) {
        let Some(folder) = ProjectFolder::open(root) else { return };
        let loaded = load_project(folder, &hub.trust);
        hub.projects.insert(root.to_path_buf(), loaded);
    }
    let Some(project) = hub.projects.get(root) else { return };
    let Some(request) = project.request.clone() else {
        let why = project.errors.first().cloned().unwrap_or_default();
        hub.error = Some(format!("Can't trust this folder until it loads: {why}").into());
        cx.refresh_windows();
        return;
    };
    approve_request(profile, request, cx);
}

/// Ask for Touch ID (or the password), then trust exactly what `request` describes, for `profile`.
pub(crate) fn approve_request(profile: &ProfileId, request: TrustRequest, cx: &mut App) {
    if hub_of(profile, cx).approving {
        return;
    }
    let root = &request.root;
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let reason = format!("trust the project folder \u{201c}{name}\u{201d} and let it run commands");
    let Some(hub) = hub_of_mut(profile, cx) else { return };
    hub.approving = true;
    hub.error = None;
    cx.refresh_windows();

    let rx = auth::authenticate_rx(&reason);
    let profile = profile.clone();
    cx.spawn(async move |cx| {
        let result = rx.recv().await;
        cx.update(|cx| {
            if let Some(hub) = hub_of_mut(&profile, cx) {
                hub.finish_approval(&request, result.ok());
            }
            projects_changed(&profile, cx);
        });
    })
    .detach();
}

pub(crate) fn revoke_project(profile: &ProfileId, root: &Path, cx: &mut App) {
    let Some(hub) = hub_of_mut(profile, cx) else { return };
    hub.trust.revoke(root);
    projects_changed(profile, cx);
}

/// `None` goes back to the default for all projects.
pub(crate) fn set_feature_for(
    profile: &ProfileId,
    root: &Path,
    feature: Feature,
    enabled: Option<bool>,
    cx: &mut App,
) {
    let Some(hub) = hub_of_mut(profile, cx) else { return };
    match enabled {
        Some(enabled) => hub.prefs.set_enabled(root, feature, enabled),
        None => hub.prefs.clear_override(root, feature),
    }
    projects_changed(profile, cx);
}

pub(crate) fn set_feature_default(profile: &ProfileId, feature: Feature, enabled: bool, cx: &mut App) {
    let Some(hub) = hub_of_mut(profile, cx) else { return };
    hub.prefs.set_default(feature, enabled);
    projects_changed(profile, cx);
}

#[cfg(test)]
mod tests {
    use crate::features::project::ProjectHub;
    use crate::platform::auth::AuthError;
    use wormhole_core::project::{ProjectFolder, TrustStatus};

    /// A fresh directory under the system temp dir, removed when dropped.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let path = std::env::temp_dir().join(format!("hd-{name}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("mkdir");
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// What each answer of the approval dialog does (no system dialog in tests).
    #[test]
    fn approval_answers() {
        let data = Scratch::new("approval-data");
        let repo = Scratch::new("approval-repo");
        let dir = repo.path().join(".wormhole");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("actions.json"),
            r#"[{"id": "t", "title": "T", "type": "run", "command": "make"}]"#,
        )
        .expect("write");
        let folder = ProjectFolder::open(repo.path()).expect("folder");
        let snapshot = folder.snapshot();
        let request = snapshot.trust_request().expect("request");
        let mut hub = ProjectHub::load_from(data.path());
        assert!(hub.error.is_none());

        for (answer, error) in [
            (Some(Err(AuthError::Cancelled)), false),
            (Some(Err(AuthError::Failed("wrong finger".into()))), true),
            (None, true),
        ] {
            hub.approving = true;
            hub.error = None;
            hub.finish_approval(&request, answer);
            assert!(!hub.approving);
            assert_eq!(hub.error.is_some(), error);
            assert_eq!(hub.trust.status_of(&snapshot), TrustStatus::Untrusted);
        }
        hub.finish_approval(&request, Some(Ok(())));
        assert_eq!(hub.trust.status_of(&snapshot), TrustStatus::Trusted);
    }
}
