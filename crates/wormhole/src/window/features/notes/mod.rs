//! Notes and next steps: the first feature that owns its state. One [`Notes`] entity per window holds,
//! for each terminal of the window, its next steps, its notes (an editor each), the folds and the
//! Notes tab's list ([`NotesOf`]), keyed by the terminal's id. The window talks to it through the
//! table (`def`): its tab, its keys, its row count, its stake in the close question, and the terminal's
//! life (`opened`, `record`, `closed`, `left`); it talks to the window by [`FeatureEvent`]s and never
//! holds the `Workspace`.
//!
//! What needs no GPUI is core's (`view::notes`: the text rules, the next steps as a list, the tab's
//! rows, the item editors' keys, the stake's words). Here are the editors, the subscriptions and the
//! drawing. A terminal's entry exists whether the feature is on or off: off, it is only not drawn,
//! bound or counted, and still saved.
//!
//! The same file layout as the table asks of every feature: `def` (the entry), `list` (the rows and the
//! keys over them), `note_ops` and `tasks` (what is done to them), `view` (the drawing), `api` (what the
//! project folder's notes.md sync may do, its bridge).

use std::collections::HashMap;

use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::*;
use wormhole_core::config::FeatureId;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::view::notes::paragraphs::chunks_blank;
use wormhole_core::view::notes::tasks::{Edit, Tasks};

use crate::window::feature::FeatureEvent;

pub(in crate::window) mod api;
mod def;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod items;
mod list;
mod note_ops;
#[cfg(test)]
mod note_tests;
#[cfg(test)]
mod state_tests;
mod task_rows;
mod tasks;
#[cfg(test)]
pub(in crate::window) mod testing;
mod view;

pub(in crate::window) use def::DEF;
use list::NotesList;

/// How tall a note's editor grows before it scrolls inside: every line shows while it's edited
/// or after "Show all"; the panel cuts an open note at `NOTE_OPEN_MAX_LINES` otherwise.
const NOTE_ROWS_MAX: usize = 1000;
/// ... and the fewest rows an editor has, however short its text.
const NOTE_ROWS_MIN: usize = 3;

/// Whether notes and tasks are on (the feature table's mask: the `notes_and_tasks` flag). Off, no tab,
/// key, count or sync shows or reacts, while what was written stays in the database for when it is on
/// again.
pub(in crate::window) fn on(cx: &App) -> bool {
    crate::settings::features(cx).has(FeatureId::Notes)
}

/// The window's notes and tasks: every terminal's, by its id.
pub(in crate::window) struct Notes {
    of: HashMap<i64, NotesOf>,
    /// The note ⌘↵ and the stash act on: terminal and index. Only the selected terminal's, so it is
    /// let go of when another is chosen.
    focused: Option<(i64, usize)>,
    /// The note whose text has the keyboard right now (drawn as the active item).
    editing: Option<EntityId>,
    /// The panel's width as the feature last heard it, for the hint of a note made without a frame
    /// to ask (a terminal opened at launch).
    panel: f32,
}

impl EventEmitter<FeatureEvent> for Notes {}

/// One terminal's notes and tasks.
pub(in crate::window) struct NotesOf {
    tasks: Tasks,
    /// The task being written or edited in place, if any.
    edit: Option<TaskEdit>,
    notes: Vec<Note>,
    /// The Notes section is folded up; the notes themselves are kept.
    notes_collapsed: bool,
    /// The tab's virtual list (bands, tasks and notes) and its folded done steps.
    list: NotesList,
    /// Notes with something in them, kept as notes change so a sidebar row reads a number instead of
    /// every note's text on each frame.
    filled: usize,
}

struct Note {
    state: Entity<TextareaState>,
    collapsed: bool,
    /// "Show all" was clicked: the open note isn't cut at `NOTE_OPEN_MAX_LINES` (until folded).
    show_all: bool,
    _subscription: Subscription,
}

/// The task being written or edited in place, and its editor.
struct TaskEdit {
    edit: Edit,
    input: Entity<InputState>,
    _subscription: Subscription,
}

impl NotesOf {
    fn new(record: &SessionRecord) -> Self {
        Self {
            tasks: Tasks::from_records(&record.tasks, record.tasks_collapsed),
            edit: None,
            notes: Vec::new(),
            notes_collapsed: record.notes_collapsed,
            list: NotesList::default(),
            filled: 0,
        }
    }

    /// Notes with something in them plus tasks not done.
    fn open_items(&self) -> usize {
        self.filled + self.tasks.open()
    }

    /// Counts the notes with something in them again; after any change to a note's text or to the
    /// set of notes.
    fn recount(&mut self, cx: &App) {
        self.filled = self.notes.iter().filter(|note| !is_blank_note(note.state.read(cx))).count();
    }
}

impl Notes {
    pub(in crate::window) fn new() -> Self {
        Self {
            of: HashMap::new(),
            focused: None,
            editing: None,
            panel: wormhole_core::view::layout::PANEL_MIN,
        }
    }

    /// Notes with something in them plus tasks not done, summed over these terminals.
    pub(super) fn open_items(&self, sessions: &[i64]) -> usize {
        sessions.iter().filter_map(|id| self.of.get(id)).map(NotesOf::open_items).sum()
    }

    /// The terminal's notes and tasks were read from its record: makes its entry. The notes' editors
    /// need `window`.
    pub(in crate::window) fn open(
        &mut self,
        record: &SessionRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.of.insert(record.id, NotesOf::new(record));
        for note in &record.notes {
            self.add_note(record.id, Some(note.body.clone()), note.collapsed, self.panel, window, cx);
        }
    }

    /// Writes the terminal's notes and tasks into its record.
    pub(in crate::window) fn record(&self, session: i64, record: &mut SessionRecord, cx: &App) {
        let Some(of) = self.of.get(&session) else { return };
        record.tasks = of.tasks.to_records();
        record.tasks_collapsed = of.tasks.collapsed;
        record.notes_collapsed = of.notes_collapsed;
        record.notes = of
            .notes
            .iter()
            .map(|note| wormhole_core::storage::store::NoteRecord {
                // From the rope: `value()` would copy the text into a `SharedString` first.
                body: note.state.read(cx).text().to_string(),
                collapsed: note.collapsed,
            })
            .collect();
    }

    /// The terminal is gone from this window (closed, or moved to another one): its entry goes with
    /// its editors. Nothing else is stored for it here.
    pub(in crate::window) fn forget(&mut self, session: i64) {
        self.of.remove(&session);
        if self.focused.is_some_and(|(id, _)| id == session) {
            self.focused = None;
        }
    }

    /// Another terminal was chosen: the focused note belonged to the one before.
    pub(in crate::window) fn selected(&mut self) {
        self.focused = None;
    }

    fn of_mut(&mut self, session: i64) -> Option<&mut NotesOf> {
        self.of.get_mut(&session)
    }

    /// Tells the window something to save changed.
    fn dirty(&self, cx: &mut Context<Self>) {
        cx.emit(FeatureEvent::Dirty);
        cx.notify();
    }
}

/// Whether the editor holds nothing but whitespace. Read where the text is, not copied: this
/// runs for every note of every terminal on the tick.
pub(in crate::window) fn is_blank_note(state: &TextareaState) -> bool {
    chunks_blank(state.text().chunks())
}

/// What ⌘↵ sends from a note: the selection, else the paragraph under the cursor.
pub(in crate::window) fn selection_or_paragraph(notes: &TextareaState) -> String {
    let selected = notes.selected_value().to_string();
    if selected.trim().is_empty() {
        wormhole_core::view::notes::paragraphs::paragraph_at(&notes.value(), notes.cursor())
    } else {
        selected
    }
}
