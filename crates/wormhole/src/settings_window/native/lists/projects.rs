//! Project folders: the projects found on this Mac with their trust, and the choices that apply to them. "For"
//! picks whose choices the rows below are: every project's defaults, or one project's own (Default, On or Off per
//! feature, written to the profile's preferences, never into the project folder).
//! Trust is the project feature's own (`approve_project` asks for Touch ID, `revoke_project` takes it back); the
//! window hears of a change from the hubs' own notification, so an approval that finishes later shows by itself.

use std::path::{Path, PathBuf};

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::project::{Feature, FeatureStanding};
use wormhole_text::keys::{
    SETTINGS_PROJECTS_ALL_PROJECTS_DEFAULTS, SETTINGS_PROJECTS_DEFAULT_OFF, SETTINGS_PROJECTS_DEFAULT_ON,
    SETTINGS_PROJECTS_FOR, SETTINGS_PROJECTS_FOR_NOTE, SETTINGS_PROJECTS_OFF, SETTINGS_PROJECTS_ON,
    settings_projects_missing_folder,
};
use wormhole_text::text;

use super::Entry;
use crate::features::project::{self, LoadedProject};

const GROUP_PROJECTS: &str = "Projects:0";
const GROUP_DEFAULTS: &str = "Projects:1";

/// The id of the "For" row (no feature is called that).
const FOR: &str = "for";

/// The most projects listed; the picker of the GPUI page is a search, a list here is not.
const MOST: usize = 40;

/// The project "For" shows, with its files as read when it was chosen (reading them is file I/O, so it is kept until
/// the hubs change).
#[derive(Default)]
pub(crate) struct Selected {
    root: Option<PathBuf>,
    info: Option<LoadedProject>,
}

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

/// A project's own choice for a feature, as the id the window holds.
fn choice_id(choice: Option<bool>) -> &'static str {
    match choice {
        None => "default",
        Some(true) => "on",
        Some(false) => "off",
    }
}

fn choice_of(id: &str) -> Option<Option<bool>> {
    match id {
        "default" => Some(None),
        "on" => Some(Some(true)),
        "off" => Some(Some(false)),
        _ => None,
    }
}

/// Read the chosen project's files again (after choosing, and whenever the hubs say something changed).
pub(super) fn refresh(cx: &mut App) {
    if !project::present(cx) {
        return;
    }
    let Some(root) = super::cache(cx).and_then(|cache| cache.selected.root.clone()) else { return };
    let info = project::load_project_info(project::shown_profile(cx), &root, cx);
    super::cache_mut(cx).selected.info = info;
}

fn for_entry(cx: &App) -> Value {
    let selected = super::cache(cx).and_then(|cache| cache.selected.root.clone());
    let trusted = project::api::trusted_roots(cx);
    let known: Vec<PathBuf> = super::cache(cx).map(|cache| cache.projects.clone()).unwrap_or_default();
    let mut roots: Vec<PathBuf> = trusted;
    for root in known.into_iter().chain(selected.clone()) {
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    let mut choices = vec![json!(["", text(SETTINGS_PROJECTS_ALL_PROJECTS_DEFAULTS)])];
    choices.extend(roots.iter().take(MOST + 1).map(|root| json!([root.to_string_lossy(), name_of(root)])));
    Entry::new(FOR, "popup", text(SETTINGS_PROJECTS_FOR))
        .with("hint", text(SETTINGS_PROJECTS_FOR_NOTE))
        .with("choices", choices)
        .with("value", selected.map(|root| root.to_string_lossy().into_owned()).unwrap_or_default())
        .into_value()
}

fn feature_entries(root: &Path, cx: &App) -> Vec<Value> {
    let Some(info) = super::cache(cx).and_then(|cache| cache.selected.info.as_ref()) else {
        // A folder that is no project (or is gone): say so, as the GPUI page does.
        return vec![
            Entry::new("missing", "value", &settings_projects_missing_folder(&root.display().to_string()))
                .into_value(),
        ];
    };
    project::api::feature_rows(root, info, cx)
        .into_iter()
        .map(|(feature, standing, current, default)| {
            let words = |on_default: bool| {
                text(if on_default { SETTINGS_PROJECTS_DEFAULT_ON } else { SETTINGS_PROJECTS_DEFAULT_OFF })
            };
            Entry::new(&format!("feature-{}", feature.key()), "popup", feature.label())
                .with("hint", crate::settings_window::sections::feature_description(feature, standing))
                .with(
                    "choices",
                    json!([
                        ["default", words(default)],
                        ["on", text(SETTINGS_PROJECTS_ON)],
                        ["off", text(SETTINGS_PROJECTS_OFF)],
                    ]),
                )
                .with("value", choice_id(current))
                // Notes without a file are still for the person to switch on: that makes the file.
                .with("dim", standing == FeatureStanding::Missing && feature != Feature::Notes)
                .into_value()
        })
        .collect()
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
        defaults.push(for_entry(cx));
        match super::cache(cx).and_then(|cache| cache.selected.root.clone()) {
            Some(root) => defaults.extend(feature_entries(&root, cx)),
            None => {
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
        }
    }
    out.insert(GROUP_PROJECTS.into(), Value::Array(projects));
    out.insert(GROUP_DEFAULTS.into(), Value::Array(defaults));
}

pub(super) fn load(cx: &mut App) {
    // Deferred: Settings can be opened from inside a window's update, and the list reads the windows' terminals.
    cx.defer(|cx| {
        if project::present(cx) && cx.has_global::<crate::settings_window::native::Native>() {
            let known = project::known_projects(cx);
            super::cache_mut(cx).projects = known;
            refresh(cx);
            crate::settings_window::native::push(cx);
        }
    });
}

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
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
            if action == "revoke" {
                project::revoke_project(&profile, &root, cx);
            } else {
                project::approve_project(&profile, &root, cx);
            }
        }
        "1" if id == FOR => {
            let picked = action.strip_prefix('=').filter(|root| !root.is_empty()).map(PathBuf::from);
            super::cache_mut(cx).selected = Selected { root: picked, info: None };
            refresh(cx);
        }
        "1" => {
            let selected = super::cache(cx).and_then(|cache| cache.selected.root.clone());
            if let Some(root) = selected {
                let feature = id.strip_prefix("feature-").and_then(Feature::from_key);
                let choice = action.strip_prefix('=').and_then(choice_of);
                if let (Some(feature), Some(choice)) = (feature, choice) {
                    project::set_feature_for(&profile, &root, feature, choice, cx);
                    refresh(cx);
                }
            } else if let Some(feature) = Feature::from_key(id) {
                project::set_feature_default(&profile, feature, action == "on", cx);
            }
        }
        _ => return false,
    }
    true
}
