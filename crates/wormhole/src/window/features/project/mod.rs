//! `.wormhole/` project folders: detection per session, trust (approved with
//! Touch ID), per-feature toggles, project settings / env, actions, prompts,
//! session layouts and two-way `notes.md` sync.
//!
//! Every feature is optional and can be switched off per project, so a
//! committed `.wormhole/` never forces anything on a collaborator.
//! Nothing executable runs until the folder is trusted.
//!
//! Here: `hub` (what is known of the projects, for a profile) and `hubs` (each profile's own, and which is
//! the shown one) and `projects` (keeping it current). `detect`: which project each session
//! is in, and the settings, theme, env and shell that follow. `trust`: approval and the feature
//! switches. `actions`: actions, layouts, prompts and action keys. `notes_sync`: notes.md.
//! `create`: making a project folder. `color`: writing its colour. `panel`: the Project tab.

mod actions;
mod color;
mod create;
mod detect;
mod hub;
mod hubs;
mod notes_sync;
mod panel;
mod projects;
#[cfg(test)]
mod tests;
mod trust;
mod trust_window;

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::terminal::SpawnOptions;
use wormhole_core::terminal::shells::ShellCommand;

pub(in crate::window) use color::set_project_color;
pub(crate) use hub::{LoadedProject, ProjectHub, load_project};
pub(crate) use hubs::{hub, hub_mut, id_allocated, profile_deleted, profile_opened, profile_shown};
pub(crate) use projects::{known_projects, load_project_info, prune_projects};
use projects::{load_projects, projects_changed, reload_projects};
pub(crate) use trust::{approve_project, revoke_project, set_feature_default, set_feature_for};
#[cfg(feature = "devctl")]
pub(crate) use trust_window::devctl_trust_shot;
pub(crate) use trust_window::open_trust_window;

/// Whether the project-folder feature is on (the feature table's mask: its flag, see core `features`). Everything that reads
/// or writes a `.wormhole/` asks this itself instead of trusting that `refresh_projects` has
/// already cleared the window's projects: a settings change doesn't run that refresh, so for up to
/// a tick after the flag goes off the old projects are still there.
pub(crate) fn project_folders_on(cx: &App) -> bool {
    crate::settings::features(cx).has(wormhole_core::config::FeatureId::Project)
}

pub(crate) type BTreeMapEnv = std::collections::BTreeMap<String, String>;

pub(in crate::window) fn spawn_options(
    tab_id: i64,
    cwd: Option<PathBuf>,
    restore: Option<String>,
    env: BTreeMapEnv,
    shell: ShellCommand,
) -> SpawnOptions {
    SpawnOptions {
        cwd,
        tab_id: tab_id.to_string(),
        restore,
        scrollback_lines: 0,
        allow_screen_checksum: false,
        allow_window_ops: false,
        allow_title_report: false,
        env,
        shell: Some(shell),
        welcome: false,
        colors: None,
        color_scheme: None,
    }
}
