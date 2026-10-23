//! Which project each terminal is in, keeping the hub's copy of those projects current, and the
//! project settings, theme, environment and shell that follow from it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::project::{self as proj, Feature};
use wormhole_core::storage::paths::ProfileId;

use super::{Project, hub_of, hub_of_mut, load_project, load_projects, project_folders_on, shown_profile};
use crate::settings::SettingsStore;
use crate::window::facts::WindowFacts;
use crate::window::feature::{FeatureEvent, SpawnAdd};

impl Project {
    /// Find each terminal's project and bring the ones in use up to date. Redraws only when something
    /// changed (this runs every 2 s for every window).
    pub(super) fn refresh(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        // The whole project-folder feature is behind a flag: while it is off, nothing reads a
        // `.wormhole/`. Drop any project each terminal was in and any overlay a project applied, so
        // bands untint, the Project tab goes away and no settings or theme follow from one.
        if !project_folders_on(facts.profile, cx) {
            let mut moved = !self.folder_projects.is_empty();
            self.folder_projects.clear();
            // The Project tab is gone with the feature; leave it showing nothing.
            if facts.tab == TabId::Project {
                cx.emit(FeatureEvent::TabGone(TabId::Project));
                moved = true;
            }
            for of in self.of.values_mut() {
                moved |= of.root.is_some();
                of.root = None;
            }
            // Unwind any settings or theme a project had layered on, from whichever window sees the flag
            // go off, not only the front one. Toggling it in the Settings window leaves no workspace
            // front, so a front-only unwind would keep a project's overlay applied. The overlay is
            // app-global and the active project is none with the flag off, so this clears it.
            self.apply_project_settings(facts, cx);
            if moved {
                cx.notify();
            }
            return;
        }
        let mut roots = Vec::new();
        let mut moved = false;
        let mut folders = Vec::new();
        for session in facts.sessions() {
            let root = session.folder(cx).and_then(proj::find_project).map(|folder| folder.root);
            let of = self.of.entry(session.id).or_default();
            moved |= of.root != root;
            of.root = root;
            roots.extend(of.root.clone());
            folders.extend(session.band_folder(cx));
        }
        // The project that governs each folder a band names holds that folder's colour; it may be above
        // the terminal's own (a terminal in a nested project) or none.
        let mut folder_projects = HashMap::new();
        for folder in folders {
            if !folder_projects.contains_key(&folder)
                && let Some(project) = proj::find_project(&folder)
            {
                folder_projects.insert(folder, project.root);
            }
        }
        roots.extend(folder_projects.values().cloned());
        moved |= self.folder_projects != folder_projects;
        self.folder_projects = folder_projects;
        if load_projects(facts.profile, roots, cx) {
            // Other windows (and Settings) may show the project that changed.
            cx.refresh_windows();
        }
        // Settings and theme come from the project of the window in front.
        if facts.is_front(cx) {
            self.apply_project_settings(facts, cx);
        }
        if moved {
            cx.notify();
        }
    }

    /// Layer the active terminal's project settings over the user's. They go over the shown profile's
    /// settings, so only a window of the shown profile puts any on: a project another profile trusts
    /// does not change the look and behaviour of this one.
    fn apply_project_settings(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        if facts.profile != shown_profile(cx) {
            return;
        }
        let active = facts.active().map(|session| session.id);
        let hub = hub_of(facts.profile, cx);
        let overrides = active
            .and_then(|id| self.project(id, facts.profile, cx))
            .filter(|p| hub.enabled(p, Feature::Settings))
            .and_then(|p| p.snapshot.config())
            .map(|c| c.settings.clone())
            .filter(|s| !s.is_empty());
        if overrides != hub.applied_overrides {
            SettingsStore::set_project_overrides(cx, overrides.clone());
            if let Some(hub) = hub_of_mut(facts.profile, cx) {
                hub.applied_overrides = overrides;
            }
            cx.refresh_windows();
        }

        // Project theme: harmless, so it needs no trust, but can be switched off.
        let hub = hub_of(facts.profile, cx);
        let theme_json = active
            .and_then(|id| self.project(id, facts.profile, cx))
            .filter(|p| hub.enabled(p, Feature::Theme))
            .and_then(|p| p.snapshot.theme_json())
            .map(|value| value.to_string());
        if theme_json != hub.applied_theme {
            let tokens = theme_json.as_deref().and_then(|json| {
                wormhole_core::config::tokens_from_json_in(&crate::active_profile::paths().themes_dir(), json)
                    .inspect_err(|err| tracing::warn!("project theme: {err}"))
                    .ok()
            });
            SettingsStore::set_project_theme(cx, tokens);
            if let Some(hub) = hub_of_mut(facts.profile, cx) {
                hub.applied_theme = theme_json;
            }
            cx.refresh_windows();
        }
    }
}

/// Env and shell for a new terminal of a window of `profile` started in `cwd`, if its project allows it
/// under that profile's approvals.
pub(super) fn spawn_options(profile: &ProfileId, cwd: Option<&Path>, cx: &mut App) -> SpawnAdd {
    if !project_folders_on(profile, cx) {
        return Default::default();
    }
    let Some(folder) = cwd.and_then(proj::find_project) else {
        return Default::default();
    };
    // Terminals restored at startup spawn before the first refresh; load on demand.
    let root: PathBuf = folder.root.clone();
    let Some(hub) = hub_of_mut(profile, cx) else { return Default::default() };
    if !hub.projects.contains_key(&root) {
        let loaded = load_project(folder, &hub.trust);
        hub.projects.insert(root.clone(), loaded);
    }
    let Some(project) = hub.projects.get(&root) else {
        return Default::default();
    };
    if !hub.enabled(project, Feature::Settings) {
        return Default::default();
    }
    let config = project.snapshot.config().cloned().unwrap_or_default();
    SpawnAdd { env: config.env, shell: config.shell }
}

impl Project {
    /// Once per tick, from the window's feature loop.
    pub(super) fn tick(&mut self, facts: &WindowFacts<'_>, window: &mut Window, cx: &mut Context<Self>) {
        if facts.ticks % 5 == 1 {
            self.refresh(facts, cx);
        }
        if facts.ticks.is_multiple_of(3) {
            self.sync_notes(facts, window, cx);
        }
        if facts.is_front(cx)
            && let Some(root) = facts.active().and_then(|session| self.of.get(&session.id)?.root.clone())
        {
            crate::app_state::app_mut(cx).remember_project(root);
        }
    }
}
