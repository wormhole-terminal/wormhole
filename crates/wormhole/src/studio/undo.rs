//! Undo and redo in the Studio window: every change to the file keeps the file as it was (a drag
//! of a colour is one step), ⌘Z and ⇧⌘Z walk back and forth, and the Edit menu says what they
//! would undo while this window is in front. A text field being typed in undoes its typing first.

use std::time::Duration;

use gpui_kit::component::input;
use gpui_kit::*;
use wormhole_text::keys::{STUDIO_UNDO_RENAME, STUDIO_UNDO_REVERT, studio_undo_change, studio_undo_reset};

use super::StudioWindow;
use super::document::history::{Step, StepLabel};
use crate::settings::SettingsStore;
use crate::window::menu_state::{EditTitles, set_edit_titles};

/// How long a drag of a colour, or a run of key presses on a slider, may pause and still be one step.
const GESTURE_QUIET: Duration = Duration::from_millis(600);

/// Which text fields hold typing that is not a step yet: ⌘Z undoes that first.
#[derive(Default)]
pub(super) struct Typing {
    pub(super) expression: bool,
    pub(super) name: bool,
    pub(super) filter: bool,
}

/// What the Edit menu says after Undo or Redo.
fn words(label: &StepLabel) -> String {
    match label {
        StepLabel::Token(token) => studio_undo_change(token),
        StepLabel::Reset(token) => studio_undo_reset(token),
        StepLabel::Rename => wormhole_text::text(STUDIO_UNDO_RENAME).to_string(),
        StepLabel::Revert => wormhole_text::text(STUDIO_UNDO_REVERT).to_string(),
    }
}

impl StudioWindow {
    /// A change to the file is about to be made: keep the file as it is. `gesture`: the change is
    /// one of a run (a drag), which is one step.
    pub(super) fn record_step(&mut self, label: StepLabel, gesture: bool, cx: &mut Context<Self>) {
        let before = Step { label, file: self.doc.file.clone(), selected: self.selected.clone() };
        self.history.record(before, gesture);
        self.gesture_timer = gesture.then(|| {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(GESTURE_QUIET).await;
                this.update(cx, |this, cx| this.end_gesture(cx)).ok();
            })
        });
    }

    /// The drag is over: the next change is a step of its own, and the menu says what it undoes.
    pub(super) fn end_gesture(&mut self, cx: &mut Context<Self>) {
        self.history.end_gesture();
        self.gesture_timer = None;
        self.publish_edit_titles(cx);
    }

    pub(super) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(step) = self.history.undo(&self.doc.file, self.selected.clone()) else { return };
        self.restore(step, window, cx);
    }

    pub(super) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(step) = self.history.redo(&self.doc.file, self.selected.clone()) else { return };
        self.restore(step, window, cx);
    }

    /// Put a step's file back: the preview, the name and the editor follow, and the editor jumps to
    /// the token that was selected when the change was made.
    fn restore(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(resolved) = (self.themes.resolve)(&step.file) else { return };
        self.doc.file = step.file;
        self.revision += 1;
        self.field_error = None;
        self.notice = None;
        self.gesture_timer = None;
        let name = self.doc.name().to_string();
        self.name.update(cx, |input, cx| input.set_value(name, window, cx));
        SettingsStore::preview_tokens(cx, Some(resolved));
        cx.refresh_windows();
        match step.selected.or_else(|| self.selected.clone()) {
            Some(token) => self.select_token(&token, window, cx),
            None => cx.notify(),
        }
        self.publish_edit_titles(cx);
    }

    /// ⌘Z or ⇧⌘Z (`input::Undo`, `input::Redo`, caught before the focused field sees them): a
    /// field with typing in it keeps its own undo, everything else undoes the file.
    pub(super) fn route_undo(&mut self, redo: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.focused_field_has_typing(window, cx) {
            cx.propagate();
            return;
        }
        if redo { self.redo(window, cx) } else { self.undo(window, cx) }
    }

    fn focused_field_has_typing(&self, window: &Window, cx: &App) -> bool {
        let focused = |input: &Entity<input::InputState>| input.read(cx).focus_handle(cx).is_focused(window);
        let list = self.list.read(cx);
        (self.typing.expression && focused(&self.expression))
            || (self.typing.name && focused(&self.name))
            || (self.typing.filter && focused(&list.filter))
    }

    /// Tell the Edit menu what Undo and Redo would do here, while this is the window in front.
    pub(super) fn publish_edit_titles(&self, cx: &mut Context<Self>) {
        let titles = EditTitles {
            undo: self.history.undo_label().map(words),
            redo: self.history.redo_label().map(words),
        };
        let active = self.active;
        cx.defer(move |cx| set_edit_titles(if active { titles } else { EditTitles::default() }, cx));
    }
}
