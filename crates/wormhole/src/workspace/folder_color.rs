//! A folder's colour, set from a band's menu and the Project tab: which folder a terminal belongs
//! to, where its colour lives, the colour as the theme in force draws it, the tints the band and
//! the title row wear, and the one place it is changed.
//!
//! It lives in one of two places, never both. A folder whose project has a `.wormhole/config.json`
//! keeps it there, as `project_color`: the project's, shared with everyone who uses it, and the
//! person's own colour for that folder (`folder_colors` in Wormhole's `config.json`) is not looked
//! at (it stays in the file, for if the project folder goes away). Every other folder has the
//! person's own, for every window, which never reaches a repository.

use wormhole_core::folder_color::FolderColor;
use wormhole_core::project::{Feature, ProjectColor, ProjectColorChoice};
use wormhole_core::tokens::{Color, ThemeTokens};

use gpui_kit::component::color_picker::ColorPicker;

use super::project::{project_folders_on, set_project_color};
use super::settings_page::update_settings;
use super::title_row::Tint;
use super::*;
use crate::style::BandTint;
use crate::theme_bridge;

/// The most a folder's band is tinted at its left edge, where the title row goes up to 36 %: the
/// band is shorter and its text smaller, and it is meant as a mark in a long list, not a banner.
const BAND_TINT_MAX: f32 = 0.30;
/// How much of its strength the tint has left where the band's text starts, x 40 of a sidebar
/// 350 pt wide (the gradient fades out with `TINT_STOPS`; a wider sidebar leaves less): the second
/// line has to read on that, not on the edge's full strength.
const BAND_TINT_AT_TEXT: f32 = 0.6;

/// Where a folder's colour lives, and what it is now.
#[derive(Clone, Debug)]
pub(in crate::workspace) enum FolderPaint {
    /// No project `config.json` governs the folder: the person's own colour, in Wormhole's settings.
    Own(Option<ProjectColorChoice>),
    /// The folder's project has a `config.json`: its `project_color`.
    Project(ProjectPaint),
}

/// The colour a project's `config.json` gives its folder, as of the snapshot the app holds.
#[derive(Clone, Debug)]
pub(in crate::workspace) struct ProjectPaint {
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
    pub(in crate::workspace) fn checked(&self) -> Option<FolderColor> {
        match self {
            Self::Own(choice) => choice.and_then(ProjectColorChoice::named),
            Self::Project(project) => project.chosen.and_then(ProjectColorChoice::named),
        }
    }

    /// A hex colour, picked or set by the project by hand: shown as it is, matching none of the
    /// names.
    pub(in crate::workspace) fn custom(&self) -> Option<ProjectColor> {
        match self {
            Self::Own(Some(ProjectColorChoice::Hex(color)))
            | Self::Project(ProjectPaint { chosen: Some(ProjectColorChoice::Hex(color)), .. }) => {
                Some(*color)
            }
            _ => None,
        }
    }

    /// What the picker starts from: the colour as the theme `tokens` draws it, if there is one.
    pub(in crate::workspace) fn shown(&self, tokens: &ThemeTokens) -> Option<ProjectColor> {
        self.tint(tokens).map(|(color, _)| color)
    }

    /// What to say instead of offering the colours, when they can't be chosen.
    pub(in crate::workspace) fn closed(&self) -> Option<&'static str> {
        match self {
            Self::Project(project) if project.locked => Some("Set by the project"),
            Self::Project(project) if project.unreadable => Some("config.json can't be read"),
            _ => None,
        }
    }

    /// The line under the Project tab's row: whose colour it is and where it is kept.
    pub(in crate::workspace) fn note(&self) -> String {
        let marks = "Marks this folder's band and its terminals' title row.";
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

    /// How devctl's `rows` says it: ` color=teal` (a hex colour as `color=#2bb3a3`), ` project` when
    /// the project's `config.json` holds it, ` locked`, ` hidden` while its Settings switch is off.
    #[cfg(feature = "devctl")]
    pub(in crate::workspace) fn devctl_words(&self) -> String {
        match self {
            Self::Own(color) => color.map_or(String::new(), |color| format!(" color={}", color.to_value())),
            Self::Project(project) => {
                let color = match project.chosen {
                    Some(ProjectColorChoice::Named(name)) => format!(" color={}", name.key()),
                    Some(ProjectColorChoice::Hex(color)) => format!(" color={}", color.color().to_hex()),
                    None => String::new(),
                };
                let locked = if project.locked { " locked" } else { "" };
                let hidden = if project.shown { "" } else { " hidden" };
                format!("{color} project{locked}{hidden}")
            }
        }
    }

    /// The tint it draws in the theme `tokens`, and the words VoiceOver says for it ("coloured teal").
    pub(in crate::workspace) fn tint(&self, tokens: &ThemeTokens) -> Option<(ProjectColor, &'static str)> {
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
    pub(in crate::workspace) fn folder_paint(&self, folder: &Path, cx: &App) -> FolderPaint {
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
            None => FolderPaint::Own(settings(cx).folder_color_choice(folder)),
        }
    }

    /// The colour the active terminal's folder wears, for the title row's tint. A terminal in a
    /// project of its own below its folder (a nested `.wormhole/`) still shows that project's
    /// colour while the folder has none.
    pub(super) fn folder_tint_color(&self, cx: &App) -> Option<ProjectColor> {
        let tokens = &cx.global::<SettingsStore>().tokens;
        match self.active_folder(cx).map(|folder| self.folder_paint(&folder, cx)) {
            Some(paint @ FolderPaint::Project(_)) => paint.tint(tokens).map(|(color, _)| color),
            Some(paint @ FolderPaint::Own(Some(_))) => paint.tint(tokens).map(|(color, _)| color),
            Some(FolderPaint::Own(None)) | None => self.project_color(cx),
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
        let tokens = &cx.global::<SettingsStore>().tokens;
        let Some(color) = self.folder_paint(&folder, cx).shown(tokens) else { return };
        let color = theme_bridge::hsla(color.color());
        let hex = |color: Hsla| theme_bridge::color(color).with_alpha(1.0).to_hex();
        if self.folder_picker.read(cx).value().map(hex) != Some(hex(color)) {
            self.folder_picker.update(cx, |picker, cx| picker.set_value(color, window, cx));
        }
    }

    /// A band's "Custom colour…": the picker opens under that band, on the colour the folder wears
    /// (the theme's primary while it has none).
    pub(super) fn start_custom_color(
        &mut self,
        folder: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tokens = &cx.global::<SettingsStore>().tokens;
        let start = self
            .folder_paint(&folder, cx)
            .shown(tokens)
            .map(|color| color.color())
            .or_else(|| tokens.get("primary"));
        if let Some(start) = start {
            self.folder_picker
                .update(cx, |picker, cx| picker.set_value(theme_bridge::hsla(start), window, cx));
        }
        self.picking_folder = Some(folder);
        self.folder_picker.update(cx, |picker, cx| picker.set_open(true, cx));
        cx.notify();
    }

    /// The picker as it hangs off the band of `folder` while it is the one being coloured: its
    /// trigger is invisible, the band's menu item was what opened it.
    pub(super) fn band_picker(&self, folder: &Path) -> Option<AnyElement> {
        (self.picking_folder.as_deref() == Some(folder)).then(|| {
            div()
                .absolute()
                .left(px(style::PANEL_INSET))
                .bottom_0()
                .opacity(0.0)
                .child(ColorPicker::new(&self.folder_picker).anchor(Anchor::TopLeft))
                .into_any_element()
        })
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

    /// The tint of `folder`'s band, if it has a colour: the words for it too, for VoiceOver.
    pub(super) fn band_tint(&self, folder: &Path, cx: &App) -> Option<(&'static str, BandTint)> {
        let tokens = &cx.global::<SettingsStore>().tokens;
        let (color, name) = self.folder_paint(folder, cx).tint(tokens)?;
        Some((name, band_tint_of(color, tokens)))
    }
}

/// Colour `folder` with one of the names, or (`None`) take its colour away: [`set_folder_choice`].
pub(in crate::workspace) fn set_folder_color(
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
pub(in crate::workspace) fn set_folder_choice(
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
pub(in crate::workspace) fn choices(cx: &App) -> Vec<(FolderColor, Hsla)> {
    let tokens = &cx.global::<SettingsStore>().tokens;
    FolderColor::ALL.into_iter().map(|color| (color, theme_bridge::hsla(color.color(tokens)))).collect()
}

/// `color` as the title row's tint: as strong as the title's text lets it be over the window.
pub(super) fn title_tint(color: ProjectColor, cx: &App) -> Tint {
    let store = cx.global::<SettingsStore>();
    let bar = store.tokens.get("surface.window").map_or(Color::BLACK, |c| c.with_alpha(1.0));
    let text = store.tokens.get("text.primary").unwrap_or(Color::WHITE);
    Tint { color, peak: color.tint_peak(bar, text), line: theme_bridge::hsla(color.line(bar)) }
}

/// `color` as a band's tint in a theme: as strong as the band's second line (muted text) lets it
/// be where that line starts, and never past [`BAND_TINT_MAX`]. The Studio's gallery draws it from the
/// draft theme too.
pub(in crate::workspace) fn band_tint_in(color: FolderColor, tokens: &ThemeTokens) -> BandTint {
    band_tint_of(ProjectColor::from_color(color.color(tokens)), tokens)
}

/// [`band_tint_in`] for a colour that is already a tint (a project's hex colour, or a name resolved).
fn band_tint_of(color: ProjectColor, tokens: &ThemeTokens) -> BandTint {
    let band = tokens.get("band.solid").map_or(Color::BLACK, |c| c.with_alpha(1.0));
    let muted = tokens.get("text.muted").unwrap_or(Color::WHITE);
    BandTint {
        color: theme_bridge::hsla(color.color()),
        peak: (color.tint_peak(band, muted) / BAND_TINT_AT_TEXT).min(BAND_TINT_MAX),
    }
}
