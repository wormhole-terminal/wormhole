//! A selection a full-screen program draws itself, and pasting over it.
//!
//! A program that tracks the mouse (Claude Code) selects text on its own: it gets the press, the
//! drag and the release, highlights the text and copies it with OSC 52 when the button goes up.
//! Backspace then deletes the selection, but a paste goes in at the cursor and leaves the selected
//! text where it was. ⌘V over a selection should replace it, as in any text field, so the
//! terminal remembers the selection the program copied and, on ⌘V, sends a Backspace ahead of the
//! paste. The copy also took the clipboard: what was on it before is what to paste.
//!
//! Only a selection on the cursor's rows counts: elsewhere (the transcript above an input box)
//! the program doesn't delete it, and the Backspace would eat a character of the input instead.

use std::time::{Duration, Instant};

/// How long after the button goes up the program's copy still belongs to that selection.
const COPY_WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub struct AppSelection {
    gesture: Option<Gesture>,
    selection: Option<Selected>,
}

/// The rows of a left-button press (and drag) reported to the program.
#[derive(Debug)]
struct Gesture {
    first_row: usize,
    last_row: usize,
    released: Option<Instant>,
}

#[derive(Debug)]
struct Selected {
    first_row: usize,
    last_row: usize,
    copied: String,
    /// What the clipboard held before the program's copy.
    replaced: Option<String>,
}

/// What ⌘V does.
#[derive(Debug, PartialEq, Eq)]
pub struct PastePlan {
    /// Delete the program's selection first (a Backspace).
    pub replace_selection: bool,
    pub text: Option<String>,
}

impl AppSelection {
    /// The left button went down: a new selection starts, any earlier one is gone.
    pub fn pressed(&mut self, row: usize) {
        self.selection = None;
        self.gesture = Some(Gesture { first_row: row, last_row: row, released: None });
    }

    /// The button went up at `row`.
    pub fn released(&mut self, row: usize, now: Instant) {
        if let Some(gesture) = &mut self.gesture {
            gesture.first_row = gesture.first_row.min(row);
            gesture.last_row = gesture.last_row.max(row);
            gesture.released = Some(now);
        }
    }

    /// A key (or anything else the program reads as input) went to the program: whatever it
    /// had selected is no longer something ⌘V should replace.
    pub fn forget(&mut self) {
        self.gesture = None;
        self.selection = None;
    }

    /// Whether a copy the program makes now comes from a mouse selection; if so, call
    /// [`Self::copied`] with what the clipboard held.
    pub fn copy_from_selection(&self, now: Instant) -> bool {
        self.gesture
            .as_ref()
            .and_then(|gesture| gesture.released)
            .is_some_and(|released| now.saturating_duration_since(released) <= COPY_WINDOW)
    }

    /// The program copied `copied` (OSC 52) right after a mouse selection; the clipboard held
    /// `replaced`.
    pub fn copied(&mut self, copied: String, replaced: Option<String>) {
        let Some(gesture) = self.gesture.take() else { return };
        self.selection =
            Some(Selected { first_row: gesture.first_row, last_row: gesture.last_row, copied, replaced });
    }

    /// What ⌘V does with `clipboard` and the cursor on `cursor_row` (of the screen). Either way
    /// the selection is used up: after a paste the program has none left to replace.
    pub fn paste(&mut self, clipboard: Option<String>, cursor_row: Option<usize>) -> PastePlan {
        self.gesture = None;
        let selection = self.selection.take().filter(|selection| {
            cursor_row.is_some_and(|row| (selection.first_row..=selection.last_row).contains(&row))
        });
        let Some(selection) = selection else {
            return PastePlan { replace_selection: false, text: clipboard };
        };
        // Pasting the selection over itself means nothing: the copy only took the clipboard.
        let text = match clipboard {
            Some(text) if text == selection.copied => selection.replaced.or(Some(text)),
            other => other,
        };
        PastePlan { replace_selection: true, text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(rows: (usize, usize), copied: &str, replaced: Option<&str>) -> AppSelection {
        let now = Instant::now();
        let mut selection = AppSelection::default();
        selection.pressed(rows.0);
        selection.released(rows.1, now);
        assert!(selection.copy_from_selection(now));
        selection.copied(copied.into(), replaced.map(Into::into));
        selection
    }

    fn plan(replace_selection: bool, text: Option<&str>) -> PastePlan {
        PastePlan { replace_selection, text: text.map(Into::into) }
    }

    #[test]
    fn pasting_over_a_selection_on_the_input_replaces_it_with_what_was_copied_before() {
        let mut selection = selected((27, 27), "hello", Some("XYZ"));
        assert_eq!(selection.paste(Some("hello".into()), Some(27)), plan(true, Some("XYZ")));
        assert_eq!(selection.paste(Some("hello".into()), Some(27)), plan(false, Some("hello")), "used up");
    }

    #[test]
    fn something_copied_since_is_pasted_instead() {
        let mut selection = selected((27, 27), "hello", Some("XYZ"));
        assert_eq!(selection.paste(Some("other".into()), Some(27)), plan(true, Some("other")));
    }

    #[test]
    fn an_empty_clipboard_before_still_deletes_the_selection() {
        let mut selection = selected((27, 27), "hello", None);
        assert_eq!(selection.paste(Some("hello".into()), Some(27)), plan(true, Some("hello")));
    }

    #[test]
    fn a_selection_off_the_cursors_rows_is_left_alone() {
        let mut selection = selected((3, 5), "transcript", Some("XYZ"));
        assert_eq!(selection.paste(Some("transcript".into()), Some(27)), plan(false, Some("transcript")));
        let mut selection = selected((5, 3), "dragged up", None);
        assert_eq!(selection.paste(None, Some(4)), plan(true, None), "rows in either order");
    }

    #[test]
    fn a_key_or_a_new_click_forgets_the_selection() {
        let mut selection = selected((27, 27), "hello", Some("XYZ"));
        selection.forget();
        assert_eq!(selection.paste(Some("hello".into()), Some(27)), plan(false, Some("hello")));

        let mut selection = selected((27, 27), "hello", Some("XYZ"));
        selection.pressed(27);
        assert_eq!(selection.paste(Some("hello".into()), Some(27)), plan(false, Some("hello")));
    }

    #[test]
    fn only_a_copy_right_after_the_release_comes_from_the_selection() {
        let now = Instant::now();
        let mut selection = AppSelection::default();
        assert!(!selection.copy_from_selection(now), "no gesture");
        selection.pressed(1);
        assert!(!selection.copy_from_selection(now), "still held");
        selection.released(1, now);
        assert!(selection.copy_from_selection(now + COPY_WINDOW));
        assert!(!selection.copy_from_selection(now + COPY_WINDOW * 2));
    }
}
