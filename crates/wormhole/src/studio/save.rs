//! Saving the draft: Save theme (into the themes folder, then switched to), Save Copy
//! and Save Copy and Switch, Export JSON, Copy JSON and Use in this project; the notice each
//! leaves under the title row; and the two questions, sheets on the window as macOS asks them:
//! a name that is taken, and closing with edits.

use std::path::{Path, PathBuf};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use style::BannerKind;
use wormhole_core::platform::paths;
use wormhole_core::project::ProjectFolder;
use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::tokens::{Appearance, ThemeFile};

use super::document::{Subject, changes};
use super::draft::{copy_name, export_dir, export_file_name, export_json};
use super::themes::{SaveError, Saved};
use super::{StudioWindow, ui};
use crate::app_state::AppState;
use crate::features::project;
use crate::settings::{SettingsStore, picks};
use crate::style::{self, Edges};

/// The app record (`Store::meta`) of the folder the last export went to, so the next one's
/// dialog starts there, across launches.
const EXPORT_DIR_KEY: &str = "studio_export_dir";

/// The close question's answers, by position: Save is the default (Return), Cancel takes Esc,
/// and Don't Save is the one that loses the edits.
const CLOSE_SAVE: usize = 0;
const CLOSE_DONT_SAVE: usize = 2;
/// The name clash's answers: Replace is the default, Cancel takes Esc.
const REPLACE: usize = 0;
/// The changed-on-disk question's: Cancel is the default (Return), Overwrite the second button.
const OVERWRITE: usize = 1;

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
        let name = self.doc.name().trim().to_string();
        if name.is_empty() {
            self.notice = Some(Notice::Failed("Give the theme a name first.".into()));
            cx.notify();
            return None;
        }
        Some(name)
    }

    /// Save theme (⌘S): a theme with a file is written over it, with no question; a copy with no
    /// file yet asks for its name first.
    pub(super) fn request_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save(false, window, cx);
    }

    /// Save, and with `then_close` (the close question's Save) close the window once the save
    /// went through. The window stays while a name or a question waits for its answer, and when
    /// the save fails, with the notice saying why.
    pub(super) fn save(&mut self, then_close: bool, window: &mut Window, cx: &mut Context<Self>) {
        // A sheet is up: it has to be answered first.
        if self.asking {
            return;
        }
        if self.renaming {
            if self.naming.is_some() {
                return self.commit_name(window, cx);
            }
            self.finish_rename(window, cx);
        }
        match self.doc.subject.clone() {
            Subject::Own { path } => self.write_own(path, true, then_close, window, cx),
            Subject::Untitled { .. } => {
                // The name is asked for in the title row; Return goes on from there.
                self.naming = Some(then_close);
                self.start_rename(window, cx);
            }
            Subject::Project { .. } => {
                self.use_in_project(cx);
                if then_close && !self.dirty() {
                    window.remove_window();
                }
            }
        }
    }

    /// The name was given: save the copy under it, unless the themes folder has a theme by that
    /// name, which a sheet asks about first.
    pub(super) fn save_named(&mut self, then_close: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.theme_name(cx) else { return };
        if (self.themes.saved_path)(&name).is_some() {
            self.ask_to_replace(name, then_close, window, cx);
            return;
        }
        self.write_new(name, cx);
        if then_close && !self.dirty() {
            window.remove_window();
        }
    }

    /// Write the theme over its own file. `check`: only if the file is as it was read; a file
    /// that is different asks whether to overwrite it.
    pub(super) fn write_own(
        &mut self,
        path: PathBuf,
        check: bool,
        then_close: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let expected = if check { self.doc.stamp } else { None };
        match (self.themes.overwrite)(&path, &self.doc.file, expected) {
            Ok(saved) => {
                self.saved_as(saved, false, cx);
                if then_close && !self.dirty() {
                    window.remove_window();
                }
            }
            Err(SaveError::ChangedOnDisk) => self.ask_to_overwrite(path, then_close, window, cx),
            Err(SaveError::Other(err)) => {
                self.notice = Some(Notice::Failed(format!("Could not save: {err}").into()));
            }
        }
        cx.notify();
    }

    /// Save the copy as `name`, a theme of its own from now on: it gets its id (once), a file,
    /// and is what the window edits.
    pub(super) fn write_new(&mut self, name: String, cx: &mut Context<Self>) {
        let mut file = self.doc.file.clone();
        file.name = Some(name.clone());
        if file.version.is_some() && file.id.is_none() {
            file.id = Some((self.themes.new_id)(&name));
        }
        match (self.themes.save_new)(&file) {
            Ok(saved) => {
                self.doc.file = file;
                self.saved_as(saved, true, cx);
            }
            Err(err) => self.notice = Some(Notice::Failed(format!("Could not save: {err}").into())),
        }
        cx.notify();
    }

    /// The file as it stands is now what `saved` holds. `new`: it has a file for the first time.
    fn saved_as(&mut self, saved: Saved, new: bool, cx: &mut Context<Self>) {
        let Saved { path, stamp } = saved;
        if new {
            self.doc.subject = Subject::Own { path: path.clone() };
        }
        self.doc.stamp = stamp;
        self.doc.mark_saved();
        let set = self.switched_to(&path, cx);
        let text = if set {
            format!("Saved to {} and set as your theme.", display_path(&path))
        } else {
            format!("Saved to {}.", display_path(&path))
        };
        self.notice = Some(Notice::Saved { path, text: text.into() });
    }

    /// The theme saved at `path` becomes the pick of its own appearance unless it is a pick
    /// already; the preview ends (the theme itself shows now). Whether a pick was set.
    fn switched_to(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        self.revision += 1;
        let appearance = (self.themes.resolve)(&self.doc.file).map_or(Appearance::Dark, |t| t.appearance);
        let (value, selects) = (path.to_string_lossy().into_owned(), self.themes.selects);
        let path = path.to_path_buf();
        let mut set = false;
        SettingsStore::update(cx, |s| {
            let before = (s.theme.clone(), s.theme_light.clone());
            picks::pick_after_save(s, value, appearance, &|setting| selects(setting, &path));
            set = before != (s.theme.clone(), s.theme_light.clone());
        });
        SettingsStore::preview_tokens(cx, None);
        cx.refresh_windows();
        set
    }

    /// Save Copy (⇧⌘S) and Save Copy and Switch: the draft under a free "… copy" name.
    /// Plain, the theme in use and the draft stay as they are; with `switch`, the app starts
    /// using the copy and the window goes on editing it.
    pub(super) fn save_copy(&mut self, switch: bool, cx: &mut Context<Self>) {
        let Some(name) = self.theme_name(cx) else { return };
        let taken = self.themes.saved_path;
        let copy = copy_name(&name, |candidate| taken(candidate).is_some());
        let mut file = ThemeFile { name: Some(copy.clone()), id: None, ..self.doc.file.clone() };
        if file.version.is_some() {
            file.id = Some((self.themes.new_id)(&copy));
        }
        self.notice = Some(match (self.themes.save_new)(&file).map(|saved| saved.path) {
            Ok(path) if switch => {
                self.doc.file = file;
                self.doc.mark_saved();
                self.switched_to(&path, cx);
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
        let json = match export_json(&self.doc.file, self.themes.resolve) {
            Ok(json) => json,
            Err(err) => {
                self.notice = Some(Notice::Failed(format!("Could not export: {err}").into()));
                cx.notify();
                return;
            }
        };
        let file_name = export_file_name(self.doc.file.name.as_deref().unwrap_or("theme"));
        let remembered = store(cx).and_then(|store| store.meta(EXPORT_DIR_KEY).ok().flatten());
        let home = paths::home();
        let dir = export_dir(remembered.as_deref(), home, |dir| dir.is_dir());
        let chosen = cx.prompt_for_new_path(&dir, Some(&file_name));
        self.export = Some(cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = chosen.await else { return };
            // The person's own file: a link there is theirs to keep.
            let notice = match files::write_atomic(&path, json.as_bytes(), LinkPolicy::Follow) {
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
        self.notice = Some(match export_json(&self.doc.file, self.themes.resolve) {
            Ok(json) => {
                crate::platform::clipboard::write(json, cx);
                Notice::Done("Copied the theme's JSON.".into())
            }
            Err(err) => Notice::Failed(format!("Could not copy: {err}").into()),
        });
        cx.notify();
    }

    /// Use in this project…: the theme as the `.wormhole/theme.json` of the project the terminal
    /// in front is in, in the older format. Only a trusted project with its theme switched on gets
    /// one.
    pub(super) fn use_in_project(&mut self, cx: &mut Context<Self>) {
        let folder = match project_for_theme(cx) {
            Ok(folder) => folder,
            Err(why) => {
                self.notice = Some(Notice::Failed(why.into()));
                cx.notify();
                return;
            }
        };
        let project_subject = matches!(&self.doc.subject, Subject::Project { root } if *root == folder.root);
        if matches!(self.doc.subject, Subject::Project { .. }) && !project_subject {
            self.notice = Some(Notice::Failed("The terminal in front is in another project.".into()));
            cx.notify();
            return;
        }
        let written = export_json(&self.doc.file, self.themes.resolve)
            .map_err(|err| err.to_string())
            .and_then(|json| folder.write_theme(&json).map_err(|err| err.to_string()));
        self.notice = Some(match written {
            Ok(path) => {
                // Saved where it applies: for the project's own theme, closing no longer loses it.
                if project_subject {
                    self.doc.mark_saved();
                }
                self.revision += 1;
                let text = format!("Saved to {}: terminals in this project use it.", display_path(&path));
                Notice::Saved { path, text: text.into() }
            }
            Err(err) => Notice::Failed(format!("Could not save to the project: {err}").into()),
        });
        cx.notify();
    }

    /// See [`project_for_theme`].
    pub(super) fn project_for_theme(&self, cx: &App) -> Result<ProjectFolder, &'static str> {
        project_for_theme(cx)
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
        let name = self.doc.name().to_string();
        let count = changes(&self.doc.saved, &self.doc.file);
        let message = format!("Do you want to save the changes to \u{201c}{name}\u{201d}?");
        let detail = format!(
            "{count} {}. Your changes are lost if you don\u{2019}t save them.",
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
                        studio.doc.mark_saved();
                        window.remove_window();
                    }
                    _ => {}
                }
            })
            .ok();
        }));
    }

    /// The theme's file is not what it was when it was read: another program wrote it. A sheet
    /// asks whether to overwrite those changes (the file as it is now is kept as a saved version).
    fn ask_to_overwrite(
        &mut self,
        path: PathBuf,
        then_close: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.asking = true;
        let message = format!("\u{201c}{}\u{201d} was changed by another application.", self.doc.name());
        let detail = "Saving replaces those changes. The file as it is now is kept in the theme\u{2019}s \
                      saved versions.";
        let buttons = [PromptButton::cancel("Cancel"), PromptButton::ok("Overwrite")];
        let answer = window.prompt(PromptLevel::Warning, &message, Some(detail), &buttons, cx);
        self.question = Some(cx.spawn_in(window, async move |this, cx| {
            let overwrite = answer.await.ok() == Some(OVERWRITE);
            this.update_in(cx, |studio, window, cx| {
                studio.asking = false;
                if overwrite {
                    studio.write_own(path, false, then_close, window, cx);
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
                    studio.write_new(name, cx);
                    if then_close && !studio.dirty() {
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
    let home = paths::home();
    wormhole_core::view::sidebar::display_path(path, home.as_deref())
}

/// The project "Use in this project" writes to: that of the terminal in front, when the project is
/// trusted and its theme isn't switched off (`ProjectHub::enabled` once the file exists).
/// Otherwise what stands in the way, in the person's words. Reads the front window's workspace, so
/// it never runs during this window's own draw or update.
pub(super) fn project_for_theme(cx: &App) -> Result<ProjectFolder, &'static str> {
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
    // The trust of the profile the window in front lives in, shown or not.
    project::api::theme_target(&front.profile, &root, cx)
}
