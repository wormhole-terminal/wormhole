//! A folder's colour, set from a band's menu and the Project tab: which folder a terminal belongs
//! to, where its colour lives, the colour as the theme in force draws it, the tints the band and
//! the title row wear, and the one place it is changed.
//!
//! It lives in one of two places, never both. A folder whose project has a `.wormhole/config.json`
//! keeps it there, as `project_color`: the project's, shared with everyone who uses it, and the
//! person's own colour for that folder (`folder_colors` in Wormhole's `config.json`) is not looked
//! at (it stays in the file, for if the project folder goes away). Every other folder has the
//! person's own, for every window, which never reaches a repository.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::project::{Feature, ProjectColor, ProjectColorChoice};
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::theme::folder_color::FolderColor;
use wormhole_core::theme::tokens::{Color, ThemeTokens};

use crate::app_state::app;
use crate::settings::update_settings;
use crate::settings::{SettingsStore, settings, settings_in, store_in};
use crate::theme_bridge;
use crate::workspace::Workspace;
use crate::workspace::features::project::{project_folders_on, set_project_color};
use crate::workspace::title::Tint;

/// Where a folder's colour lives, and what it is now.
#[derive(Clone, Debug)]
pub(in crate::window) enum FolderPaint {
    /// No project `config.json` governs the folder: the person's own colour, in Wormhole's settings.
    Own(Option<ProjectColorChoice>),
    /// The folder's project has a `config.json`: its `project_color`.
    Project(ProjectPaint),
}

/// The colour a project's `config.json` gives its folder, as of the snapshot the app holds.
#[derive(Clone, Debug)]
pub(in crate::window) struct ProjectPaint {
    pub root: PathBuf,
    /// What `project_color` says, when it is a colour.
    pub chosen: Option<ProjectColorChoice>,
    /// `project_color_locked`: only editing the file changes the colour.
    pub locked: bool,
    /// The project's Settings switch is on, which a project colour follows (it needs no trust).
    pub shown: bool,
    /// `config.json` doesn't load, so there is no telling what it says and nothing to edit.
    pub unreadable: bool,
}

impl FolderPaint {
    /// The name that is the current colour; a hex colour a project sets is none of the eight.
    pub(in crate::window) fn checked(&self) -> Option<FolderColor> {
        match self {
            Self::Own(choice) => choice.and_then(ProjectColorChoice::named),
            Self::Project(project) => project.chosen.and_then(ProjectColorChoice::named),
        }
    }

    /// A hex colour, picked or set by the project by hand: shown as it is, matching none of the
    /// names.
    pub(in crate::window) fn custom(&self) -> Option<ProjectColor> {
        match self {
            Self::Own(Some(ProjectColorChoice::Hex(color)))
            | Self::Project(ProjectPaint { chosen: Some(ProjectColorChoice::Hex(color)), .. }) => {
                Some(*color)
            }
            _ => None,
        }
    }

    /// What the picker starts from: the colour as the theme `tokens` draws it, if there is one.
    pub(in crate::window) fn shown(&self, tokens: &ThemeTokens) -> Option<ProjectColor> {
        self.tint(tokens).map(|(color, _)| color)
    }

    /// What to say instead of offering the colours, when they can't be chosen.
    pub(in crate::window) fn closed(&self) -> Option<&'static str> {
        match self {
            Self::Project(project) if project.locked => Some("Set by the project"),
            Self::Project(project) if project.unreadable => Some("config.json can't be read"),
            _ => None,
        }
    }

    /// The line under the Project tab's row: whose colour it is and where it is kept.
    pub(in crate::window) fn note(&self) -> String {
        let marks = "Marks this folder's band.";
        match self {
            Self::Own(_) => format!("{marks} Yours only: it isn't saved in the project."),
            Self::Project(project) if project.locked => {
                format!(
                    "{marks} Set by the project: its config.json locks the colour. Edit the file to change it."
                )
            }
            Self::Project(project) if project.unreadable => {
                format!(
                    "{marks} The colour is in the project's config.json, which can't be read: fix the error below."
                )
            }
            Self::Project(project) => {
                let shared = "Saved in the project's config.json, so everyone who uses the project sees it.";
                let off =
                    (!project.shown).then_some(" Not shown while Settings is switched off for this project.");
                format!("{marks} {shared}{}", off.unwrap_or_default())
            }
        }
    }

    /// The tint it draws in the theme `tokens`, and the words VoiceOver says for it ("coloured teal").
    pub(in crate::window) fn tint(&self, tokens: &ThemeTokens) -> Option<(ProjectColor, &'static str)> {
        match self {
            Self::Own(choice) => {
                let choice = (*choice)?;
                Some((choice.resolve(tokens), choice.named().map_or("custom", FolderColor::label)))
            }
            Self::Project(project) => {
                let chosen = project.chosen.filter(|_| project.shown)?;
                Some((chosen.resolve(tokens), chosen.named().map_or("by its project", FolderColor::label)))
            }
        }
    }
}

impl Workspace {
    /// The folder the active terminal belongs to, as the sidebar's bands name it: its repository,
    /// else the folder itself. `None` until its shell has said where it is.
    pub(super) fn active_folder(&self, cx: &App) -> Option<PathBuf> {
        let cwd = self.active_terminal().read(cx).cwd.clone()?;
        Some(self.active_session().repo_root.of(&cwd))
    }

    /// Where `folder`'s colour lives and what it is: its project's `project_color` when the project
    /// that governs it has a `config.json`, else the person's own. No disk access: the project is
    /// the one `refresh_projects` found and the hub holds.
    pub(in crate::window) fn folder_paint(&self, folder: &Path, cx: &App) -> FolderPaint {
        // With the project-folder feature off a folder only has the person's own colour, at once:
        // `folder_projects` still holds the old projects until the next refresh.
        let project = project_folders_on(cx)
            .then(|| self.folder_projects.get(folder))
            .flatten()
            .and_then(|root| app(cx).hub.projects.get(root))
            .filter(|project| project.snapshot.has(Feature::Settings));
        match project {
            Some(project) => FolderPaint::Project(ProjectPaint {
                root: project.folder.root.clone(),
                chosen: project.snapshot.config().and_then(|config| config.color),
                locked: project.snapshot.config().is_some_and(|config| config.color_locked),
                shown: app(cx).hub.prefs.enabled(&project.folder.root, Feature::Settings),
                unreadable: project.snapshot.config_error().is_some(),
            }),
            None => FolderPaint::Own(settings_in(&self.profile, cx).folder_color_choice(folder)),
        }
    }

    /// Point the Project tab's picker at the colour the active folder wears, unless it already
    /// shows it. Compared as hex, so a slider drag in progress (whose colour was just saved) is
    /// not rewritten under the person's hand.
    pub(super) fn sync_folder_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A picker hanging off a band shows that band's folder, which it was pointed at.
        if self.picking_folder.is_some() {
            return;
        }
        let Some(folder) = self.active_folder(cx) else { return };
        let tokens = &store_in(&self.profile, cx).tokens;
        let Some(color) = self.folder_paint(&folder, cx).shown(tokens) else { return };
        let color = theme_bridge::hsla(color.color());
        let hex = |color: Hsla| theme_bridge::color(color).with_alpha(1.0).to_hex();
        if self.folder_picker.read(cx).value().map(hex) != Some(hex(color)) {
            self.folder_picker.update(cx, |picker, cx| picker.set_value(color, window, cx));
        }
    }

    /// A colour picked in the Project tab's picker, for the active terminal's folder: kept as the
    /// hex colour it is, where the folder's colour lives.
    pub(super) fn pick_folder_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        let Some(folder) = self.picking_folder.clone().or_else(|| self.active_folder(cx)) else { return };
        let Ok(picked) = ProjectColor::parse(&theme_bridge::color(color).with_alpha(1.0).to_hex()) else {
            return;
        };
        let paint = self.folder_paint(&folder, cx);
        set_folder_choice(&paint, &folder, Some(ProjectColorChoice::Hex(picked)), cx);
    }
}

/// Colour `folder` with one of the names, or (`None`) take its colour away: [`set_folder_choice`].
pub(in crate::window) fn set_folder_color(
    paint: &FolderPaint,
    folder: &Path,
    color: Option<FolderColor>,
    cx: &mut App,
) {
    set_folder_choice(paint, folder, color.map(ProjectColorChoice::Named), cx);
}

/// Colour `folder`, or (`None`) take its colour away, where `paint` says it lives: for the person's
/// own, the one list of every window saved in `config.json`; for a project's, `project_color` in
/// its `config.json`, which every window then reads again.
pub(in crate::window) fn set_folder_choice(
    paint: &FolderPaint,
    folder: &Path,
    color: Option<ProjectColorChoice>,
    cx: &mut App,
) {
    // A menu opened before the project-folder flag went off still holds the project's paint; with
    // the feature off the pick is the person's own colour, never a write into the repository.
    let project = match paint {
        FolderPaint::Project(project) if project_folders_on(cx) => project,
        _ => {
            if settings(cx).folder_color_choice(folder) != color {
                update_settings(cx, |settings| {
                    settings.set_folder_color_choice(folder, color);
                });
            }
            return;
        }
    };
    // The menus don't offer it while closed; this is for a menu that was open when it closed.
    if project.locked || project.unreadable || project.chosen == color {
        return;
    }
    set_project_color(&project.root, color, cx);
}

/// The colours a folder can wear, as the theme in force draws them.
pub(in crate::window) fn choices(cx: &App) -> Vec<(FolderColor, Hsla)> {
    let tokens = &cx.global::<SettingsStore>().tokens;
    FolderColor::ALL.into_iter().map(|color| (color, theme_bridge::hsla(color.color(tokens)))).collect()
}

/// `color` as the title row's tint: as strong as the title's text lets it be over the window.
pub(super) fn title_tint(profile: &ProfileId, color: ProjectColor, cx: &App) -> Tint {
    let store = store_in(profile, cx);
    let bar = store.tokens.get("surface.window").map_or(Color::BLACK, |c| c.with_alpha(1.0));
    let text = store.tokens.get("text.primary").unwrap_or(Color::WHITE);
    Tint { color, peak: color.tint_peak(bar, text), line: theme_bridge::hsla(color.line(bar)) }
}
