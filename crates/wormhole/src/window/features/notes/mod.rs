//! The Notes tab: the Next steps and Notes bands over one virtual list per
//! session, the notes themselves, and sending their text to the terminal.
use std::time::Duration;

use gpui_kit::component::input::Position;
use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::*;
use wormhole_core::view::reorder;

use crate::window::features::notes::items::{ItemKey, item_key};
use crate::window::features::notes::paragraphs::{chunks_blank, paragraph_at};
use crate::window::{RightTab, Workspace, panel_list};

pub(in crate::window) mod items;
#[cfg(test)]
mod note_tests;
pub(in crate::window) mod paragraphs;
pub(in crate::window) mod tasks;
mod view;

use crate::settings::ui_in;
use view::wrapped_placeholder;
pub(in crate::window) use view::{NotesList, NotesView};

pub(in crate::window) struct Note {
    pub(in crate::window) state: Entity<TextareaState>,
    pub(in crate::window) collapsed: bool,
    /// "Show all" was clicked: the open note isn't cut at `NOTE_OPEN_MAX_LINES` (until folded).
    pub(in crate::window) show_all: bool,
    _subscription: Subscription,
}

#[derive(Clone)]
pub(in crate::window) struct TaskItem {
    pub(in crate::window) text: SharedString,
    pub(in crate::window) done: bool,
}

/// How tall a note's editor grows before it scrolls inside: every line shows while it's edited
/// or after "Show all"; the panel cuts an open note at `NOTE_OPEN_MAX_LINES` otherwise.
const NOTE_ROWS_MAX: usize = 1000;
/// ... and the fewest rows an editor has, however short its text.
const NOTE_ROWS_MIN: usize = 3;

/// Whether notes and tasks are on: the `notes_and_tasks` feature flag. Off, no tab, key, count or
/// sync shows or reacts, while what was written stays in the database for when it is on again.
pub(crate) fn notes_on(cx: &App) -> bool {
    crate::settings::settings(cx).feature_enabled(wormhole_core::config::features::FeatureFlag::NotesAndTasks)
}

impl Workspace {
    pub(in crate::window) fn add_note(
        &mut self,
        session_ix: usize,
        value: Option<String>,
        collapsed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let hint = wrapped_placeholder(self.columns().panel, window, cx);
        let state = cx.new(|cx| {
            let state =
                TextareaState::new(window, cx).auto_grow(NOTE_ROWS_MIN, NOTE_ROWS_MAX).placeholder(hint);
            match value {
                Some(value) => state.default_value(value),
                None => state,
            }
        });
        let subscription =
            cx.subscribe_in(&state, window, |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { secondary: true, shift } => {
                    let text = selection_or_paragraph(state.read(cx));
                    this.send_text(text, *shift, window, cx);
                }
                InputEvent::Change => this.mark_dirty(cx),
                InputEvent::Focus => {
                    // The panel may have another width than when the note was made.
                    let hint = wrapped_placeholder(this.columns().panel, window, cx);
                    state.update(cx, |state, cx| state.set_placeholder(hint, window, cx));
                    this.focused_note =
                        this.active_session().notes.iter().position(|note| &note.state == state);
                    this.editing_note = Some(state.entity_id());
                    cx.notify();
                }
                // A note left empty is thrown away rather than kept as an empty entry.
                InputEvent::Blur => {
                    if this.editing_note == Some(state.entity_id()) {
                        this.editing_note = None;
                        cx.notify();
                    }
                    this.prune_blank_notes(window, cx);
                }
                _ => {}
            });
        let session = &mut self.sessions[session_ix];
        session.notes.push(Note { state, collapsed, show_all: false, _subscription: subscription });
        self.dirty = true;
        cx.notify();
        session.notes.len() - 1
    }

    /// Make the Notes tab what the panel shows, open and with its section unfolded, so a
    /// note added or focused next is on screen. (With Changes, History or Project showing, or
    /// the section folded, there is nothing to type into.)
    pub(in crate::window) fn reveal_notes_tab(&mut self, cx: &mut Context<Self>) {
        self.show_tab(RightTab::Notes, cx);
        self.sessions[self.active].notes_collapsed = false;
    }

    pub(in crate::window) fn new_note(
        &mut self,
        text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !notes_on(cx) {
            return;
        }
        self.reveal_notes_tab(cx);
        let ix = self.add_note(self.active, text, false, window, cx);
        self.focused_note = Some(ix);
        let state = self.active_session().notes[ix].state.clone();
        state.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Put note `ix` of the active session on screen and give it the keyboard: the Notes tab
    /// open, the section and the note unfolded. `to_end` also puts the cursor at the end of its
    /// text (clicking its title); otherwise it stays where it was (⇧⌘E).
    pub(in crate::window) fn show_note(
        &mut self,
        ix: usize,
        to_end: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ix >= self.active_session().notes.len() {
            return;
        }
        self.reveal_notes_tab(cx);
        let note = &mut self.sessions[self.active].notes[ix];
        note.collapsed = false;
        let state = note.state.clone();
        self.focused_note = Some(ix);
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
    pub(in crate::window) fn open_note(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.show_note(ix, true, window, cx);
    }

    /// Throw away every note with nothing in it that isn't being written right now. Runs
    /// when a note loses focus, and on the tick for one that never got it.
    pub(in crate::window) fn prune_blank_notes(&mut self, window: &Window, cx: &mut Context<Self>) {
        let blank = |note: &Note| {
            let state = note.state.read(cx);
            is_blank_note(state) && !state.focus_handle(cx).is_focused(window)
        };
        let mut changed = false;
        for s in 0..self.sessions.len() {
            let mut ix = 0;
            while ix < self.sessions[s].notes.len() {
                if !blank(&self.sessions[s].notes[ix]) {
                    ix += 1;
                    continue;
                }
                self.sessions[s].notes.remove(ix);
                if s == self.active {
                    self.focused_note = self.focused_note.and_then(|f| match f.cmp(&ix) {
                        std::cmp::Ordering::Less => Some(f),
                        std::cmp::Ordering::Equal => None,
                        std::cmp::Ordering::Greater => Some(f - 1),
                    });
                }
                changed = true;
            }
            let session = &mut self.sessions[s];
            session.notes_collapsed &= !session.notes.is_empty();
        }
        if changed {
            self.mark_dirty(cx);
        }
    }

    /// The keys a note's text shares with a task's editor ([`ItemKey`]).
    pub(in crate::window) fn note_key(
        &mut self,
        state: &Entity<TextareaState>,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let notes = &self.active_session().notes;
        let Some(ix) = notes.iter().position(|note| &note.state == state) else { return false };
        match item_key(&event.keystroke, state.read(cx).value().is_empty()) {
            // Leaving it empty throws it away (see `prune_blank_notes`).
            Some(ItemKey::Leave) => self.focus_terminal(window, cx),
            Some(ItemKey::DeleteEmpty) => {
                self.delete_note(ix, cx);
                match ix.checked_sub(1) {
                    Some(above) => self.open_note(above, window, cx),
                    None => self.focus_terminal(window, cx),
                }
            }
            Some(ItemKey::Move(delta)) => {
                if let Some(to) = reorder::step(ix, delta, notes.len()) {
                    self.move_note(ix, to, cx);
                }
            }
            None => return false,
        }
        true
    }

    /// Move a note to another place in the list (drag and drop, ⌥⌘↑/↓).
    pub(in crate::window) fn move_note(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if !reorder::move_item(&mut self.sessions[self.active].notes, from, to) {
            return;
        }
        self.focused_note = self.focused_note.map(|ix| reorder::follow(ix, from, to));
        self.mark_dirty(cx);
    }

    /// Fill the active session with `count` notes of `lines` lines each, for
    /// measuring how the Notes tab copes with a long list.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_seed_notes(
        &mut self,
        count: usize,
        lines: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for n in 0..count {
            let text = (0..lines)
                .map(|l| match l % 5 {
                    4 => String::new(),
                    _ => {
                        format!("Note {n} line {l}: some text that is long enough to wrap in a narrow panel.")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            self.add_note(self.active, Some(text), false, window, cx);
        }
    }

    /// Add a note with `text` at the end of the active session's list, open.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_add_note(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_notes_tab(cx);
        self.add_note(self.active, Some(text), false, window, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_toggle_note(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.toggle_note(ix, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_toggle_task(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.toggle_task(ix, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_panel_scroll(&mut self, y: f32, cx: &mut Context<Self>) {
        let list = match self.right_tab {
            RightTab::Changes => self.devctl_changes_list(),
            RightTab::History => self.devctl_history_list(),
            RightTab::Notes => self.devctl_notes_list(),
            RightTab::Project | RightTab::Usage => return,
        };
        list.scroll_by(px(y));
        cx.notify();
    }

    pub(in crate::window) fn delete_note(&mut self, ix: usize, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        if ix < session.notes.len() {
            session.notes.remove(ix);
            // An empty section has no chevron to open it with.
            session.notes_collapsed &= !session.notes.is_empty();
            self.focused_note =
                self.focused_note.filter(|&f| f != ix).map(|f| if f > ix { f - 1 } else { f });
            self.mark_dirty(cx);
        }
    }

    pub(in crate::window) fn toggle_notes_section(&mut self, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        session.notes_collapsed = !session.notes_collapsed;
        // A note that was focused is about to disappear from the list.
        self.focused_note = None;
        self.mark_dirty(cx);
    }

    pub(in crate::window) fn toggle_note(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(note) = self.sessions[self.active].notes.get_mut(ix) {
            note.collapsed = !note.collapsed;
            // Opened again, a long note is cut at the cap again.
            note.show_all = false;
            self.mark_dirty(cx);
        }
    }

    /// "Show all · N lines" under a long open note.
    fn show_all_of_note(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(note) = self.sessions[self.active].notes.get_mut(ix) {
            note.show_all = true;
            cx.notify();
        }
    }

    /// "N done" under the open next steps: show or fold the done ones.
    pub(in crate::window) fn toggle_done_steps(&mut self, cx: &mut Context<Self>) {
        let list = &mut self.sessions[self.active].notes_list;
        list.done_open = !list.done_open;
        cx.notify();
    }

    /// Paste text into the active terminal, optionally pressing Enter after.
    pub(crate) fn send_text(
        &mut self,
        text: String,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let terminal = self.active_terminal();
        terminal.update(cx, |t, _| t.paste(&text));
        if submit {
            // Give the app a moment to process the paste before Enter.
            let terminal = terminal.clone();
            cx.spawn(async move |_, cx| {
                cx.background_executor().timer(Duration::from_millis(60)).await;
                terminal.update(cx, |t, _| t.write(b"\r".to_vec()));
            })
            .detach();
        }
        window.focus(&terminal.focus_handle(cx), cx);
    }

    /// ⌘↵ outside a note: send from the last focused note.
    #[cfg(feature = "devctl")]
    pub(crate) fn send_focused_note(&mut self, submit: bool, window: &mut Window, cx: &mut Context<Self>) {
        let session = self.active_session();
        let Some(note) = self.focused_note.and_then(|ix| session.notes.get(ix)) else {
            return;
        };
        let text = selection_or_paragraph(note.state.read(cx));
        self.send_text(text, submit, window, cx);
    }

    /// Move the prompt typed in the terminal (or the selection) into a new note.
    pub(crate) fn stash_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !notes_on(cx) {
            return;
        }
        let Some(text) = self.active_terminal().read(cx).grab_prompt() else {
            return;
        };
        self.reveal_notes_tab(cx);
        self.add_note(self.active, Some(text), false, window, cx);
        // Keep typing in the terminal (e.g. to answer the agent's question).
        window.focus(&self.active_terminal().focus_handle(cx), cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn set_first_note(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_session().notes.is_empty() {
            self.add_note(self.active, None, false, window, cx);
        }
        let state = self.active_session().notes[0].state.clone();
        state.update(cx, |n, cx| n.set_value(SharedString::from(text), window, cx));
        self.focused_note = Some(0);
        self.sessions[self.active].notes_collapsed = false;
    }

    // ---------------------------------------------------------------- notes tab

    /// The right panel, `width` points wide (its column's, or its width at rest while it slides).
    pub(in crate::window) fn render_notes_panel(
        &self,
        width: f32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ui = ui_in(&self.profile, cx);
        // The tab actually on screen: if the chosen one isn't in the tab list any more (the
        // Project tab vanished with its feature flag), show Notes until the next refresh resets it.
        let tab = if self.panel_tab_list(cx).contains(&self.right_tab) {
            self.right_tab
        } else {
            RightTab::Changes
        };
        div()
            .relative()
            .flex()
            .flex_col()
            // Narrower than chosen while the window can't spare it (see core `Panes::columns`).
            .w(px(width))
            .flex_none()
            .h_full()
            .bg(ui.panel_right)
            .child(self.render_panel_tab_bar(window, cx))
            .child(div().flex().flex_col().flex_1().min_h_0().child(match tab {
                // Virtualized lists fill the panel and scroll themselves; the list area holds the
                // keyboard for them.
                RightTab::Changes => {
                    let list = self.render_changes_tab(window, cx);
                    self.panel_content(list, cx)
                }
                RightTab::History => {
                    let list = self.render_history_tab(window, cx);
                    self.panel_content(list, cx)
                }
                RightTab::Notes => {
                    let list = self.render_notes_tab(window, cx);
                    self.panel_content(list, cx)
                }
                // Short tabs, all there at once: a plain scrolling column.
                RightTab::Project => {
                    let content = self.render_project_tab(cx).into_any_element();
                    panel_list::scroll_column("project-scroll", &self.panel_scroll, ui, content, cx)
                }
                RightTab::Usage => {
                    let content = self.render_usage_tab(cx).into_any_element();
                    panel_list::scroll_column("usage-scroll", &self.panel_scroll, ui, content, cx)
                }
            }))
    }

    pub(crate) fn show_tab(&mut self, tab: RightTab, cx: &mut Context<Self>) {
        // The Notes tab is gone while its flag is off.
        let tab = if tab == RightTab::Notes && !notes_on(cx) { RightTab::Changes } else { tab };
        let tab = if tab == RightTab::Usage && !self.usage_tab_on(cx) { RightTab::Changes } else { tab };
        if tab == RightTab::Usage && self.right_tab != RightTab::Usage {
            self.tab_before_usage = Some(self.right_tab);
        }
        if tab != self.right_tab {
            self.panel_scroll.set_offset(Point::default());
        }
        self.right_tab = tab;
        self.reveal_notes();
        self.remember_panes(cx);
        self.picker = None;
        let ix = self.active;
        match tab {
            RightTab::Changes => self.refresh_git(ix, cx),
            RightTab::History => self.refresh_history(ix, cx),
            RightTab::Project => self.refresh_projects(cx),
            RightTab::Notes | RightTab::Usage => {}
        }
        self.mark_dirty(cx);
    }
}

/// Whether the editor holds nothing but whitespace. Read where the text is, not copied: this
/// runs for every note of every session on the tick and in the sidebar's render.
pub(in crate::window) fn is_blank_note(state: &TextareaState) -> bool {
    chunks_blank(state.text().chunks())
}

pub(in crate::window) fn selection_or_paragraph(notes: &TextareaState) -> String {
    let selected = notes.selected_value().to_string();
    if selected.trim().is_empty() { paragraph_at(&notes.value(), notes.cursor()) } else { selected }
}
