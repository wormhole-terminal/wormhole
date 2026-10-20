//! What the walk of vttest does beyond choosing every entry once: how the work is split, the entries
//! it leaves out, the order it runs some in, the keys it types where vttest waits for a person.
//!
//! An entry is left out only when it needs a person (typed keys, a mouse, a window of its own) or
//! when it is the same test as one that runs elsewhere (vttest's later-model menus list the
//! earlier ones' as their first entry, and several tests hang from more than one menu). Each
//! reason is written to `notes.txt`, so what is not measured is on record. A path is the menu
//! numbers from the main menu down to the entry, as vttest numbers them (11.3.7.3.9).

use std::time::Duration;

use wormhole_vt::input::{Key, KeypadKey};

/// What the host types where vttest waits for a key.
#[derive(Clone, Copy)]
pub enum Answer {
    /// A key, as the engine's keyboard encoder makes it under the terminal's modes now.
    Key(Key),
    /// These bytes.
    Text(&'static str),
}

const KEYPAD_5: Key = Key::Keypad(KeypadKey::Digit(5));

const TYPING: &str = "it needs a person at the keyboard (the test reads the keys pressed)";

/// The folder of the top-level menu `number`: its number and a short name.
pub fn menu_dir(number: u32) -> String {
    let name = match number {
        1 => "cursor-movements",
        2 => "screen-features",
        3 => "character-sets",
        4 => "double-sized",
        5 => "keyboard",
        6 => "reports",
        7 => "vt52",
        8 => "insert-delete",
        9 => "known-bugs",
        10 => "reset",
        11 => "non-vt100",
        _ => "other",
    };
    format!("{number:02}-{name}")
}

/// The parts a top-level menu is walked in, each in a vttest of its own, a few at once: the
/// non-VT100 menu has hundreds of screens, and most of the time is vttest pausing for answers.
/// The parts' screens are numbered in this order. Together they must cover the menu: `--only 11`
/// walks it whole in one part (two minutes) to compare with.
pub fn units(top: u32) -> Vec<Vec<u32>> {
    // The parts below `prefix` numbered `first` to `last`.
    fn parts(prefix: &[u32], first: u32, last: u32) -> impl Iterator<Item = Vec<u32>> + '_ {
        (first..=last).map(move |n| [prefix, &[n]].concat())
    }
    match top {
        11 => {
            let mut units = vec![vec![11, 1]];
            units.extend(parts(&[11, 2], 1, 4));
            units.extend(parts(&[11, 2, 5], 1, 2));
            units.extend(parts(&[11, 2, 5, 3], 1, 2));
            units.extend(parts(&[11, 2, 5, 3, 3], 1, 2));
            units.extend(parts(&[11, 2, 5, 3], 4, 6));
            units.extend(parts(&[11, 2, 5], 4, 6));
            units.push(vec![11, 2, 6]);
            units.extend(parts(&[11, 3], 1, 6));
            units.extend(parts(&[11, 3, 7], 1, 1));
            units.extend(parts(&[11, 3, 7, 2], 1, 1));
            units.extend(parts(&[11, 3, 7, 2, 2], 1, 2));
            units.extend(parts(&[11, 3, 7, 2], 3, 3));
            units.extend(parts(&[11, 3, 7, 3], 1, 11));
            units.push(vec![11, 3, 8]);
            units.extend(parts(&[11, 4], 1, 4));
            units.push(vec![11, 4, 5]);
            units.push(vec![11, 4, 6]);
            units.extend(parts(&[11], 5, 7));
            units.extend(parts(&[11, 8], 1, 9));
            units
        }
        _ => vec![vec![top]],
    }
}

/// Whether the part takes much longer than the others (the walk starts those first).
pub fn slow(unit: &[u32]) -> bool {
    // The checksums of every character on the screen, one request each; a report for every mode.
    matches!(
        unit,
        [11, 3, 7, 3, 9 | 10]
            | [11, 2, 5, 3, 3, _]
            | [11, 3, 7, 2, 2, _]
            | [11, 3, 4]
            | [11, 8, 2]
            | [11, 8, 8]
    )
}

/// Why the entry at `path` (named `name` on vttest's menu) is not run, if it is not.
pub fn skip(path: &[u32], name: &str) -> Option<&'static str> {
    if name.starts_with("Specify G") {
        return Some(
            "it picks one of 25 character sets for the tests that follow, which run with the defaults",
        );
    }
    if name.ends_with("color-switching") {
        return Some(
            "it turns the colours off for the tests after it, which run with them on as vttest starts",
        );
    }
    match name {
        "Test Keyboard Layout with G0 Selection" => return Some(TYPING),
        "Test Send/Receive mode (SRM)" => return Some("it waits for keys to be pressed to show local echo"),
        // The later models' menus list the earlier ones' as their first entry.
        "Test VT220 features" => return Some("the same menu as 11.1, which ran"),
        "Test VT320 features" => return Some("the same menu as 11.2, which ran"),
        "Test VT420 features" => return Some("the same menu as 11.3, which ran"),
        "Test VT520 features" => return Some("the same menu as 11.4, which ran"),
        "Test VT320 features (DECRQSS)"
        | "Test VT420 features (DECRQSS)"
        | "Test VT510 features (DECRQSS)" => {
            return Some("the same menu as the earlier model's, which ran");
        }
        "Test VT520 features (DECRQSS)" => return Some("the same menu as 11.4.5.2.3, which ran"),
        "Test character sets" => return Some("the same menu as 3, which ran"),
        _ => {}
    }
    match path {
        [5, 2..=9] => Some(TYPING),
        // The VT520's reports list the VT420's tests again, under the same names.
        [11, 4, 5, 3] => Some("the same menu as 11.3.7.3, which ran"),
        [11, 4, 5, 2, 2] => Some("the same test as 11.3.7.2.2, which ran"),
        [11, 8, 5, 3..=8] => Some("it waits for mouse clicks"),
        [11, 8, 6, _] => Some(
            "Tektronix 4014 mode (DECSET 38) is xterm's second window, not a screen the engine has; the tests then wait for keys in it",
        ),
        _ => None,
    }
}

/// The entries of the menu at `path` to run, in order (a number may come twice), when that is not
/// each entry once.
pub fn order(path: &[u32]) -> Option<&'static [u32]> {
    match path {
        // The character set tests (8 to 11) as vttest starts them, then with 2 (highlighting of what
        // is not ISO 8859-1) and 3 (national replacement sets) switched on.
        [3] => Some(&[8, 9, 10, 11, 2, 8, 9, 10, 11, 3, 8, 9, 10, 11]),
        // The VT420 menus that begin with switches (origin mode, left and right margins mode, the
        // margins): the tests as vttest starts them, then with the switches on.
        [11, 3, 2] => Some(&[7, 8, 9, 10, 2, 3, 4, 5, 7, 8, 9, 10]),
        [11, 3, 3] => Some(&[6, 7, 8, 9, 10, 11, 12, 13, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13]),
        [11, 3, 6] => Some(&[7, 8, 9, 10, 11, 12, 1, 2, 3, 4, 7, 8, 9, 10, 11, 12]),
        [11, 4, 2] => Some(&[7, 8, 9, 10, 11, 12, 13, 14, 15, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        // The cursor report with and without national replacement sets; the tab stop report with the
        // tabs as they are, cleared, and set every eight.
        [11, 2, 5, 3, 1] => Some(&[7, 2, 7]),
        [11, 2, 5, 3, 2] => Some(&[3, 1, 3, 2, 3]),
        _ => None,
    }
}

/// How long vttest may be silent while the entry at `path` runs before the screen counts as
/// waiting: the entries that sleep get longer.
pub fn quiet(path: &[u32]) -> Duration {
    // Typing answers into the test: vttest redraws at once.
    if !answers(path).is_empty() {
        return Duration::from_millis(400);
    }
    match path {
        // RIS and DECTST: vttest sleeps five seconds (a quarter of that in the quick build) after them.
        [10, 1] | [10, 2] => Duration::from_millis(2500),
        // Window modify-operations: sleep(2) and sleep(1), whole seconds, before the first screen.
        [11, 8, 8] => Duration::from_millis(4500),
        _ => Duration::from_millis(1500),
    }
}

/// The keys to type, one per silence, while the entry at `path` runs.
pub fn answers(path: &[u32]) -> &'static [Answer] {
    match path {
        // LNM: "push the RETURN key" once with the mode set and once reset.
        [6, 2] => &[Answer::Key(Key::Enter), Answer::Key(Key::Enter)],
        // Bug D asks whether to try again.
        [9, 4] => &[Answer::Text("0\n")],
        // DECBKM: the backspace key with the mode set and reset.
        [11, 3, 4, 1] => &[Answer::Key(Key::Backspace), Answer::Key(Key::Backspace)],
        // DECNKM: a keypad key, twice to finish each of the two passes.
        [11, 3, 4, 2] => &[Answer::Key(KEYPAD_5); 4],
        // DECKBUM and DECKPM: the same with a letter.
        [11, 3, 4, 3] | [11, 3, 4, 4] => &[Answer::Key(Key::Char('a')); 4],
        // Set window title asks for the title as a line.
        [11, 8, 3] => &[Answer::Text("a window title\n")],
        // Modify font asks for a font name as a line.
        [11, 8, 4, 1] => &[Answer::Text("fixed\n")],
        // DECUDK waits for keys until 'q'.
        [11, 1, 7] => &[Answer::Text("q")],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::{menu_dir, skip, units};

    #[test]
    fn every_part_of_a_menu_is_walked_once_and_inside_its_menu() {
        for top in 1..=11 {
            let parts = units(top);
            assert!(parts.iter().all(|p| p.first() == Some(&top)), "menu {top}: {parts:?}");
            let mut sorted = parts.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), parts.len(), "menu {top} lists a part twice");
        }
    }

    #[test]
    fn no_part_lies_inside_another() {
        // A part inside another would be walked twice, and its screens kept twice.
        let parts = units(11);
        for (i, a) in parts.iter().enumerate() {
            for b in &parts[i + 1..] {
                assert!(!a.starts_with(b) && !b.starts_with(a), "{a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn menu_folders_are_numbered_so_they_sort_in_vttests_order() {
        let dirs: Vec<String> = (1..=11).map(menu_dir).collect();
        let mut sorted = dirs.clone();
        sorted.sort();
        assert_eq!(dirs, sorted);
    }

    #[test]
    fn what_is_left_out_is_named_with_a_reason() {
        assert!(skip(&[5, 4], "Cursor Keys").is_some_and(|reason| reason.contains("keyboard")));
        assert!(skip(&[5, 1], "LED Lights").is_none());
        assert!(skip(&[11, 3, 1], "Test VT320 features").is_some());
    }
}
