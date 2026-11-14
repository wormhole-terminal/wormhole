//! A folder's colour, where it lives and who may change it: in the project's `config.json` as
//! `project_color` when the project that governs the folder has one (shared with everyone who uses it),
//! else the person's own in `config.json` (`folder_colors`), which never reaches a repository. A project
//! that locks its colour, or whose file can't be read, is never edited. The person's own is in the settings of
//! the window's own profile (the one whose stores the service holds), as the GPUI app's window reads its own.

use std::path::Path;

use wormhole_core::config::Config;
use wormhole_core::project::{self as proj, Feature, ProjectColorChoice, ProjectFolder};
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};

use super::records::{FolderPaintRecord, ProjectFailure};
use super::service::ProjectService;
use crate::files_panel::names;
use crate::files_panel::settings_edit::update_settings_of;

/// Where a folder's colour lives, and what it is now.
enum Paint {
    Own(Option<ProjectColorChoice>),
    Project {
        root: std::path::PathBuf,
        chosen: Option<ProjectColorChoice>,
        locked: bool,
        /// The project's Settings switch is on, which its colour follows (it needs no trust).
        shown: bool,
        /// `config.json` doesn't load: there is no telling what it says and nothing to edit.
        unreadable: bool,
    },
}

impl Paint {
    fn choice(&self) -> Option<ProjectColorChoice> {
        match self {
            Self::Own(choice) | Self::Project { chosen: choice, .. } => *choice,
        }
    }

    /// The line under the row: whose colour it is and where it is kept.
    fn note(&self) -> String {
        let marks = "Marks this folder's band.";
        match self {
            Self::Own(_) => format!("{marks} Yours only: it isn't saved in the project."),
            Self::Project { locked: true, .. } => format!(
                "{marks} Set by the project: its config.json locks the colour. Edit the file to change it."
            ),
            Self::Project { unreadable: true, .. } => format!(
                "{marks} The colour is in the project's config.json, which can't be read: fix the error below."
            ),
            Self::Project { shown, .. } => {
                let shared = "Saved in the project's config.json, so everyone who uses the project sees it.";
                let off = (!shown).then_some(" Not shown while Settings is switched off for this project.");
                format!("{marks} {shared}{}", off.unwrap_or_default())
            }
        }
    }
}

impl ProjectService {
    /// The files of the profile whose stores the service holds: the window's own. A folder that is no profile's
    /// (a test's) reads the Default profile's settings.
    fn own_profile(&self) -> ProfilePaths {
        let root = DataRoot::current();
        let hub = self.lock();
        let named = hub
            .dir()
            .strip_prefix(root.profiles_dir())
            .ok()
            .and_then(|rest| rest.to_str())
            .and_then(|name| ProfileId::parse(name).ok())
            .map(|id| root.profile(&id))
            .filter(|paths| paths.dir() == hub.dir());
        named.unwrap_or_else(|| root.default_profile())
    }

    fn paint_of(&self, folder: &Path) -> Paint {
        // With project folders off the colour is the person's own: nothing is written into a project.
        if !self.off()
            && let Some(found) = proj::find_project(folder)
        {
            let mut hub = self.lock();
            let snapshot = hub.snapshot(&found);
            if snapshot.has(Feature::Settings) {
                let config = snapshot.config();
                return Paint::Project {
                    root: found.root,
                    chosen: config.and_then(|config| config.color),
                    locked: config.is_some_and(|config| config.color_locked),
                    shown: hub.prefs.enabled(snapshot.root(), Feature::Settings),
                    unreadable: snapshot.config_error().is_some(),
                };
            }
        }
        Paint::Own(Config::load_for(&self.own_profile()).settings.folder_color_choice(folder))
    }
}

#[uniffi::export]
impl ProjectService {
    /// What `folder`'s colour is, where it is kept and whether it may be changed.
    pub fn folder_paint(&self, folder: String) -> FolderPaintRecord {
        let paint = self.paint_of(&names::folder(&folder));
        let choice = paint.choice();
        FolderPaintRecord {
            chosen: choice.and_then(ProjectColorChoice::named).map(|name| name.key().to_string()),
            custom: match choice {
                Some(ProjectColorChoice::Hex(color)) => Some(color.color().to_hex()),
                _ => None,
            },
            locked: matches!(
                paint,
                Paint::Project { locked: true, .. } | Paint::Project { unreadable: true, .. }
            ),
            note: paint.note(),
        }
    }

    /// Colours `folder` with a name (`teal`) or a hex colour (`#7fb2ff`), or (`None`) takes its colour
    /// away, where the colour lives. A project's own colour is changed in its `config.json` by a splice
    /// that keeps the comments; a locked or unreadable one is refused.
    pub fn set_folder_color(&self, folder: String, color: Option<String>) -> Result<(), ProjectFailure> {
        let choice =
            color.map(|text| ProjectColorChoice::parse(&text)).transpose().map_err(ProjectFailure::failed)?;
        let folder = names::folder(&folder);
        let folder = folder.as_path();
        match self.paint_of(folder) {
            Paint::Own(current) => {
                if current != choice {
                    update_settings_of(&self.own_profile(), |settings| {
                        settings.set_folder_color_choice(folder, choice)
                    });
                }
            }
            Paint::Project { locked: true, .. } => {
                return Err(ProjectFailure::refused("the project locks its colour"));
            }
            Paint::Project { unreadable: true, .. } => {
                return Err(ProjectFailure::refused("the project's config.json can't be read"));
            }
            Paint::Project { root, chosen, .. } => {
                if chosen != choice {
                    let project = ProjectFolder::open(&root)
                        .ok_or_else(|| ProjectFailure::not_found("the project folder"))?;
                    project.set_project_color_choice(choice).map_err(ProjectFailure::failed)?;
                }
            }
        }
        self.changed();
        Ok(())
    }
}
