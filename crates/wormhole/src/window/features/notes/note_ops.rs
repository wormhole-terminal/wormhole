//! What is done to a terminal's notes: made, shown, written in, folded, moved, deleted, sent, and
//! thrown away when left empty.
use std::cmp::Ordering;

use gpui_kit::component::input::{InputEvent, Position, TextareaState};
use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::view::notes::keys::ItemKey;
use wormhole_core::view::reorder;

use super::items::item_key;
use super::view::wrapped_placeholder;
use super::{NOTE_ROWS_MAX, NOTE_ROWS_MIN, Note, Notes, is_blank_note, on, selection_or_paragraph};
use crate::window::feature::FeatureEvent;

impl Notes {
    /// Makes a note of terminal `session` with `value` in it, at the end of its list. `panel` is the
    /// panel's width, which the hint of an empty note is broken to. Its index.
    pub(in crate::window) fn add_note(
        &mut self,
        session: i64,
        value: Option<String>,
        collapsed: bool,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        self.panel = panel;
        let hint = wrapped_placeholder(panel, window, cx);
        let state = cx.new(|cx| {
            let state =
                TextareaState::new(window, cx).auto_grow(NOTE_ROWS_MIN, NOTE_ROWS_MAX).placeholder(hint);
            match value {
                Some(value) => state.default_value(value),
                None => state,
            }
        });
        let subscription =
            cx.subscribe_in(&state, window, move |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { secondary: true, shift } => {
                    let text = selection_or_paragraph(state.read(cx));
                    this.send_text(session, text, *shift, cx);
                }
                InputEvent::Change => {
                    if let Some(of) = this.of_mut(session) {
                        of.recount(cx);
                    }
                    this.dirty(cx);
                }
                InputEvent::Focus => {
                    // The panel may have another width than when the note was made.
                    let hint = wrapped_placeholder(this.panel, window, cx);
                    state.update(cx, |state, cx| state.set_placeholder(hint, window, cx));
                    this.focused = this
                        .of
                        .get(&session)
                        .and_then(|of| of.notes.iter().position(|note| &note.state == state))
                        .map(|ix| (session, ix));
                    this.editing = Some(state.entity_id());
                    cx.emit(FeatureEvent::Editing(true));
                    cx.notify();
                }
                // A note left empty is thrown away rather than kept as an empty entry.
                InputEvent::Blur => {
                    if this.editing == Some(state.entity_id()) {
                        this.editing = None;
                        cx.emit(FeatureEvent::Editing(false));
                        cx.notify();
                    }
                    this.prune_blank_notes(window, cx);
                }
                _ => {}
            });
        let Some(of) = self.of_mut(session) else { return 0 };
        of.notes.push(Note { state, collapsed, show_all: false, _subscription: subscription });
        of.recount(cx);
        let ix = of.notes.len() - 1;
        self.dirty(cx);
        ix
    }

    /// Replaces the text of note `ix` of terminal `session`; whether there was such a note. Setting
    /// an editor's value sends no `InputEvent`, so what a typed change would have told is told here: the
    /// count of notes with something in them (the row, VoiceOver and the close question read it) and
    /// the save. Every place that sets a note's text from outside goes through this.
    pub(super) fn set_note_text(
        &mut self,
        session: i64,
        ix: usize,
        text: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.of.get(&session).and_then(|of| of.notes.get(ix)).map(|n| n.state.clone())
        else {
            return false;
        };
        state.update(cx, |state, cx| state.set_value(text, window, cx));
        if let Some(of) = self.of_mut(session) {
            of.recount(cx);
        }
        self.dirty(cx);
        true
    }

    /// Make the Notes tab what the panel shows, open and with `session`'s section unfolded, so a note
    /// added or focused next is on screen. (With Changes, History or Project showing, or the section
    /// folded, there is nothing to type into.)
    pub(super) fn reveal(&mut self, session: i64, cx: &mut Context<Self>) {
        cx.emit(FeatureEvent::ShowTab(TabId::Notes));
        if let Some(of) = self.of_mut(session) {
            of.notes_collapsed = false;
        }
    }

    pub(in crate::window) fn new_note(
        &mut self,
        session: i64,
        text: Option<String>,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !on(cx) || !self.of.contains_key(&session) {
            return;
        }
        self.reveal(session, cx);
        let ix = self.add_note(session, text, false, panel, window, cx);
        self.focused = Some((session, ix));
        let state = self.of[&session].notes[ix].state.clone();
        state.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Put note `ix` of `session` on screen and give it the keyboard: the Notes tab open, the section
    /// and the note unfolded. `to_end` also puts the cursor at the end of its text (clicking its
    /// title); otherwise it stays where it was (⇧⌘E).
    pub(in crate::window) fn show_note(
        &mut self,
        session: i64,
        ix: usize,
        to_end: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.of.get(&session).is_none_or(|of| ix >= of.notes.len()) {
            return;
        }
        self.reveal(session, cx);
        let note = &mut self.of_mut(session).expect("checked above").notes[ix];
        note.collapsed = false;
        let state = note.state.clone();
        self.focused = Some((session, ix));
        // The editor only exists once the list has been rebuilt with the note on screen.
        cx.defer_in(window, move |_, window, cx| {
            state.update(cx, |state, cx| {
                if !to_end {
                    state.focus(window, cx);
                    return;
                }
                let text = state.value();
                let line = text.matches('\n').count() as u32;
                let column = text.rsplit('\n').next().map_or(0, |last| last.chars().count()) as u32;
                // Also focuses it.
                state.set_cursor_position(Position::new(line, column), window, cx);
            });
        });
    }

    /// Clicking a note's title: open it and put the cursor at the end of its text.
    pub(in crate::window) fn open_note(
        &mut self,
        session: i64,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_note(session, ix, true, window, cx);
    }

    /// Throw away every note with nothing in it that isn't being written right now. Runs when a note
    /// loses focus, and on the tick for one that never got it.
    pub(in crate::window) fn prune_blank_notes(&mut self, window: &Window, cx: &mut Context<Self>) {
        let mut changed = false;
        let focused = self.focused;
        let mut focused_now = focused;
        for (&session, of) in &mut self.of {
            let mut ix = 0;
            while ix < of.notes.len() {
                let state = of.notes[ix].state.read(cx);
                if !(is_blank_note(state) && !state.focus_handle(cx).is_focused(window)) {
                    ix += 1;
                    continue;
                }
                of.notes.remove(ix);
                if let Some((id, f)) = focused_now.filter(|&(id, _)| id == session) {
                    focused_now = match f.cmp(&ix) {
                        Ordering::Less => Some((id, f)),
                        Ordering::Equal => None,
                        Ordering::Greater => Some((id, f - 1)),
                    };
                }
                changed = true;
            }
            of.notes_collapsed &= !of.notes.is_empty();
        }
        self.focused = focused_now;
        if changed {
            for of in self.of.values_mut() {
                of.recount(cx);
            }
            self.dirty(cx);
        }
    }

    /// The keys a note's text shares with a task's editor ([`ItemKey`]); whether it handled the key.
    pub(in crate::window) fn note_key(
        &mut self,
        session: i64,
        state: &Entity<TextareaState>,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(of) = self.of.get(&session) else { return false };
        let Some(ix) = of.notes.iter().position(|note| &note.state == state) else { return false };
        let len = of.notes.len();
        match item_key(&event.keystroke, state.read(cx).value().is_empty()) {
            // Leaving it empty throws it away (see `prune_blank_notes`).
            Some(ItemKey::Leave) => cx.emit(FeatureEvent::FocusTerminal),
            Some(ItemKey::DeleteEmpty) => {
                self.delete_note(session, ix, cx);
                match ix.checked_sub(1) {
                    Some(above) => self.open_note(session, above, window, cx),
                    None => cx.emit(FeatureEvent::FocusTerminal),
                }
            }
            Some(ItemKey::Move(delta)) => {
                if let Some(to) = reorder::step(ix, delta, len) {
                    self.move_note(session, ix, to, cx);
                }
            }
            None => return false,
        }
        true
    }

    /// Move a note to another place in the list (drag and drop, ⌥⌘↑/↓).
    pub(in crate::window) fn move_note(
        &mut self,
        session: i64,
        from: usize,
        to: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(of) = self.of_mut(session) else { return };
        if !reorder::move_item(&mut of.notes, from, to) {
            return;
        }
        if let Some((id, ix)) = self.focused.filter(|&(id, _)| id == session) {
            self.focused = Some((id, reorder::follow(ix, from, to)));
        }
        self.dirty(cx);
    }

    pub(in crate::window) fn delete_note(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        let Some(of) = self.of_mut(session) else { return };
        if ix >= of.notes.len() {
            return;
        }
        of.notes.remove(ix);
        // An empty section has no chevron to open it with.
        of.notes_collapsed &= !of.notes.is_empty();
        of.recount(cx);
        self.focused = self
            .focused
            .filter(|&(id, f)| id != session || f != ix)
            .map(|(id, f)| if id == session && f > ix { (id, f - 1) } else { (id, f) });
        self.dirty(cx);
    }

    pub(in crate::window) fn toggle_notes_section(&mut self, session: i64, cx: &mut Context<Self>) {
        let Some(of) = self.of_mut(session) else { return };
        of.notes_collapsed = !of.notes_collapsed;
        // A note that was focused is about to disappear from the list.
        self.focused = None;
        self.dirty(cx);
    }

    pub(in crate::window) fn toggle_note(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        if let Some(note) = self.of_mut(session).and_then(|of| of.notes.get_mut(ix)) {
            note.collapsed = !note.collapsed;
            // Opened again, a long note is cut at the cap again.
            note.show_all = false;
            self.dirty(cx);
        }
    }

    /// "Show all · N lines" under a long open note.
    pub(in crate::window) fn show_all_of_note(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        if let Some(note) = self.of_mut(session).and_then(|of| of.notes.get_mut(ix)) {
            note.show_all = true;
            cx.notify();
        }
    }

    /// "N done" under the open next steps: show or fold the done ones.
    pub(in crate::window) fn toggle_done_steps(&mut self, session: i64, cx: &mut Context<Self>) {
        if let Some(of) = self.of_mut(session) {
            of.list.done_open = !of.list.done_open;
            cx.notify();
        }
    }

    /// Paste `text` into terminal `session`, and press Enter after it with `submit`. The window does
    /// it (the terminal is its); the keyboard goes to the terminal.
    pub(in crate::window) fn send_text(
        &mut self,
        session: i64,
        text: String,
        submit: bool,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        cx.emit(FeatureEvent::SendText { session, text: text.to_string().into(), submit });
    }

    /// Move the prompt typed in the terminal (or the selection) into a new note.
    pub(in crate::window) fn stash_prompt(
        &mut self,
        session: i64,
        terminal: &Entity<crate::terminal_view::TerminalView>,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !on(cx) || !self.of.contains_key(&session) {
            return;
        }
        let Some(text) = terminal.read(cx).grab_prompt() else { return };
        self.reveal(session, cx);
        self.add_note(session, Some(text), false, panel, window, cx);
        // Keep typing in the terminal (e.g. to answer the agent's question).
        cx.emit(FeatureEvent::FocusTerminal);
    }

    /// ⇧⌘E from the terminal: into the note being worked on, else the first, else a new one. `true`:
    /// the notes took the keys (they always do while they are on).
    pub(in crate::window) fn focus_in(
        &mut self,
        session: i64,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !on(cx) {
            return false;
        }
        let Some(of) = self.of.get(&session) else { return false };
        let target = self
            .focused
            .filter(|&(id, ix)| id == session && ix < of.notes.len())
            .map(|(_, ix)| ix)
            .or_else(|| (!of.notes.is_empty()).then_some(0));
        match target {
            Some(ix) => self.show_note(session, ix, false, window, cx),
            None => self.new_note(session, None, panel, window, cx),
        }
        true
    }
}
