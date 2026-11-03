//! Project folders: the projects found on this Mac with their trust, and the defaults every project starts from.
//! Trust is the project feature's own (`approve_project` asks for Touch ID, `revoke_project` takes it back).

use std::path::{Path, PathBuf};

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::project::Feature;
use wormhole_text::keys::*;
use wormhole_text::text;

use super::Entry;
use crate::features::project;

const GROUP_PROJECTS: &str = "Projects:0";
const GROUP_DEFAULTS: &str = "Projects:1";

/// The most projects listed; the picker of the GPUI page is a search, a list here is not.
const MOST: usize = 40;

fn name_of(root: &Path) -> String {
    root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

fn shown(root: &Path) -> String {
    match crate::settings_window::home() {
        Some(home) if root.starts_with(&home) => {
            format!("~/{}", root.strip_prefix(&home).unwrap_or(root).display())
        }
        _ => root.display().to_string(),
    }
}

pub(super) fn lists(out: &mut Map<String, Value>, cx: &mut App) {
    let (mut projects, mut defaults) = (Vec::new(), Vec::new());
    if project::present(cx) {
        let trusted = project::api::trusted_roots(cx);
        let known: Vec<PathBuf> = super::cache(cx).map(|cache| cache.projects.clone()).unwrap_or_default();
        let others = known.into_iter().filter(|root| !trusted.contains(root));
        for (root, value) in
            trusted.iter().cloned().map(|r| (r, "trusted")).chain(others.map(|r| (r, "untrusted"))).take(MOST)
        {
            projects.push(
                Entry::new(&root.to_string_lossy(), "project", &name_of(&root))
                    .with("hint", shown(&root))
                    .with("value", value)
                    .into_value(),
            );
        }
        for feature in Feature::ALL {
            let on = project::api::default_enabled(feature, cx);
            defaults.push(
                Entry::new(feature.key(), "toggle", feature.label())
                    .with("hint", feature.description())
                    .with("value", if on { "on" } else { "off" })
                    .into_value(),
            );
        }
    }
    out.insert(GROUP_PROJECTS.into(), Value::Array(projects));
    out.insert(GROUP_DEFAULTS.into(), Value::Array(defaults));
}

pub(super) fn load(cx: &mut App) {
    if project::present(cx) {
        let known = project::known_projects(cx);
        super::cache_mut(cx).projects = known;
    }
}

pub(super) fn perform(key: &str, name: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some("Projects"), Some(group), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if !project::present(cx) {
        return true;
    }
    let profile = project::shown_profile(cx).clone();
    match group {
        "0" => {
            let root = PathBuf::from(id);
            if name == text(SETTINGS_PROJECTS_REVOKE) {
                project::revoke_project(&profile, &root, cx);
            } else {
                project::approve_project(&profile, &root, cx);
            }
        }
        "1" => {
            if let Some(feature) = Feature::from_key(id) {
                project::set_feature_default(&profile, feature, name == "on", cx);
            }
        }
        _ => return false,
    }
    true
}
