//! What the picker's keys do, as one pure rule, so the panel that owns the keyboard can ask it on the spot
//! (is this arrow mine, or the field's?) and the model can act on the answer with what it holds.
//!
//! The panel hands over a [`Key`] and the field as it is ([`Field`]); [`Facts`] is what the model last said
//! about itself, small enough to keep beside the field: AppKit wants its answer before the key returns,
//! and the model lives on the other side of a queue.

/// A key the picker takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Tab,
    Right,
    Left,
    Escape,
    /// ⌘⌫ on the selected row.
    Forget,
}

/// The text field as it stands: its text and where the caret is, as byte offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub text: String,
    /// Where the selection starts: the caret when nothing is selected.
    pub caret: usize,
    /// How many bytes are selected.
    pub selected: usize,
}

impl Field {
    /// A field with the caret after `text`, nothing selected.
    pub fn at_end(text: &str) -> Self {
        Self { text: text.to_string(), caret: text.len(), selected: 0 }
    }

    fn caret_at_end(&self) -> bool {
        self.selected == 0 && self.caret >= self.text.len()
    }

    fn caret_at_start(&self) -> bool {
        self.selected == 0 && self.caret == 0
    }
}

/// What the model says about itself that the keys depend on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// There is a level up from this list.
    pub up: bool,
    /// The selected row can be gone into.
    pub descends: bool,
    /// The field shows a ghost completion.
    pub ghost: bool,
    /// The selected row can be forgotten.
    pub forgettable: bool,
}

/// What a key does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Do {
    /// Not the picker's: the field has it (an arrow moves the caret).
    Pass,
    Close,
    Step(isize),
    Pick,
    Descend,
    Up,
    Forget,
}

/// How far a page key moves the selection.
pub const PAGE: isize = 8;

/// What `key` does with the field as `field` and the model as `facts`.
///
/// - ↑ ↓ and the page keys move the selection, ↩ picks, ⎋ closes: always the picker's.
/// - ⇥ goes into the selected row (which is also accepting the ghost, the ghost being its name).
/// - → goes into the selected row only with the caret after the last character, nothing selected (otherwise
///   it moves the caret), and only when there is something to go into or complete.
/// - ← goes up one level only with nothing selected and the caret at the very start of the field, or after a
///   closing `/` at its end, so it still moves the caret while a name is being edited.
/// - ⌘⌫ forgets the selected row when it can be forgotten, and otherwise deletes in the field as it does.
pub fn decide(key: Key, field: &Field, facts: &Facts) -> Do {
    match key {
        Key::Escape => Do::Close,
        Key::Up => Do::Step(-1),
        Key::Down => Do::Step(1),
        Key::PageUp => Do::Step(-PAGE),
        Key::PageDown => Do::Step(PAGE),
        Key::Enter => Do::Pick,
        Key::Tab => Do::Descend,
        Key::Right if field.caret_at_end() && (facts.ghost || facts.descends) => Do::Descend,
        Key::Left
            if facts.up && (field.caret_at_start() || field.caret_at_end() && field.text.ends_with('/')) =>
        {
            Do::Up
        }
        Key::Forget if facts.forgettable => Do::Forget,
        Key::Right | Key::Left | Key::Forget => Do::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::{Do, Facts, Field, Key, decide};

    const ALL: Facts = Facts { up: true, descends: true, ghost: true, forgettable: true };

    fn at(text: &str, caret: usize) -> Field {
        Field { text: text.into(), caret, selected: 0 }
    }

    #[test]
    fn the_list_keys_are_always_the_pickers() {
        let none = Facts::default();
        let f = at("abc", 1);
        assert_eq!(decide(Key::Up, &f, &none), Do::Step(-1));
        assert_eq!(decide(Key::Down, &f, &none), Do::Step(1));
        assert_eq!(decide(Key::PageDown, &f, &none), Do::Step(8));
        assert_eq!(decide(Key::Enter, &f, &none), Do::Pick);
        assert_eq!(decide(Key::Escape, &f, &none), Do::Close);
        assert_eq!(decide(Key::Tab, &f, &none), Do::Descend);
    }

    #[test]
    fn right_descends_only_at_the_end_of_the_text() {
        assert_eq!(decide(Key::Right, &at("src", 3), &ALL), Do::Descend);
        assert_eq!(decide(Key::Right, &at("src", 1), &ALL), Do::Pass, "it moves the caret");
        let selected = Field { text: "src".into(), caret: 0, selected: 3 };
        assert_eq!(decide(Key::Right, &selected, &ALL), Do::Pass, "with a selection it collapses it");
        let nothing = Facts { descends: false, ghost: false, ..ALL };
        assert_eq!(decide(Key::Right, &at("src", 3), &nothing), Do::Pass, "nothing to go into");
    }

    #[test]
    fn left_goes_up_at_the_start_or_after_a_slash_at_the_end_and_otherwise_moves_the_caret() {
        assert_eq!(decide(Key::Left, &at("", 0), &ALL), Do::Up, "an empty field");
        assert_eq!(decide(Key::Left, &at("src", 0), &ALL), Do::Up, "at the start");
        assert_eq!(decide(Key::Left, &at("src/", 4), &ALL), Do::Up, "after the slash at the end");
        assert_eq!(decide(Key::Left, &at("src/ui", 6), &ALL), Do::Pass, "editing a name");
        assert_eq!(decide(Key::Left, &at("src/ui", 3), &ALL), Do::Pass);
        assert_eq!(
            decide(Key::Left, &at("src/", 2), &ALL),
            Do::Pass,
            "a slash at the end but the caret is not"
        );
        let selected = Field { text: "src/".into(), caret: 0, selected: 4 };
        assert_eq!(decide(Key::Left, &selected, &ALL), Do::Pass);
        let top = Facts { up: false, ..ALL };
        assert_eq!(decide(Key::Left, &at("/", 1), &top), Do::Pass, "nothing above");
    }

    #[test]
    fn forget_is_only_for_a_row_that_can_be_forgotten() {
        assert_eq!(decide(Key::Forget, &at("a", 1), &ALL), Do::Forget);
        let none = Facts { forgettable: false, ..ALL };
        assert_eq!(decide(Key::Forget, &at("a", 1), &none), Do::Pass);
    }
}
