//! Move to Trash: the theme menu takes the file of one of the person's own themes to the Trash,
//! after a sheet that says what changes (the picks that fall back to the defaults, the edits that
//! are dropped). Built-ins, a copy that has no file yet and the project's theme have no such item.
//! The picks and the move itself are `settings::theme_trash`'s, shared with the Settings card.

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_text::keys::settings_appearance_trash_failed;

use super::document::{Document, Subject};
use super::save::Notice;
use super::{StudioWindow, opening, opening_inputs, project_theme};
use crate::settings::settings;
use crate::settings::theme_trash;

impl StudioWindow {
    /// The file Move to Trash would take: the theme being edited when it is one of the person's own.
    fn trash_path(&self) -> Option<PathBuf> {
        match &self.doc.subject {
            Subject::Own { path } => Some(path.clone()),
            _ => None,
        }
    }

    pub(super) fn can_trash(&self) -> bool {
        self.trash_path().is_some()
    }

    /// Whether a pick's value names the file being edited: by the folder's rules, or (a pair that
    /// is not in the folder yet) by its id, as `refresh_own_in_use` does.
    fn names_file(&self, setting: &str, path: &std::path::Path) -> bool {
        let selects = self.themes.selects;
        selects(setting, path)
            || self.doc.file.id.as_deref().is_some_and(|id| {
                setting == id || setting.strip_prefix(id).is_some_and(|rest| rest.starts_with('.'))
            })
    }

    /// Ask, then move the file to the Trash and open the theme now in use.
    pub(super) fn ask_trash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.trash_path() else { return };
        if self.asking {
            return;
        }
        let which = theme_trash::affected(settings(cx), &|v| self.names_file(v, &path));
        let name = self.doc.name().to_string();
        let answer = theme_trash::ask(&name, which, self.dirty(), window, cx);
        self.asking = true;
        self.question = Some(cx.spawn_in(window, async move |this, cx| {
            let confirmed = answer.await;
            this.update_in(cx, |studio, window, cx| {
                studio.asking = false;
                if confirmed {
                    studio.trash_now(&path, &name, window, cx);
                }
            })
            .ok();
        }));
    }

    fn trash_now(&mut self, path: &std::path::Path, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let selects = self.themes.selects;
        let id = self.doc.file.id.clone();
        let names = move |v: &str| {
            selects(v, path)
                || id
                    .as_deref()
                    .is_some_and(|id| v == id || v.strip_prefix(id).is_some_and(|rest| rest.starts_with('.')))
        };
        match theme_trash::trash(cx, path, &names, self.themes.trash) {
            Ok(()) => {
                // The edits went with the file: the window now shows what the app shows.
                let inputs = opening::Inputs { project: project_theme(cx), ..opening_inputs(cx) };
                let doc: Document = opening::open_document(&inputs, &self.themes);
                self.load(doc, window, cx);
            }
            Err(error) => {
                tracing::warn!("theme studio: moving {} to the Trash failed: {error:#}", path.display());
                self.notice = Some(Notice::Failed(settings_appearance_trash_failed(name).into()));
                cx.notify();
            }
        }
    }
}
