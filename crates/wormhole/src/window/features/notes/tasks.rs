//! What is done to a terminal's next steps: written and edited in place, ticked, moved and deleted.
//! The list and its rules are core's (`view::notes::tasks`); this holds the editor and says when.
//! A task left or made empty is never kept.
use gpui_kit::component::input::{InputEvent, InputState, Position};
use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::view::notes::keys::ItemKey;
use wormhole_core::view::notes::tasks::{Edit, Finished};
use wormhole_core::view::reorder;

use super::items::item_key;
use super::{Notes, TaskEdit};
use crate::window::feature::FeatureEvent;

impl Notes {
    /// Add a finished task at the end (devctl, tests). Blank text adds nothing.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn add_task(&mut self, session: i64, text: &str, cx: &mut Context<Self>) {
        if self.of_mut(session).is_some_and(|of| of.tasks.add(text, false)) {
            self.dirty(cx);
        }
    }

    /// Fill `session` with `open` next steps and `done` done ones (interleaved, one done after every
    /// few open ones), for checking how the list copes with many.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_seed_tasks(
        &mut self,
        session: i64,
        open: usize,
        done: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(of) = self.of_mut(session) else { return };
        let total = open + done;
        for ix in 0..total {
            // Spread evenly: the count of done ones so far grows by one every total / done steps.
            let finish = (ix + 1) * done / total > ix * done / total;
            let n = of.tasks.len() + 1;
            let text = if finish { format!("Finished step {n}") } else { format!("Next step {n}") };
            of.tasks.add(&text, finish);
        }
        of.tasks.collapsed = false;
        self.dirty(cx);
    }

    /// The header's "+": a new, empty task at the end, with the cursor in it.
    pub(crate) fn new_task(&mut self, session: i64, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.of.get(&session).map_or(0, |of| of.tasks.len());
        self.edit_task(session, end, true, window, cx);
    }

    /// Edit task `ix` in place, or with `new`, start writing a task that goes in at `ix`. Whatever was
    /// being edited is finished first.
    pub(crate) fn edit_task(
        &mut self,
        session: i64,
        ix: usize,
        new: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(of) = self.of.get(&session) else { return };
        let mut ix = ix;
        let current = of.edit.as_ref().map(|edit| edit.input.clone());
        if let Some(current) = current {
            // Finishing can insert or delete a task: keep `ix` on the one that was meant.
            if let Some(finished) = self.finish_task_edit(session, &current, cx) {
                match finished.retarget(ix, new) {
                    Some(retargeted) => ix = retargeted,
                    // The task asked for was the one just emptied, and is gone.
                    None => return,
                }
            }
        }
        let Some(of) = self.of_mut(session) else { return };
        if !new && ix >= of.tasks.len() {
            return;
        }
        let ix = ix.min(of.tasks.len());
        let text =
            if new { SharedString::default() } else { SharedString::from(of.tasks.items()[ix].text.clone()) };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Next step").default_value(text));
        let subscription =
            cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { secondary: false, .. } => {
                    this.task_enter(session, input, window, cx)
                }
                // Like ⌘↵ in a note. The terminal takes the focus, which finishes the edit.
                InputEvent::PressEnter { secondary: true, shift } => {
                    let text = input.read(cx).value().to_string();
                    this.send_text(session, text, *shift, cx);
                }
                InputEvent::Blur => {
                    this.finish_task_edit(session, input, cx);
                }
                _ => {}
            });
        let Some(of) = self.of_mut(session) else { return };
        of.edit =
            Some(TaskEdit { edit: Edit { ix, new }, input: input.clone(), _subscription: subscription });
        of.tasks.collapsed = false;
        // The panel shows the Notes tab for the step being written.
        cx.emit(FeatureEvent::ShowTab(TabId::Notes));
        cx.notify();
        // The editor's row only exists once the list has been rebuilt.
        cx.defer_in(window, move |_, window, cx| {
            input.update(cx, |input, cx| {
                // At the end of the text, where clicking to add to it expects it.
                let end = input.value().chars().count() as u32;
                input.set_cursor_position(Position::new(0, end), window, cx);
            });
        });
    }

    /// End the edit `input` belongs to, if it is still going: save the text, delete a task edited down
    /// to nothing, drop a new one left empty.
    fn finish_task_edit(
        &mut self,
        session: i64,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Option<Finished> {
        let of = self.of_mut(session)?;
        if of.edit.as_ref().is_none_or(|edit| &edit.input != input) {
            return None;
        }
        let edit = of.edit.take()?;
        let finished = of.tasks.finish(edit.edit, &input.read(cx).value());
        self.dirty(cx);
        Some(finished)
    }

    /// Enter in a task: keep it and write the next one right below, like a checklist. Enter in an
    /// empty one is done writing.
    fn task_enter(
        &mut self,
        session: i64,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.finish_task_edit(session, input, cx) {
            Some(Finished::Kept(ix) | Finished::Inserted(ix)) => {
                self.edit_task(session, ix + 1, true, window, cx)
            }
            Some(Finished::Removed(_) | Finished::Dropped) => cx.emit(FeatureEvent::FocusTerminal),
            None => {}
        }
    }

    /// The keys a task's editor shares with a note's ([`ItemKey`]); whether it handled the key.
    pub(crate) fn task_key(
        &mut self,
        session: i64,
        input: &Entity<InputState>,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(of) = self.of.get(&session) else { return false };
        let Some(edit) = of.edit.as_ref().filter(|edit| &edit.input == input) else { return false };
        let (Edit { ix, new }, len) = (edit.edit, of.tasks.len());
        match item_key(&event.keystroke, input.read(cx).value().is_empty()) {
            // The terminal takes the focus, which finishes the edit.
            Some(ItemKey::Leave) => cx.emit(FeatureEvent::FocusTerminal),
            Some(ItemKey::DeleteEmpty) => {
                self.finish_task_edit(session, input, cx);
                match ix.checked_sub(1) {
                    Some(above) => self.edit_task(session, above, false, window, cx),
                    None => cx.emit(FeatureEvent::FocusTerminal),
                }
            }
            Some(ItemKey::Move(delta)) if !new => {
                if let Some(to) = reorder::step(ix, delta, len) {
                    self.move_task(session, ix, to, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// Move a task to another place in the list (drag and drop, ⌥⌘↑/↓).
    pub(crate) fn move_task(&mut self, session: i64, from: usize, to: usize, cx: &mut Context<Self>) {
        let Some(of) = self.of_mut(session) else { return };
        if !of.tasks.move_item(from, to) {
            return;
        }
        if let Some(edit) = of.edit.as_mut() {
            edit.edit = edit.edit.after_move(from, to);
        }
        self.dirty(cx);
    }

    pub(crate) fn toggle_task(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        if self.of_mut(session).is_some_and(|of| of.tasks.toggle(ix)) {
            self.dirty(cx);
        }
    }

    pub(crate) fn toggle_tasks_section(&mut self, session: i64, cx: &mut Context<Self>) {
        if let Some(of) = self.of_mut(session) {
            of.tasks.collapsed = !of.tasks.collapsed;
            self.dirty(cx);
        }
    }

    pub(crate) fn delete_task(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        let Some(of) = self.of_mut(session) else { return };
        if !of.tasks.delete(ix) {
            return;
        }
        // Keep an edit in progress on its task (or drop it with its task).
        if let Some(edit) = of.edit.take() {
            of.edit = edit.edit.after_delete(ix).map(|kept| TaskEdit { edit: kept, ..edit });
        }
        self.dirty(cx);
    }

    /// Throw away the new task being written (its hover ✕).
    pub(crate) fn discard_task_draft(&mut self, session: i64, cx: &mut Context<Self>) {
        let Some(of) = self.of_mut(session) else { return };
        if of.edit.as_ref().is_some_and(|edit| edit.edit.new) {
            of.edit = None;
            cx.notify();
        }
    }
}
