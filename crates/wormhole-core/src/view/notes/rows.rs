//! The Notes tab as a list of rows, and the rows the keys can be on. The tab shows two cards, Next
//! steps and Notes, as one virtual list; this works out its rows from what the terminal has, and
//! the app draws them.

use std::sync::Arc;

use super::tasks::{Edit, Tasks};
use crate::view::focus::list::Item;
use crate::view::panel::{self, StepRow};

/// One line of the Notes tab (tasks and notes share one virtual list).
#[derive(Clone, Debug, PartialEq)]
pub enum NotesRow {
    TasksBand,
    /// `text` is only here so that a different task in the same slot re-measures.
    Task {
        ix: usize,
        text: Arc<str>,
        editing: bool,
    },
    /// A new task being written, in the place it will go.
    TaskDraft,
    /// "N done": the done steps, folded after the open ones; `open` shows them below it.
    DoneFold {
        count: usize,
        open: bool,
    },
    NotesBand,
    /// `id` (the note's editor) tells notes apart when they change places.
    Note {
        ix: usize,
        id: u64,
        collapsed: bool,
    },
    /// Bottom padding, so the last row can scroll clear of the panel edge.
    Spacer,
}

/// What the rows need to know of one note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteFact {
    /// Tells it apart from the others when notes change places.
    pub id: u64,
    pub collapsed: bool,
}

/// Whether a section of `items` rows is folded away. An empty one never is: it has no chevron to
/// open it with.
pub fn folded(items: usize, collapsed: bool) -> bool {
    items > 0 && collapsed
}

/// The rows of a terminal's Notes tab: the Next steps band and its steps (a new one where it will
/// go in, the done ones folded after the open ones past a limit, `done_open` showing them), then
/// the Notes band and its notes, then the spacer.
pub fn rows(
    tasks: &Tasks,
    edit: Option<Edit>,
    done_open: bool,
    notes: impl ExactSizeIterator<Item = NoteFact>,
    notes_collapsed: bool,
) -> Vec<NotesRow> {
    let mut rows = vec![NotesRow::TasksBand];
    let draft = edit.filter(|edit| edit.new).map(|edit| edit.ix.min(tasks.len()));
    let steps = tasks.len() + usize::from(draft.is_some());
    if !folded(steps, tasks.collapsed) {
        let done: Vec<bool> = tasks.items().iter().map(|task| task.done).collect();
        rows.extend(panel::steps(&done, draft, done_open).into_iter().map(|row| match row {
            StepRow::Step(ix) => NotesRow::Task {
                ix,
                text: tasks.items()[ix].text.clone(),
                editing: edit == Some(Edit { ix, new: false }),
            },
            StepRow::Draft => NotesRow::TaskDraft,
            StepRow::Done(count) => NotesRow::DoneFold { count, open: done_open },
        }));
    }
    rows.push(NotesRow::NotesBand);
    if !folded(notes.len(), notes_collapsed) {
        rows.extend(notes.enumerate().map(|(ix, note)| NotesRow::Note {
            ix,
            id: note.id,
            collapsed: note.collapsed,
        }));
    }
    rows.push(NotesRow::Spacer);
    rows
}

/// A row the keys can be on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stop {
    /// Its place among the tab's rows.
    pub row: usize,
    pub item: Item,
    /// A section's band, where a ⇥ stop starts.
    pub band: bool,
}

/// The rows the keys can be on: the two bands (⇥ stops), each next step, the done steps' fold and
/// each note. A step being written is its editor's. `tasks_fold` and `notes_fold` say whether each
/// band folds and how (`None`: nothing in the section), as the band's `Item` carries it.
pub fn walkable<'a>(
    rows: &'a [NotesRow],
    tasks_fold: Option<bool>,
    notes_fold: Option<bool>,
) -> impl Iterator<Item = Stop> + 'a {
    rows.iter().enumerate().filter_map(move |(row, line)| {
        let (level, fold, band) = match line {
            NotesRow::TasksBand => (0, tasks_fold, true),
            NotesRow::NotesBand => (0, notes_fold, true),
            NotesRow::DoneFold { open, .. } => (1, Some(*open), false),
            NotesRow::Task { .. } | NotesRow::Note { .. } => (1, None, false),
            NotesRow::TaskDraft | NotesRow::Spacer => return None,
        };
        Some(Stop { row, item: Item { level, fold }, band })
    })
}

#[cfg(test)]
mod tests {
    use super::{NoteFact, NotesRow, folded, rows, walkable};
    use crate::view::notes::tasks::{Edit, Task, Tasks};

    fn tasks(items: &[(&str, bool)]) -> Tasks {
        let mut list = Tasks::default();
        list.replace(items.iter().map(|&(text, done)| Task { text: text.into(), done }));
        list
    }

    fn notes(n: usize) -> impl ExactSizeIterator<Item = NoteFact> {
        (0..n).map(|id| NoteFact { id: id as u64, collapsed: false })
    }

    fn collapsed_empty() -> Tasks {
        let mut list = tasks(&[]);
        list.collapsed = true;
        list
    }

    #[test]
    fn an_empty_tab_is_two_bands_and_the_spacer() {
        assert_eq!(
            rows(&Tasks::default(), None, false, notes(0), false),
            [NotesRow::TasksBand, NotesRow::NotesBand, NotesRow::Spacer]
        );
    }

    #[test]
    fn a_folded_section_hides_its_rows_but_an_empty_one_cannot_be_folded() {
        let list = tasks(&[("a", false)]);
        let mut folded_list = list.clone();
        folded_list.collapsed = true;
        assert_eq!(rows(&folded_list, None, false, notes(2), true).len(), 3, "bands and spacer");
        assert!(!folded(0, true));
        assert!(folded(1, true));
        // A step being written counts as one, so its section is not "empty".
        let edit = Some(Edit { ix: 0, new: true });
        assert_eq!(rows(&collapsed_empty(), edit, false, notes(0), false).len(), 3);
    }

    #[test]
    fn steps_a_draft_and_notes_come_in_order_with_what_re_measures() {
        let list = tasks(&[("a", false), ("b", false)]);
        let edit = Some(Edit { ix: 1, new: false });
        let got = rows(&list, edit, false, notes(1), false);
        assert_eq!(
            got,
            [
                NotesRow::TasksBand,
                NotesRow::Task { ix: 0, text: "a".into(), editing: false },
                NotesRow::Task { ix: 1, text: "b".into(), editing: true },
                NotesRow::NotesBand,
                NotesRow::Note { ix: 0, id: 0, collapsed: false },
                NotesRow::Spacer,
            ]
        );
        let draft = rows(&list, Some(Edit { ix: 9, new: true }), false, notes(0), false);
        assert_eq!(draft[3], NotesRow::TaskDraft, "a new step goes in at the end when asked past it");
    }

    #[test]
    fn the_keys_stop_on_bands_steps_the_done_fold_and_notes() {
        let list = tasks(&[("a", false)]);
        let got = rows(&list, Some(Edit { ix: 1, new: true }), false, notes(1), false);
        let stops: Vec<_> =
            walkable(&got, Some(true), Some(true)).map(|s| (s.row, s.item.level, s.band)).collect();
        // The draft (row 2) and the spacer are not stops.
        assert_eq!(stops, [(0, 0, true), (1, 1, false), (3, 0, true), (4, 1, false)]);
        let bands: Vec<_> =
            walkable(&got, Some(true), None).filter(|s| s.band).map(|s| s.item.fold).collect();
        assert_eq!(bands, [Some(true), None]);
    }
}
