//! Opening another theme in the window: from the theme menu, or the copy a Duplicate makes. With
//! edits that aren't saved the same sheet as closing asks first (Save, Cancel, Don't Save); the new
//! theme starts with a history of its own.

use gpui_kit::*;
use wormhole_text::keys::{
    STUDIO_CANCEL, STUDIO_DONT_SAVE, STUDIO_OPEN_FAILED, STUDIO_SAVE, studio_close_message,
    studio_switch_message, studio_unsaved_detail,
};
use wormhole_text::text;

use super::document::history::UndoStack;
use super::document::{Document, Subject};
use super::opening::{Target, open_target};
use super::save::Notice;
use super::theme_list::theme_list;
use super::theme_menu::{self, Facts};
use super::{FIRST_TOKEN, StudioWindow, opening_inputs, project_theme};
use crate::platform;
use crate::settings::SettingsStore;

/// What the unsaved-edits question is asked for.
pub(super) enum Then {
    /// Esc, ⌘W or the red light: close the window.
    Close,
    /// Another theme was chosen. It is read after the question is answered, not before: a Save
    /// writes the file first, and a copy read earlier would be the one from before it. `String` is
    /// what the question calls it.
    Switch(Target, String),
}

impl StudioWindow {
    /// The name in the title row: the theme menu, hanging from the name.
    pub(super) fn open_theme_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let catalog = (self.themes.catalog)();
        let list = theme_list(&catalog, project_theme(cx).is_some());
        let facts = Facts {
            subject: &self.doc.subject,
            name: self.doc.name(),
            dirty: self.dirty(),
            addable: self.missing_appearance().filter(|_| !self.doc.is_newer()),
            versions: matches!(self.doc.subject, Subject::Own { .. }),
        };
        let menu = theme_menu::build(&list, &facts, &cx.weak_entity());
        platform::show_menu(menu, self.name_bounds.get().bottom_left(), window, cx);
    }

    /// A choice of the theme menu.
    pub(super) fn open_target(&mut self, target: &Target, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking {
            return;
        }
        // The theme already being edited: nothing to open, and no question about its edits.
        if matches!((target, &self.doc.subject), (Target::Own(path), Subject::Own { path: own }) if path == own)
        {
            return;
        }
        let Some(doc) = self.read_target(target, cx) else {
            self.notice = Some(Notice::Failed(wormhole_text::text(STUDIO_OPEN_FAILED).into()));
            cx.notify();
            return;
        };
        if self.dirty() {
            // A copy of a theme is called by the theme it is made from: that is what was chosen.
            let called = match &doc.subject {
                Subject::Untitled { from } if !from.is_empty() => from.clone(),
                _ => doc.name().to_string(),
            };
            self.ask_unsaved(Then::Switch(target.clone(), called), window, cx);
        } else {
            self.load(doc, window, cx);
        }
    }

    /// What `target` opens as now; `None` when it can't be read.
    fn read_target(&self, target: &Target, cx: &App) -> Option<Document> {
        let inputs = opening_inputs(cx);
        let inputs = super::opening::Inputs { project: project_theme(cx), ..inputs };
        open_target(target, &inputs, &self.themes)
    }

    /// The window edits `doc` from now on, with a history of its own; the preview follows.
    pub(super) fn load(&mut self, doc: Document, window: &mut Window, cx: &mut Context<Self>) {
        self.history = UndoStack::default();
        self.gesture_timer = None;
        self.typing = Default::default();
        self.naming = None;
        self.renaming = false;
        self.completion = None;
        self.field_error = None;
        self.notice = None;
        let name = doc.name().to_string();
        self.name.update(cx, |input, cx| input.set_value(name, window, cx));
        let resolved = (self.themes.resolve)(&doc.folded()).ok();
        self.doc = doc;
        self.touched();
        SettingsStore::preview_tokens(cx, resolved);
        cx.refresh_windows();
        self.publish_edit_titles(cx);
        let token = self.selected.clone().unwrap_or_else(|| FIRST_TOKEN.to_string());
        self.select_token(&token, window, cx);
    }

    /// The question for `then`: Save (Return), Cancel (Esc) or Don't Save, which loses the edits.
    pub(super) fn ask_unsaved(&mut self, then: Then, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = true;
        let name = self.doc.name().to_string();
        let count = super::document::changes(&self.doc.saved, &self.doc.file);
        let message = match &then {
            Then::Close => studio_close_message(&name),
            // A copy of a theme is called by the theme it is made from: that is what was chosen.
            Then::Switch(_, called) => studio_switch_message(&name, called),
        };
        let detail = studio_unsaved_detail(i64::try_from(count).unwrap_or(i64::MAX));
        let buttons = [
            PromptButton::ok(text(STUDIO_SAVE)),
            PromptButton::cancel(text(STUDIO_CANCEL)),
            PromptButton::new(text(STUDIO_DONT_SAVE)),
        ];
        let answer = window.prompt(PromptLevel::Warning, &message, Some(&detail), &buttons, cx);
        self.question = Some(cx.spawn_in(window, async move |this, cx| {
            // The window going first (the app quitting) answers with nothing: as Cancel.
            let answer = answer.await.ok();
            this.update_in(cx, |studio, window, cx| {
                studio.asking = false;
                match (answer, then) {
                    (Some(super::save::CLOSE_SAVE), Then::Close) => studio.save(true, window, cx),
                    (Some(super::save::CLOSE_SAVE), Then::Switch(target, _)) => {
                        studio.save(false, window, cx);
                        // A save that failed, or waits for a name, leaves things as they are.
                        if !studio.dirty() && !studio.asking && !studio.renaming {
                            match studio.read_target(&target, cx) {
                                Some(next) => studio.load(next, window, cx),
                                None => {
                                    studio.notice =
                                        Some(Notice::Failed(wormhole_text::text(STUDIO_OPEN_FAILED).into()));
                                    cx.notify();
                                }
                            }
                        }
                    }
                    (Some(super::save::CLOSE_DONT_SAVE), Then::Close) => {
                        studio.doc.mark_saved();
                        window.remove_window();
                    }
                    (Some(super::save::CLOSE_DONT_SAVE), Then::Switch(target, _)) => {
                        match studio.read_target(&target, cx) {
                            Some(next) => studio.load(next, window, cx),
                            None => {
                                studio.notice =
                                    Some(Notice::Failed(wormhole_text::text(STUDIO_OPEN_FAILED).into()));
                                cx.notify();
                            }
                        }
                    }
                    _ => {}
                }
            })
            .ok();
        }));
    }
}
