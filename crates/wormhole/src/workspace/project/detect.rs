//! Which project each session is in, keeping the hub's copy of those projects current, and the
//! project settings, theme, env and shell that follow from it.

use std::path::Path;

use wormhole_core::project::{self as proj, Feature};

use super::*;

impl Workspace {
    /// Find each session's project and bring the ones in use up to date. Redraws only when
    /// something changed (this runs every 2 s for every window).
    pub(in crate::workspace) fn refresh_projects(&mut self, cx: &mut Context<Self>) {
        // The whole project-folder feature is behind a flag: while it is off, nothing reads a
        // `.wormhole/`. Drop any project each session was in and any overlay a project applied,
        // so bands untint, the Project tab goes away and no settings or theme follow from one.
        if !project_folders_on(cx) {
            let mut moved = !self.folder_projects.is_empty();
            self.folder_projects.clear();
            // The Project tab is gone with the feature; leave it showing nothing.
            if self.right_tab == RightTab::Project {
                self.right_tab = RightTab::Changes;
                moved = true;
            }
            for session in &mut self.sessions {
                moved |= session.project.is_some() || session.notes_root.is_some();
                session.project = None;
                session.notes_root = None;
            }
            // Unwind any settings or theme a project had layered on, from whichever window sees the
            // flag go off — not only the front one. Toggling it in the Settings window leaves no
            // workspace front, so a front-only unwind would keep a project's overlay applied. The
            // overlay is app-global and `active_project` is None with the flag off, so this clears it.
            self.apply_project_settings(cx);
            if moved {
                cx.notify();
            }
            return;
        }
        let mut roots = Vec::new();
        let mut moved = false;
        let root_of = |terminal: &Entity<TerminalView>| {
            terminal.read(cx).cwd.as_deref().and_then(proj::find_project).map(|folder| folder.root)
        };
        for session in &mut self.sessions {
            let project = root_of(session.terminal());
            // The project its notes sync with is the one its terminal is in.
            let notes_root = project.clone();
            moved |= session.project != project || session.notes_root != notes_root;
            session.project = project;
            session.notes_root = notes_root;
            roots.extend(session.project.clone());
            roots.extend(session.notes_root.clone());
        }
        // The project that governs each folder a band names holds that folder's colour; it may be
        // above the terminal's own (a terminal in a nested project) or none.
        let folders: Vec<PathBuf> = self
            .sessions
            .iter()
            .filter_map(|session| {
                let cwd = session.terminal().read(cx).cwd.clone()?;
                Some(session.repo_root.of(&cwd))
            })
            .collect();
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
        if load_projects(roots, cx) {
            // Other windows (and Settings) may show the project that changed.
            cx.refresh_windows();
        }
        // Settings and theme come from the project of the window in front.
        if self.is_front(cx) {
            self.apply_project_settings(cx);
        }
        if moved {
            cx.notify();
        }
    }

    pub(in crate::workspace) fn active_project<'a>(&self, cx: &'a App) -> Option<&'a LoadedProject> {
        // The one accessor the title tint, menus, palette, shortcut sheet and the Project tab reach
        // a project through, so gating it here makes them all inert the instant the flag is off,
        // without waiting for the next `refresh_projects` tick to clear `session.project`.
        if !project_folders_on(cx) {
            return None;
        }
        let root = self.active_session().project.as_ref()?;
        app(cx).hub.projects.get(root)
    }

    /// Env and shell for a new session started in `cwd`, if its project allows it.
    pub(in crate::workspace) fn project_spawn_options(
        cwd: Option<&Path>,
        cx: &mut App,
    ) -> (BTreeMapEnv, Option<String>) {
        if !project_folders_on(cx) {
            return Default::default();
        }
        let Some(folder) = cwd.and_then(proj::find_project) else {
            return Default::default();
        };
        // Sessions restored at startup spawn before the first refresh; load on demand.
        let root = folder.root.clone();
        let hub = &mut app_mut(cx).hub;
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
        (config.env, config.shell)
    }

    /// Layer the active session's project settings over the user's.
    fn apply_project_settings(&mut self, cx: &mut Context<Self>) {
        let hub = &app(cx).hub;
        let overrides = self
            .active_project(cx)
            .filter(|p| hub.enabled(p, Feature::Settings))
            .and_then(|p| p.snapshot.config())
            .map(|c| c.settings.clone())
            .filter(|s| !s.is_empty());
        if overrides != hub.applied_overrides {
            SettingsStore::set_project_overrides(cx, overrides.clone());
            app_mut(cx).hub.applied_overrides = overrides;
            cx.refresh_windows();
        }

        // Project theme: harmless, so it needs no trust, but can be switched off.
        let hub = &app(cx).hub;
        let theme_json = self
            .active_project(cx)
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
            app_mut(cx).hub.applied_theme = theme_json;
            cx.refresh_windows();
        }
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_project_tab(&mut self, cx: &mut Context<Self>) {
        self.show_tab(RightTab::Project, cx);
    }
}
