//! Saving the draft: Save theme (into the themes folder, then switched to), Save Copy
//! and Save Copy and Switch, Export JSON, Copy JSON and Use in this project; the notice each
//! leaves under the title row; and the two questions, sheets on the window as macOS asks them:
//! a name that is taken, and closing with edits.

use std::path::{Path, PathBuf};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::BannerKind;
use wormhole_core::platform::paths;
use wormhole_core::project::ProjectFolder;
use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::tokens::{Appearance, ThemeFile};
use wormhole_text::keys::*;
use wormhole_text::text;

use super::document::{Document, Subject};
use super::draft::{copy_name, export_dir, export_file_name, export_json};
use super::switch::Then;
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
pub(super) const CLOSE_SAVE: usize = 0;
pub(super) const CLOSE_DONT_SAVE: usize = 2;
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
    /// A value moved between both appearances and one and a clash was settled: says which value
    /// stayed. Undo takes it back.
    Moved(SharedString),
    Failed(SharedString),
}

impl StudioWindow {
    /// The name the draft is saved under, or why there is none.
    pub(super) fn theme_name(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let name = self.doc.name().trim().to_string();
        if name.is_empty() {
            self.notice = Some(Notice::Failed(text(STUDIO_NAME_FIRST).into()));
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
        if (self.themes.version_named)(&name) {
            self.notice = Some(Notice::Failed(studio_name_is_a_version(&name).into()));
            cx.notify();
            return;
        }
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
        // A file from a newer Wormhole holds values this one dropped on reading: writing it back
        // would lose them for good. Duplicate still saves a copy.
        if self.doc.file.read.newer {
            self.notice = Some(Notice::Failed(wormhole_text::text(STUDIO_NEWER_FILE).into()));
            cx.notify();
            return;
        }
        let expected = if check { self.doc.stamp } else { None };
        let to_write = match self.file_to_write(&path) {
            Ok(to_write) => to_write,
            Err(why) => {
                self.notice = Some(Notice::Failed(why.into()));
                cx.notify();
                return;
            }
        };
        let was = self.doc.saved.clone();
        match (self.themes.overwrite)(&path, &to_write.file, expected) {
            Ok(saved) => {
                // A pick that named the theme by a name it no longer has (it was renamed) names its
                // file: nothing is lost.
                let old_names: Vec<String> = was.name.iter().cloned().collect();
                // Otherwise a pick stays as it is, a converted theme's too: an older release can't
                // read an id, and the file keeps its name and `name`, which still select it.
                if was.name != to_write.file.name {
                    follow_name(&old_names, &saved.path.to_string_lossy(), cx);
                }
                let converted = to_write.converted;
                self.doc.file = to_write.file;
                self.saved_as(saved, false, converted, cx);
                if then_close && !self.dirty() {
                    window.remove_window();
                }
            }
            Err(SaveError::ChangedOnDisk) => self.ask_to_overwrite(path, then_close, window, cx),
            Err(SaveError::Other(err)) => {
                self.notice = Some(Notice::Failed(studio_save_failed(&err.to_string()).into()));
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
            // Replacing a theme's file keeps the id that file has: it does not count against itself.
            let replaced = (self.themes.saved_path)(&name);
            file.id = Some((self.themes.new_id)(&name, replaced.as_deref()));
        }
        match (self.themes.save_new)(&file) {
            Ok(saved) => {
                self.doc.file = file;
                self.saved_as(saved, true, false, cx);
            }
            Err(err) => self.notice = Some(Notice::Failed(studio_save_failed(&err.to_string()).into())),
        }
        cx.notify();
    }

    /// The file as it stands is now what `saved` holds. `new`: it has a file for the first time.
    fn saved_as(&mut self, saved: Saved, new: bool, converted: bool, cx: &mut Context<Self>) {
        let Saved { path, stamp, lost_comments } = saved;
        if new {
            self.doc.subject = Subject::Own { path: path.clone() };
        }
        self.doc.stamp = stamp;
        self.doc.mark_saved();
        let set = self.switched_to(&path, cx);
        let mut text =
            if set { studio_saved_set(&display_path(&path)) } else { studio_saved(&display_path(&path)) };
        // Older releases of Wormhole can't read a file in the new format, and a rewritten file has
        // no comments: said once, on the save that does it.
        if converted {
            text = format!("{text} {}", wormhole_text::text(wormhole_text::keys::STUDIO_SAVED_CONVERTED));
        }
        if lost_comments {
            text = format!("{text} {}", wormhole_text::text(wormhole_text::keys::STUDIO_SAVED_COMMENTS));
        }
        self.notice = Some(Notice::Saved { path, text: text.into() });
    }

    /// The theme saved at `path` becomes the pick of its own appearance unless it is a pick
    /// already; the preview ends (the theme itself shows now). Whether a pick was set.
    fn switched_to(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        self.touched();
        let appearance = (self.themes.resolve)(&self.doc.folded()).map_or(Appearance::Dark, |t| t.appearance);
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

    /// Duplicate (⇧⌘S): the theme as it stands, edits and all, under a free "… copy" name in a file
    /// of its own, which the window goes on editing. The original stays as it was saved. Picks do not
    /// change.
    pub(super) fn duplicate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking {
            return;
        }
        let Some(name) = self.theme_name(cx) else { return };
        let taken = self.themes.saved_path;
        let copy = copy_name(&name, |candidate| taken(candidate).is_some());
        // The copy is written by this version: it is no longer "from a newer Wormhole".
        let mut file = ThemeFile {
            name: Some(copy.clone()),
            id: None,
            read: Default::default(),
            ..self.doc.file.clone()
        };
        if file.version.is_some() {
            file.id = Some((self.themes.new_id)(&copy, None));
        }
        match (self.themes.save_new)(&file) {
            Ok(Saved { path, stamp, .. }) => {
                let text = studio_copy_saved(&copy, &display_path(&path));
                let doc = Document::new(file, Subject::Own { path: path.clone() }, stamp);
                self.load(doc, window, cx);
                self.notice = Some(Notice::Saved { path, text: text.into() });
            }
            Err(err) => self.notice = Some(Notice::Failed(studio_copy_failed(&err.to_string()).into())),
        }
        cx.notify();
    }

    /// Export JSON…: the draft as a theme file, where the person picks. The dialog asks every
    /// time and starts where the last export went, the home folder the first time.
    pub(super) fn export(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let json = match export_json(&self.doc.folded(), self.themes.resolve) {
            Ok(json) => json,
            Err(err) => {
                self.notice = Some(Notice::Failed(studio_export_failed(&err.to_string()).into()));
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
                    let text = studio_exported(&display_path(&path));
                    Notice::Saved { path, text: text.into() }
                }
                Err(err) => Notice::Failed(studio_export_failed(&err.to_string()).into()),
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
        self.notice = Some(match export_json(&self.doc.folded(), self.themes.resolve) {
            Ok(json) => {
                crate::platform::clipboard::write(json, cx);
                Notice::Done(text(STUDIO_COPIED_JSON).into())
            }
            Err(err) => Notice::Failed(studio_copy_json_failed(&err.to_string()).into()),
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
            self.notice = Some(Notice::Failed(text(STUDIO_OTHER_PROJECT).into()));
            cx.notify();
            return;
        }
        let written = export_json(&self.doc.folded(), self.themes.resolve)
            .map_err(|err| err.to_string())
            .and_then(|json| folder.write_theme(&json).map_err(|err| err.to_string()));
        self.notice = Some(match written {
            Ok(path) => {
                // Saved where it applies: for the project's own theme, closing no longer loses it.
                if project_subject {
                    self.doc.mark_saved();
                }
                self.touched();
                let text = studio_project_saved(&display_path(&path));
                Notice::Saved { path, text: text.into() }
            }
            Err(err) => Notice::Failed(studio_project_failed(&err.to_string()).into()),
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
            Notice::Saved { path, text } => style::banner(ui, BannerKind::Success, false, text)
                .child(
                    Button::new("studio-reveal")
                        .ghost()
                        .small()
                        .label(wormhole_text::text(STUDIO_REVEAL))
                        .on_click(move |_, _, cx| cx.reveal_path(&path)),
                )
                .when(self.can_follow_system(cx), |banner| {
                    banner.child(
                        Button::new("studio-use-with-match-system")
                            .ghost()
                            .small()
                            .label(wormhole_text::text(wormhole_text::keys::STUDIO_USE_WITH_MATCH_SYSTEM))
                            .on_click(cx.listener(|this, _, _, cx| this.follow_system(cx))),
                    )
                }),
            Notice::Done(text) => style::banner(ui, BannerKind::Success, false, text),
            Notice::Moved(text) => style::banner(ui, BannerKind::Info, false, text).child(
                Button::new("studio-moved-undo")
                    .ghost()
                    .small()
                    .label(wormhole_text::text(wormhole_text::keys::STUDIO_MOVED_UNDO))
                    .on_click(cx.listener(|this, _, window, cx| this.undo(window, cx))),
            ),
            Notice::Failed(text) => {
                style::banner(ui, BannerKind::Danger, false, text).when(self.doc.is_newer(), |banner| {
                    banner.child(
                        Button::new("studio-newer-duplicate")
                            .ghost()
                            .small()
                            .label(wormhole_text::text(wormhole_text::keys::STUDIO_NEWER_DUPLICATE))
                            .on_click(cx.listener(|this, _, window, cx| this.duplicate(window, cx))),
                    )
                })
            }
        };
        Some(div().flex_none().px_4().py_3().bg(background).rule_bottom(ui).child(banner).into_any_element())
    }

    /// Esc, ⌘W or the red light with edits: a sheet on the window, as macOS asks it.
    /// Return saves, Esc cancels, and Don't Save closes with the edits lost. The sheet
    /// is modal for the window, so nothing else in the Studio runs until it is answered.
    pub(super) fn ask_before_closing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ask_unsaved(Then::Close, window, cx);
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
        let message = studio_changed_message(self.doc.name());
        let detail = text(STUDIO_CHANGED_DETAIL);
        let buttons = [PromptButton::cancel(text(STUDIO_CANCEL)), PromptButton::ok(text(STUDIO_OVERWRITE))];
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
        let message = studio_exists_message(&name);
        let detail = text(STUDIO_EXISTS_DETAIL);
        let buttons = [PromptButton::ok(text(STUDIO_REPLACE)), PromptButton::cancel(text(STUDIO_CANCEL))];
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
    let no_terminal = text(STUDIO_NO_TERMINAL);
    if cx.try_global::<AppState>().is_none() {
        return Err(no_terminal);
    }
    let front = AppState::front_window(cx).ok_or(no_terminal)?;
    let root = front
        .workspace
        .read_with(cx, project::api::active_root)
        .ok()
        .flatten()
        .ok_or(text(STUDIO_NO_PROJECT_FOLDER))?;
    // The trust of the profile the window in front lives in, shown or not.
    project::api::theme_target(&front.profile, &root, cx)
}

/// A pick that names a theme by one of `old` (a name or a file stem) is changed to `value` (its id,
/// or its file): only those picks. Picks that name a file or an id are not touched.
fn follow_name(old: &[String], value: &str, cx: &mut App) {
    SettingsStore::update(cx, |s| {
        for pick in [&mut s.theme, &mut s.theme_light] {
            if old.iter().any(|name| pick.eq_ignore_ascii_case(name)) {
                *pick = value.to_string();
            }
        }
    });
}
