//! The next steps of a terminal: a list that is added to, edited in place, ticked, moved and deleted,
//! with the rules of what each does to the list and to an edit in progress. The app owns the editor
//! (an input) and says when; this decides.

use std::sync::Arc;

use crate::storage::store::TaskRecord;
use crate::view::reorder;

/// One next step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Shared, so a row that carries it for re-measuring copies a pointer.
    pub text: Arc<str>,
    pub done: bool,
}

/// The task being written or edited in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edit {
    /// The task being edited; for a `new` one, where it goes (before the task now there).
    pub ix: usize,
    pub new: bool,
}

impl Edit {
    /// The edit after task `deleted` was removed: gone with its own task, else kept on the task it
    /// was on. A new task being written is nobody's.
    pub fn after_delete(self, deleted: usize) -> Option<Self> {
        match self {
            Self { new: false, ix } if ix == deleted => None,
            Self { ix, new } if ix > deleted => Some(Self { ix: ix - 1, new }),
            other => Some(other),
        }
    }

    /// The edit after the task at `from` moved to `to`: a task being edited goes with its place.
    pub fn after_move(self, from: usize, to: usize) -> Self {
        if self.new { self } else { Self { ix: reorder::follow(self.ix, from, to), ..self } }
    }
}

/// What finishing an edit did to the list, so an index taken before it can be corrected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finished {
    /// The task kept its text (or took the new one) at this index.
    Kept(usize),
    /// A new task went in here.
    Inserted(usize),
    /// The task here was emptied, and deleted.
    Removed(usize),
    /// A new task was left empty: nothing changed.
    Dropped,
}

impl Finished {
    /// Where to edit next when the task at `ix` was asked for (`new`: a new task going in there)
    /// while another edit finished with this: `ix` moved by what finishing inserted or removed.
    /// `None` when the task asked for was the one just emptied, and is gone.
    pub fn retarget(self, ix: usize, new: bool) -> Option<usize> {
        match self {
            Self::Inserted(at) if at <= ix => Some(ix + 1),
            Self::Removed(at) if at < ix => Some(ix - 1),
            Self::Removed(at) if at == ix && !new => None,
            _ => Some(ix),
        }
    }
}

/// A terminal's next steps and whether their section is folded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tasks {
    items: Vec<Task>,
    /// The Next steps section is folded up; the steps themselves are kept.
    pub collapsed: bool,
}

impl Tasks {
    /// The steps a saved terminal had.
    pub fn from_records(records: &[TaskRecord], collapsed: bool) -> Self {
        Self {
            items: records.iter().map(|t| Task { text: t.text.as_str().into(), done: t.done }).collect(),
            collapsed,
        }
    }

    /// What is saved of them.
    pub fn to_records(&self) -> Vec<TaskRecord> {
        self.items.iter().map(|t| TaskRecord { text: t.text.to_string(), done: t.done }).collect()
    }

    pub fn items(&self) -> &[Task] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The steps still to do.
    pub fn open(&self) -> usize {
        self.items.iter().filter(|t| !t.done).count()
    }

    /// Replaces the steps (a sync with the project's notes.md), leaving the fold alone.
    pub fn replace(&mut self, items: impl IntoIterator<Item = Task>) {
        self.items = items.into_iter().collect();
    }

    /// An empty section has no chevron to open it with, so it is never left folded.
    fn unfold_if_empty(&mut self) {
        self.collapsed &= !self.items.is_empty();
    }

    /// Adds a finished step at the end, and unfolds the section. Blank text adds nothing.
    pub fn add(&mut self, text: &str, done: bool) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.items.push(Task { text: text.into(), done });
        self.collapsed = false;
        true
    }

    /// Ticks or unticks step `ix`.
    pub fn toggle(&mut self, ix: usize) -> bool {
        let Some(task) = self.items.get_mut(ix) else { return false };
        task.done = !task.done;
        true
    }

    /// Deletes step `ix`.
    pub fn delete(&mut self, ix: usize) -> bool {
        if ix >= self.items.len() {
            return false;
        }
        self.items.remove(ix);
        self.unfold_if_empty();
        true
    }

    /// Moves step `from` to `to`.
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        reorder::move_item(&mut self.items, from, to)
    }

    /// Ends `edit` with what was typed: the text is kept (steps are one line, and one line in
    /// notes.md), a step edited down to nothing is deleted and a new one left empty is dropped.
    pub fn finish(&mut self, edit: Edit, typed: &str) -> Finished {
        let text = typed.split_whitespace().collect::<Vec<_>>().join(" ");
        let finished = if edit.new {
            if text.is_empty() {
                Finished::Dropped
            } else {
                let at = edit.ix.min(self.items.len());
                self.items.insert(at, Task { text: text.into(), done: false });
                Finished::Inserted(at)
            }
        } else if edit.ix >= self.items.len() {
            Finished::Dropped
        } else if text.is_empty() {
            self.items.remove(edit.ix);
            Finished::Removed(edit.ix)
        } else {
            if *self.items[edit.ix].text != *text {
                self.items[edit.ix].text = text.into();
            }
            Finished::Kept(edit.ix)
        };
        self.unfold_if_empty();
        finished
    }
}

#[cfg(test)]
mod tests {
    use super::{Edit, Finished, Task, Tasks};

    fn tasks(items: &[(&str, bool)]) -> Tasks {
        let mut list = Tasks::default();
        list.replace(items.iter().map(|&(text, done)| Task { text: text.into(), done }));
        list
    }

    fn texts(list: &Tasks) -> Vec<&str> {
        list.items().iter().map(|t| &*t.text).collect()
    }

    #[test]
    fn adding_trims_unfolds_and_ignores_blank_text() {
        let mut list = Tasks { collapsed: true, ..Tasks::default() };
        assert!(!list.add("  \n ", false));
        assert!(list.collapsed && list.is_empty());
        assert!(list.add("  ship it ", false));
        assert!(!list.collapsed, "a step added to a folded section opens it");
        assert_eq!(texts(&list), ["ship it"]);
    }

    #[test]
    fn open_counts_the_steps_not_done_and_toggling_flips_one() {
        let mut list = tasks(&[("a", false), ("b", true), ("c", false)]);
        assert_eq!(list.open(), 2);
        assert!(list.toggle(0));
        assert_eq!(list.open(), 1);
        assert!(!list.toggle(9));
    }

    #[test]
    fn deleting_the_last_step_unfolds_its_section() {
        let mut list = tasks(&[("a", false)]);
        list.collapsed = true;
        assert!(list.delete(0));
        assert!(!list.collapsed, "an empty section has no chevron to open it with");
        assert!(!list.delete(0));
    }

    #[test]
    fn finishing_an_edit_keeps_inserts_removes_or_drops() {
        let mut list = tasks(&[("a", false), ("b", false)]);
        assert_eq!(list.finish(Edit { ix: 1, new: false }, "  b   two "), Finished::Kept(1));
        assert_eq!(texts(&list), ["a", "b two"], "a step is one line");
        assert_eq!(list.finish(Edit { ix: 1, new: true }, "new"), Finished::Inserted(1));
        assert_eq!(texts(&list), ["a", "new", "b two"]);
        assert_eq!(list.finish(Edit { ix: 99, new: true }, "last"), Finished::Inserted(3));
        assert_eq!(list.finish(Edit { ix: 0, new: true }, "  "), Finished::Dropped);
        assert_eq!(list.finish(Edit { ix: 0, new: false }, " "), Finished::Removed(0));
        assert_eq!(texts(&list), ["new", "b two", "last"]);
        assert_eq!(list.finish(Edit { ix: 7, new: false }, "x"), Finished::Dropped);
    }

    #[test]
    fn an_index_asked_for_is_kept_on_its_step_when_finishing_changes_the_list() {
        assert_eq!(Finished::Inserted(1).retarget(2, false), Some(3));
        assert_eq!(Finished::Inserted(3).retarget(2, false), Some(2));
        assert_eq!(Finished::Removed(0).retarget(2, false), Some(1));
        assert_eq!(Finished::Removed(2).retarget(2, false), None, "the step asked for is gone");
        assert_eq!(Finished::Removed(2).retarget(2, true), Some(2), "a new one goes in there anyway");
        assert_eq!(Finished::Kept(2).retarget(2, false), Some(2));
        assert_eq!(Finished::Dropped.retarget(2, true), Some(2));
    }

    #[test]
    fn an_edit_follows_deletes_and_moves() {
        let edit = Edit { ix: 2, new: false };
        assert_eq!(edit.after_delete(2), None, "gone with its step");
        assert_eq!(edit.after_delete(0), Some(Edit { ix: 1, new: false }));
        assert_eq!(edit.after_delete(3), Some(edit));
        let draft = Edit { ix: 2, new: true };
        assert_eq!(draft.after_delete(2), Some(draft), "a new step is nobody's");
        assert_eq!(draft.after_delete(0), Some(Edit { ix: 1, new: true }));
        assert_eq!(edit.after_move(2, 0), Edit { ix: 0, new: false });
        assert_eq!(draft.after_move(2, 0), draft);
    }

    #[test]
    fn steps_round_trip_through_the_saved_records() {
        let list = tasks(&[("a", false), ("b", true)]);
        let records = list.to_records();
        assert_eq!(Tasks::from_records(&records, false).items(), list.items());
    }
}
