//! Saving the draft: Save theme (into the themes folder, then switched to), Save Copy
//! and Save Copy and Switch, Export JSON, Copy JSON and Use in this project; the notice each
//! leaves under the title row; and the two questions, sheets on the window as macOS asks them:
//! a name that is taken, and closing with edits.

use std::path::{Path, PathBuf};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use style::BannerKind;
use wormhole_core::project::{Feature, ProjectFolder, TrustStatus};
use wormhole_core::storage::fsutil::{self, LinkPolicy};
use wormhole_core::theme::tokens::ThemeFile;

use super::draft::{changes, copy_name, export_dir, export_file_name, export_json};
use super::{StudioWindow, ui};
use crate::app_state::AppState;
use crate::settings::SettingsStore;
use crate::style::{self, Edges};
use crate::window::features::project;

/// The app record (`Store::meta`) of the folder the last export went to, so the next one's
/// dialog starts there, across launches.
const EXPORT_DIR_KEY: &str = "studio_export_dir";

/// The close question's answers, by position: Save is the default (Return), Cancel takes Esc,
/// and Don't Save is the one that loses the edits.
const CLOSE_SAVE: usize = 0;
const CLOSE_DONT_SAVE: usize = 2;
/// The name clash's answers: Replace is the default, Cancel takes Esc.
const REPLACE: usize = 0;

/// What the last save, export or copy did, shown under the title row until the next edit.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Notice {
    /// Written to `path`; `text` says what that means. Reveal in Finder shows the file.
    Saved {
        path: PathBuf,
        text: SharedString,
    },
    /// Done, with nothing to reveal (Copy JSON).
    Done(SharedString),
    Failed(SharedString),
}

impl StudioWindow {
    /// The name the draft is saved under, or why there is none.
    pub(super) fn theme_name(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let name = self.draft.name.clone().unwrap_or_default().trim().to_string();
        if name.is_empty() {
            self.notice = Some(Notice::Failed("Give the theme a name first.".into()));
            cx.notify();
            return None;
        }
        Some(name)
    }

    /// Save theme (⌘S): write the theme and switch to it, unless the themes folder already has
    /// one by that name, which a sheet asks about first.
    pub(super) fn request_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save(false, window, cx);
    }

    /// Save, and with `then_close` (the close question's Save) close the window once the save
    /// went through. The window stays while a name clash waits for its answer, and when the
    /// save fails, with the notice saying why.
    fn save(&mut self, then_close: bool, window: &mut Window, cx: &mut Context<Self>) {
        // A sheet is up: it has to be answered first.
        if self.asking {
            return;
        }
        if self.renaming {
            self.finish_rename(window, cx);
        }
        let Some(name) = self.theme_name(cx) else { return };
        if (self.themes.saved_path)(&name).is_some() {
            self.ask_to_replace(name, then_close, window, cx);
            return;
        }
        self.write_theme(name, cx);
        if then_close && !self.dirty {
            window.remove_window();
        }
    }

    /// Save the draft as `name`, switch to it, and stop previewing.
    pub(super) fn write_theme(&mut self, name: String, cx: &mut Context<Self>) {
        self.draft.name = Some(name.clone());
        match (self.themes.save)(&self.draft) {
            Ok(path) => {
                self.switched_to(&name, cx);
                let text = format!("Saved to {} and set as your theme.", display_path(&path));
                self.notice = Some(Notice::Saved { path, text: text.into() });
            }
            Err(err) => self.notice = Some(Notice::Failed(format!("Could not save: {err}").into())),
        }
        cx.notify();
    }

    /// The draft, as it stands, is now the theme `name` in the themes folder: the app switches
    /// to it, the preview ends (the theme itself shows now), and there is nothing left to lose.
    fn switched_to(&mut self, name: &str, cx: &mut Context<Self>) {
        self.dirty = false;
        self.initial = self.draft.clone();
        self.revision += 1;
        SettingsStore::update(cx, |s| s.theme = name.to_string());
        SettingsStore::preview_tokens(cx, None);
        cx.refresh_windows();
    }

    /// Save Copy (⇧⌘S) and Save Copy and Switch: the draft under a free "… copy" name.
    /// Plain, the theme in use and the draft stay as they are; with `switch`, the app starts
    /// using the copy and the window goes on editing it.
    pub(super) fn save_copy(&mut self, switch: bool, cx: &mut Context<Self>) {
        let Some(name) = self.theme_name(cx) else { return };
        let taken = self.themes.saved_path;
        let copy = copy_name(&name, |candidate| taken(candidate).is_some());
        let file = ThemeFile { name: Some(copy.clone()), ..self.draft.clone() };
        self.notice = Some(match (self.themes.save)(&file) {
            Ok(path) if switch => {
                self.draft = file;
                self.switched_to(&copy, cx);
                let text = format!(
                    "Saved a copy as \u{201c}{copy}\u{201d} to {} and set as your theme.",
                    display_path(&path)
                );
                Notice::Saved { path, text: text.into() }
            }
            Ok(path) => {
                let text = format!("Saved a copy as \u{201c}{copy}\u{201d} to {}.", display_path(&path));
                Notice::Saved { path, text: text.into() }
            }
            Err(err) => Notice::Failed(format!("Could not save a copy: {err}").into()),
        });
        cx.notify();
    }

    /// Export JSON…: the draft as a theme file, where the person picks. The dialog asks every
    /// time and starts where the last export went, the home folder the first time.
    pub(super) fn export(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let json = export_json(&self.draft);
        let file_name = export_file_name(self.draft.name.as_deref().unwrap_or("theme"));
        let remembered = store(cx).and_then(|store| store.meta(EXPORT_DIR_KEY).ok().flatten());
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let dir = export_dir(remembered.as_deref(), home, |dir| dir.is_dir());
        let chosen = cx.prompt_for_new_path(&dir, Some(&file_name));
        self.export = Some(cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = chosen.await else { return };
            // The person's own file: a link there is theirs to keep.
            let notice = match fsutil::write_atomic(&path, json.as_bytes(), LinkPolicy::Follow) {
                Ok(()) => {
                    let text = format!("Exported to {}.", display_path(&path));
                    Notice::Saved { path, text: text.into() }
                }
                Err(err) => Notice::Failed(format!("Could not export: {err}").into()),
            };
            this.update(cx, |studio, cx| {
                if let Notice::Saved { path, .. } = &notice
                    && let Some(dir) = path.parent()
                    && let Some(store) = store(cx)
                    && let Err(err) = store.set_meta(EXPORT_DIR_KEY, &dir.to_string_lossy())
                {
                    tracing::warn!("remembering the export folder failed: {err:#}");
                }
                studio.notice = Some(notice);
                cx.notify();
            })
            .ok();
        }));
    }

    /// Copy JSON: the same file, on the clipboard.
    pub(super) fn copy_json(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(export_json(&self.draft)));
        self.notice = Some(Notice::Done("Copied the theme's JSON.".into()));
        cx.notify();
    }

    /// Use in this project…: the draft as the `.wormhole/theme.json` of the project the
    /// terminal in front is in. Only a trusted project with its theme switched on gets one.
    pub(super) fn use_in_project(&mut self, cx: &mut Context<Self>) {
        let folder = match self.project_for_theme(cx) {
            Ok(folder) => folder,
            Err(why) => {
                self.notice = Some(Notice::Failed(why.into()));
                cx.notify();
                return;
            }
        };
        self.notice = Some(match folder.write_theme(&export_json(&self.draft)) {
            Ok(path) => {
                // Saved where it applies: closing no longer loses it.
                self.dirty = false;
                self.initial = self.draft.clone();
                self.revision += 1;
                let text = format!("Saved to {}: terminals in this project use it.", display_path(&path));
                Notice::Saved { path, text: text.into() }
            }
            Err(err) => Notice::Failed(format!("Could not save to the project: {err}").into()),
        });
        cx.notify();
    }

    /// The project "Use in this project" writes to: that of the terminal in front, when the
    /// project is trusted and its theme isn't switched off (`ProjectHub::enabled` once the
    /// file exists). Otherwise what stands in the way, in the person's words.
    pub(super) fn project_for_theme(&self, cx: &App) -> Result<ProjectFolder, &'static str> {
        let no_terminal = "Open a terminal in a project first.";
        if cx.try_global::<AppState>().is_none() {
            return Err(no_terminal);
        }
        let front = AppState::front_window(cx).ok_or(no_terminal)?;
        let root = front
            .workspace
            .read_with(cx, project::api::active_root)
            .ok()
            .flatten()
            .ok_or("The terminal in front isn't in a project with a .wormhole folder.")?;
        let hub = project::hub(cx);
        let project = hub.projects.get(&root).ok_or("The project in front hasn't loaded yet.")?;
        if hub.trust.status_of(&project.snapshot) != TrustStatus::Trusted {
            return Err("Trust this project first (Project tab), then use the theme in it.");
        }
        let switched_on = if project.snapshot.has(Feature::Theme) {
            hub.enabled(project, Feature::Theme)
        } else {
            hub.prefs.enabled(&root, Feature::Theme)
        };
        if !switched_on {
            return Err("This project's theme is switched off (Project tab).");
        }
        Ok(project.folder.clone())
    }

    /// The notice under the title row, if there is one.
    pub(super) fn render_notice(&self, background: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ui = ui(cx);
        let notice = self.notice.clone()?;
        let banner = match notice {
            Notice::Saved { path, text } => style::banner(ui, BannerKind::Success, false, text).child(
                Button::new("studio-reveal")
                    .ghost()
                    .small()
                    .label("Reveal in Finder")
                    .on_click(move |_, _, cx| cx.reveal_path(&path)),
            ),
            Notice::Done(text) => style::banner(ui, BannerKind::Success, false, text),
            Notice::Failed(text) => style::banner(ui, BannerKind::Danger, false, text),
        };
        Some(div().flex_none().px_4().py_3().bg(background).rule_bottom(ui).child(banner).into_any_element())
    }

    /// Esc, ⌘W or the red light with edits: a sheet on the window, as macOS asks it.
    /// Return saves, Esc cancels, and Don't Save closes with the edits lost. The sheet
    /// is modal for the window, so nothing else in the Studio runs until it is answered.
    pub(super) fn ask_before_closing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = true;
        let name = self.draft.name.clone().unwrap_or_default();
        let count = changes(&self.initial, &self.draft);
        let base = cx.global::<SettingsStore>().base_tokens().name.clone();
        let message = format!("Do you want to save the changes to \u{201c}{name}\u{201d}?");
        let detail = format!(
            "{count} {}. If you don\u{2019}t save, the app goes back to {base}.",
            if count == 1 { "change" } else { "changes" }
        );
        let buttons = [
            PromptButton::ok("Save"),
            PromptButton::cancel("Cancel"),
            PromptButton::new("Don\u{2019}t Save"),
        ];
        let answer = window.prompt(PromptLevel::Warning, &message, Some(&detail), &buttons, cx);
        self.question = Some(cx.spawn_in(window, async move |this, cx| {
            // The window going first (the app quitting) answers with nothing: as Cancel.
            let answer = answer.await.ok();
            this.update_in(cx, |studio, window, cx| {
                studio.asking = false;
                match answer {
                    Some(CLOSE_SAVE) => studio.save(true, window, cx),
                    Some(CLOSE_DONT_SAVE) => {
                        studio.dirty = false;
                        window.remove_window();
                    }
                    _ => {}
                }
            })
            .ok();
        }));
    }

    /// Saving under a name the themes folder has: a sheet asks whether to replace that
    /// theme. Cancel leaves the draft as it is, to save under another name (click the name).
    fn ask_to_replace(
        &mut self,
        name: String,
        then_close: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.asking = true;
        let message = format!("\u{201c}{name}\u{201d} already exists. Do you want to replace it?");
        let detail = "A theme with the same name is in your themes folder. \
                      Replacing it overwrites that theme with this draft.";
        let buttons = [PromptButton::ok("Replace"), PromptButton::cancel("Cancel")];
        let answer = window.prompt(PromptLevel::Warning, &message, Some(detail), &buttons, cx);
        self.question = Some(cx.spawn_in(window, async move |this, cx| {
            let replace = answer.await.ok() == Some(REPLACE);
            this.update_in(cx, |studio, window, cx| {
                studio.asking = false;
                if replace {
                    studio.write_theme(name, cx);
                    if then_close && !studio.dirty {
                        window.remove_window();
                    }
                }
            })
            .ok();
        }));
    }
}

/// The app's store, where there is one (not in a headless test).
fn store(cx: &App) -> Option<&wormhole_core::storage::store::Store> {
    cx.try_global::<AppState>()?.store.as_ref()
}

/// A path with the home folder as `~`.
fn display_path(path: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    wormhole_core::view::sidebar::display_path(path, home.as_deref())
}
