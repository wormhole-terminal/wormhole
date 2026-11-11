//! The row's mark for notes and tasks still open (the app's `features/notes` `NotesOf::open_items`):
//! tasks not done, plus notes with something in them. Where the notes live (the app's editors today, a
//! store the Swift side reads later) does not matter: the shell hands over what it has and the count
//! is this rule's.

use crate::view::notes::paragraphs::chunks_blank;

/// How many items are open: each task that is not done (`tasks_done`: one flag per task) and each note
/// whose text is more than whitespace.
pub fn open_items<'a>(
    tasks_done: impl IntoIterator<Item = bool>,
    notes: impl IntoIterator<Item = &'a str>,
) -> usize {
    let tasks = tasks_done.into_iter().filter(|done| !done).count();
    let notes = notes.into_iter().filter(|text| !chunks_blank(std::iter::once(*text))).count();
    tasks + notes
}

#[cfg(test)]
mod tests {
    use super::open_items;

    #[test]
    fn a_blank_note_and_a_done_task_are_not_open() {
        assert_eq!(open_items([false, true, false], ["fix the login", "  \n\t", ""]), 3);
        assert_eq!(open_items([true], ["\u{a0}"]), 0, "whitespace is blank in any script");
        assert_eq!(open_items([], []), 0);
    }
}
