//! `.wormhole/` project folders: which project each terminal is in, trust (approved with Touch ID),
//! per-part switches, the project's settings, theme, environment and shell, actions, prompts, a
//! terminal layout, the folder's colour and the two-way `notes.md` sync.
//!
//! Every part is optional and can be switched off per project, so a committed `.wormhole/` never
//! forces anything on a collaborator. Nothing executable runs until the folder is trusted.
//!
//! The feature has two kinds of state. What the app knows of the projects is the profile's hub (`hub`
//! is one profile's, `hubs` holds each profile's and says which is shown, `projects` keeps it current).
//! What a window knows is the [`Project`] entity, one per window, holding for each terminal the project it
//! is in (`detect` looks), for each folder its project (whose colour it holds) and the colour picker. The
//! window talks to it through the table (`def`): the Project tab (`panel`), the run-an-action key and
//! menu lines, the tick, the terminal's start (`spawn`), and the terminal's life (`opened`, `closed`,
//! `left`); it talks to the window by `FeatureEvent`s (a terminal to type into, terminals to open).
//!
//! The rest: `trust` (approval and the switches, with `trust_window`), `actions` (actions, the layout, the
//! prompts, the action keys), `create` (making a project folder), `color` (writing its colour) and
//! `folder_color` (whose colour a folder wears), `notes_sync` (the one file that works with the notes
//! feature, through its `api`) and `api` (what the surfaces above the window may ask).

mod actions;
pub(crate) mod api;
mod color;
mod create;
mod def;
mod detect;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod folder_color;
#[cfg(test)]
mod folder_color_tests;
mod hub;
mod hubs;
mod notes_sync;
mod panel;
mod projects;
#[cfg(test)]
mod tests;
mod trust;
mod trust_window;

use std::collections::HashMap;
use std::path::PathBuf;

use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::*;
use wormhole_core::project::NotesDoc;
use wormhole_core::storage::paths::ProfileId;

use crate::window::feature::FeatureEvent;

pub(crate) use create::create_project_folder;
pub(crate) use def::DEF;
pub(crate) use hub::{LoadedProject, ProjectHub, load_project};
#[cfg(test)]
pub(crate) use hubs::hub_mut;
use hubs::profiles;
pub(crate) use hubs::{hub, hub_of, hub_of_mut, shown_profile};
pub(crate) use projects::{known_projects, load_project_info, prune_projects};
use projects::{load_projects, projects_changed, reload_projects};
pub(crate) use trust::{approve_project, revoke_project, set_feature_default, set_feature_for};
#[cfg(feature = "devctl")]
pub(crate) use trust_window::devctl_trust_shot;
pub(crate) use trust_window::open_trust_window;

/// Whether the project-folder feature is on (the feature table's mask: its flag, see core `features`).
/// Everything that reads or writes a `.wormhole/` asks this itself instead of trusting that a refresh
/// has already cleared the window's projects: a settings change doesn't run that refresh, so for up to
/// a tick after the flag goes off the old projects are still there.
pub(crate) fn project_folders_on(profile: &ProfileId, cx: &App) -> bool {
    crate::settings::features_in(profile, cx).has(wormhole_core::config::FeatureId::Project)
}

/// The window's project state: every terminal's, by its id, the project each folder a band names is in
/// and the Project tab's colour picker.
pub(crate) struct Project {
    of: HashMap<i64, ProjectOf>,
    /// The root of the `.wormhole/` project that governs each folder a band names (the terminals' and the
    /// pinned ones), as of the last look: the folder's colour lives in it, found without touching the
    /// disk. A folder in no project isn't in it.
    folder_projects: HashMap<PathBuf, PathBuf>,
    /// The Project tab's colour picker for the active terminal's folder.
    picker: Entity<ColorPickerState>,
    _picker_subscription: Subscription,
    /// The profile and folder the tab was last drawn for: what a colour picked in the picker is for. The
    /// picker's event comes with neither, and the window isn't asked (a feature never reaches it).
    drawn_for: Option<(ProfileId, PathBuf)>,
}

impl EventEmitter<FeatureEvent> for Project {}

/// One terminal's project.
#[derive(Default)]
struct ProjectOf {
    /// Root of the `.wormhole/` project the terminal's folder belongs to, as of the last look. Its notes
    /// sync with that project's `notes.md`.
    root: Option<PathBuf>,
    /// Last notes and tasks state agreed with the project's notes.md (the merge base), for this run.
    notes_base: Option<NotesDoc>,
}

impl Project {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let subscription = cx.subscribe_in(&picker, window, |this, _, event: &ColorPickerEvent, _, cx| {
            let ColorPickerEvent::Change(Some(color)) = event else { return };
            this.pick_color(*color, cx);
        });
        Self {
            of: HashMap::new(),
            folder_projects: HashMap::new(),
            picker,
            _picker_subscription: subscription,
            drawn_for: None,
        }
    }

    /// The project terminal `session` is in, from the hub of `profile`, the window's: none while the
    /// feature is off, whatever the last look found.
    pub(super) fn project<'a>(
        &self,
        session: i64,
        profile: &ProfileId,
        cx: &'a App,
    ) -> Option<&'a LoadedProject> {
        // The one accessor the menus, the palette, the shortcut sheet and the Project tab reach a project
        // through, so gating it here makes them all inert the instant the flag is off, without waiting
        // for the next look to clear the terminal's project.
        if !project_folders_on(profile, cx) {
            return None;
        }
        hub_of(profile, cx).projects.get(self.of.get(&session)?.root.as_ref()?)
    }

    /// A terminal was opened in the window: its entry, empty until the window looks.
    fn open(&mut self, session: i64) {
        self.of.insert(session, ProjectOf::default());
    }

    /// The terminal is gone from this window (closed, or moved to another one).
    fn forget(&mut self, session: i64) {
        self.of.remove(&session);
    }
}
