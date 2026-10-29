//! Opening another theme in the window: from the theme menu, or the copy a Duplicate makes. With
//! edits that aren't saved the same sheet as closing asks first (Save, Cancel, Don't Save); the new
//! theme starts with a history of its own.

use gpui_kit::*;
use wormhole_text::keys::{STUDIO_OPEN_FAILED, studio_switch_message};

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
    /// Another theme was chosen.
    Switch(Box<Document>),
}

impl StudioWindow {
    /// The name in the title row: the theme menu, hanging from the name.
    pub(super) fn open_theme_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let catalog = (self.themes.catalog)();
        let list = theme_list(&catalog, project_theme(cx).is_some());
        let facts = Facts { subject: &self.doc.subject, name: self.doc.name(), dirty: self.dirty() };
        let menu = theme_menu::build(&list, &facts, &cx.weak_entity());
        platform::show_menu(menu, self.name_bounds.get().bottom_left(), window, cx);
    }

    /// A choice of the theme menu.
    pub(super) fn open_target(&mut self, target: &Target, window: &mut Window, cx: &mut Context<Self>) {
        let inputs = opening_inputs(cx);
        let inputs = super::opening::Inputs { project: project_theme(cx), ..inputs };
        match open_target(target, &inputs, &self.themes) {
            Some(doc) => self.switch_to(doc, window, cx),
            None => {
                self.notice = Some(Notice::Failed(wormhole_text::text(STUDIO_OPEN_FAILED).into()));
                cx.notify();
            }
        }
    }

    /// Open `doc` in the window: at once when nothing would be lost, otherwise after the question.
    pub(super) fn switch_to(&mut self, doc: Document, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking {
            return;
        }
        if self.dirty() {
            self.ask_unsaved(Then::Switch(Box::new(doc)), window, cx);
        } else {
            self.load(doc, window, cx);
        }
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
        self.revision += 1;
        let name = doc.name().to_string();
        self.name.update(cx, |input, cx| input.set_value(name, window, cx));
        let resolved = (self.themes.resolve)(&doc.file).ok();
        self.doc = doc;
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
            Then::Close => format!("Do you want to save the changes to \u{201c}{name}\u{201d}?"),
            // A copy of a theme is called by the theme it is made from: that is what was chosen.
            Then::Switch(next) => match &next.subject {
                Subject::Untitled { from } => studio_switch_message(&name, from),
                _ => studio_switch_message(&name, next.name()),
            },
        };
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
                match (answer, then) {
                    (Some(super::save::CLOSE_SAVE), Then::Close) => studio.save(true, window, cx),
                    (Some(super::save::CLOSE_SAVE), Then::Switch(next)) => {
                        studio.save(false, window, cx);
                        // A save that failed, or waits for a name, leaves things as they are.
                        if !studio.dirty() && !studio.asking && !studio.renaming {
                            studio.load(*next, window, cx);
                        }
                    }
                    (Some(super::save::CLOSE_DONT_SAVE), Then::Close) => {
                        studio.doc.mark_saved();
                        window.remove_window();
                    }
                    (Some(super::save::CLOSE_DONT_SAVE), Then::Switch(next)) => {
                        studio.load(*next, window, cx)
                    }
                    _ => {}
                }
            })
            .ok();
        }));
    }
}
