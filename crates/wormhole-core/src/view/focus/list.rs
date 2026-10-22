//! Roving focus in a list: the list is one Tab stop and keeps one focused item, which the
//! arrow keys move. A list may be a two-level tree, folded where it folds: the sidebar's bands and
//! their terminals, Changes' folders and files, History's commits and their files, the panel's
//! section bands and their items. The keys work as in Finder's list view, type-ahead included;
//! ↩ and Space press the focused item, and on a band only Space does anything.

use std::time::{Duration, Instant};

/// One item of a list as the keyboard walks it. Only items on screen are passed: the rows of a
/// folded band are left out by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    /// How deep it sits: 0 at the top, one more for each fold it is inside.
    pub level: u8,
    /// `Some(open)` for one that folds (a band, a folder, a commit).
    pub fold: Option<bool>,
}

/// A key that moves the focus in a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    /// ⌥↑
    First,
    /// ⌥↓
    Last,
}

/// A key that presses the item the focus is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// ↩: open it (a terminal, a diff, a next step to edit).
    Return,
    /// Space: show it while browsing, tick a next step, fold a note; elsewhere as ↩.
    Space,
}

/// Whether `press` folds or opens a band (the sidebar's folders, the panel's sections):
/// Space does, as on any disclosure; ↩ does nothing there, so it opens what an item stands for and
/// never folds a band away under the keys.
pub fn band_folds(press: Press) -> bool {
    press == Press::Space
}

/// What a key does to a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// The focus moves to this item.
    To(usize),
    /// Unfold this item; the focus stays on it.
    Open(usize),
    /// Fold this item; the focus stays on it.
    Fold(usize),
    /// Nothing to do: an end of the list, or a key that means nothing on this item.
    Stay,
}

/// What `key` does with the focus on `at`.
///
/// Up/Down don't wrap (as Finder's lists); from `None`, Down or First goes to the first item and
/// Up or Last to the last. Left on an open foldable folds it; on anything else goes to its parent
/// (the nearest item above with a lower level), else stays. Right on a folded foldable opens it; on
/// an open one goes to its first child when the next item is deeper, else stays; on a leaf stays.
/// An `at` past the end counts as the last item. Empty list: Stay.
pub fn walk(items: &[Item], at: Option<usize>, key: Key) -> Move {
    let Some(last) = items.len().checked_sub(1) else {
        return Move::Stay;
    };
    let Some(at) = at.map(|at| at.min(last)) else {
        return match key {
            Key::Down | Key::First => Move::To(0),
            Key::Up | Key::Last => Move::To(last),
            Key::Left | Key::Right => Move::Stay,
        };
    };
    let item = items[at];
    match key {
        Key::Up => at.checked_sub(1).map_or(Move::Stay, Move::To),
        Key::Down if at < last => Move::To(at + 1),
        Key::Down => Move::Stay,
        Key::First => Move::To(0),
        Key::Last => Move::To(last),
        Key::Left if item.fold == Some(true) => Move::Fold(at),
        Key::Left => {
            items[..at].iter().rposition(|above| above.level < item.level).map_or(Move::Stay, Move::To)
        }
        Key::Right => match item.fold {
            Some(false) => Move::Open(at),
            Some(true) if items.get(at + 1).is_some_and(|next| next.level > item.level) => Move::To(at + 1),
            _ => Move::Stay,
        },
    }
}

/// How long a pause between typed letters may be before type-ahead starts afresh.
pub const TYPE_AHEAD_WINDOW: Duration = Duration::from_secs(1);

/// What has been typed into a list, for type-ahead (a–z jump to the next name starting with
/// the typed letters).
#[derive(Debug, Default, Clone)]
pub struct TypeAhead {
    typed: String,
    /// When the last character came; the window runs from there, so a steady typist can spell out
    /// a long name.
    last: Option<Instant>,
}

impl TypeAhead {
    /// One more typed character at `now`; returns what has been typed so far (a pause longer than
    /// the window starts afresh).
    pub fn push(&mut self, ch: char, now: Instant) -> &str {
        if self.last.is_none_or(|last| now.saturating_duration_since(last) > TYPE_AHEAD_WINDOW) {
            self.typed.clear();
        }
        self.typed.push(ch);
        self.last = Some(now);
        &self.typed
    }

    /// Forget what was typed, when the focus moves some other way or leaves the list, so the next
    /// letter starts a new name.
    pub fn clear(&mut self) {
        self.typed.clear();
        self.last = None;
    }
}

/// The item to jump to for `typed`: the first whose name starts with it, ignoring case, looking from
/// the item after `at` and wrapping. With more than one character typed the search starts at `at`
/// itself, so a name that still matches keeps the focus; the same letter typed again and again
/// ("sss") steps through the names starting with it, as Finder does. `None` when nothing matches.
/// An `at` past the end counts as the last item.
pub fn type_ahead(names: &[&str], at: Option<usize>, typed: &str) -> Option<usize> {
    let mut chars = typed.chars();
    let first = chars.next()?;
    let last = names.len().checked_sub(1)?;
    let at = at.map(|at| at.min(last));
    // One letter, however often typed, is a step to the next name with it; anything longer is a
    // name being spelled out, which the focused item may already be.
    let (prefix, start) = if chars.all(|ch| same_letter(ch, first)) {
        (&typed[..first.len_utf8()], at.map_or(0, |at| at + 1))
    } else {
        (typed, at.unwrap_or(0))
    };
    (start..start + names.len()).map(|i| i % names.len()).find(|&i| starts_with_folded(names[i], prefix))
}

fn same_letter(a: char, b: char) -> bool {
    a.to_lowercase().eq(b.to_lowercase())
}

/// Whether `name` starts with `prefix`, ignoring case, without lowercasing either into a new string.
fn starts_with_folded(name: &str, prefix: &str) -> bool {
    let mut name = name.chars().flat_map(char::to_lowercase);
    prefix.chars().flat_map(char::to_lowercase).all(|p| name.next() == Some(p))
}

#[cfg(test)]
mod tests {
    use super::{Item, Key, Move, Press, TYPE_AHEAD_WINDOW, TypeAhead, band_folds, type_ahead, walk};
    use std::time::{Duration, Instant};

    const MS: Duration = Duration::from_millis(1);

    fn band(open: bool) -> Item {
        Item { level: 0, fold: Some(open) }
    }

    fn folder(level: u8, open: bool) -> Item {
        Item { level, fold: Some(open) }
    }

    fn leaf(level: u8) -> Item {
        Item { level, fold: None }
    }

    /// The sidebar: bands and their terminals; "web" is folded, so its rows aren't passed.
    fn sidebar() -> Vec<Item> {
        vec![
            band(true),  // 0 api
            leaf(1),     // 1   server
            leaf(1),     // 2   tests
            band(false), // 3 web (folded)
            band(true),  // 4 No folder
            leaf(1),     // 5   scratch
        ]
    }

    #[test]
    fn up_and_down_step_through_bands_and_terminals_without_wrapping() {
        let items = sidebar();
        let mut at = 0;
        let mut seen = vec![at];
        while let Move::To(next) = walk(&items, Some(at), Key::Down) {
            at = next;
            seen.push(at);
        }
        assert_eq!(seen, [0, 1, 2, 3, 4, 5], "the folded band is one stop, its rows skipped");
        assert_eq!(walk(&items, Some(5), Key::Down), Move::Stay, "no wrap at the bottom");
        assert_eq!(walk(&items, Some(4), Key::Up), Move::To(3));
        assert_eq!(walk(&items, Some(0), Key::Up), Move::Stay, "no wrap at the top");
    }

    #[test]
    fn left_and_right_fold_and_move_between_bands_and_terminals() {
        let items = sidebar();
        assert_eq!(walk(&items, Some(2), Key::Left), Move::To(0), "a terminal goes to its band");
        assert_eq!(walk(&items, Some(5), Key::Left), Move::To(4));
        assert_eq!(walk(&items, Some(0), Key::Left), Move::Fold(0), "an open band folds");
        assert_eq!(walk(&items, Some(3), Key::Left), Move::Stay, "a folded band has nowhere to go");
        assert_eq!(walk(&items, Some(3), Key::Right), Move::Open(3), "a folded band opens");
        assert_eq!(walk(&items, Some(0), Key::Right), Move::To(1), "an open band goes to its first terminal");
        assert_eq!(walk(&items, Some(1), Key::Right), Move::Stay, "a terminal has nothing inside");
        // An open band with no rows on screen (all of them filtered out).
        assert_eq!(walk(&[band(true), band(true)], Some(0), Key::Right), Move::Stay);
        // Without bands (Group by None) the terminals are all at the top.
        assert_eq!(walk(&[leaf(0), leaf(0)], Some(1), Key::Left), Move::Stay);
    }

    #[test]
    fn option_arrows_go_to_the_ends() {
        let items = sidebar();
        assert_eq!(walk(&items, Some(5), Key::First), Move::To(0));
        assert_eq!(walk(&items, Some(0), Key::Last), Move::To(5));
        assert_eq!(walk(&items, Some(2), Key::Last), Move::To(5));
    }

    #[test]
    fn from_no_focused_item_the_arrows_pick_an_end() {
        let items = sidebar();
        assert_eq!(walk(&items, None, Key::Down), Move::To(0));
        assert_eq!(walk(&items, None, Key::First), Move::To(0));
        assert_eq!(walk(&items, None, Key::Up), Move::To(5));
        assert_eq!(walk(&items, None, Key::Last), Move::To(5));
        assert_eq!(walk(&items, None, Key::Left), Move::Stay);
        assert_eq!(walk(&items, None, Key::Right), Move::Stay);
        for key in [Key::Up, Key::Down, Key::Left, Key::Right, Key::First, Key::Last] {
            assert_eq!(walk(&[], None, key), Move::Stay, "{key:?} in an empty list");
            assert_eq!(walk(&[], Some(3), key), Move::Stay, "{key:?} in an empty list");
        }
    }

    #[test]
    fn a_focus_past_the_end_counts_as_the_last_item() {
        // Rows went away under the focus.
        let items = sidebar();
        assert_eq!(walk(&items, Some(9), Key::Up), Move::To(4));
        assert_eq!(walk(&items, Some(9), Key::Down), Move::Stay);
        assert_eq!(walk(&items, Some(9), Key::Left), Move::To(4));
    }

    #[test]
    fn a_changes_tree_folds_folders_and_climbs_to_parents() {
        let items = [
            folder(0, true),  // 0 src/
            folder(1, true),  // 1   app/
            leaf(2),          // 2     main.rs
            leaf(2),          // 3     view.rs
            leaf(1),          // 4   lib.rs
            folder(0, false), // 5 tests/ (folded)
            leaf(0),          // 6 README.md
        ];
        assert_eq!(walk(&items, Some(3), Key::Left), Move::To(1), "a file goes to its folder");
        assert_eq!(walk(&items, Some(4), Key::Left), Move::To(0), "past the deeper files above it");
        assert_eq!(walk(&items, Some(1), Key::Left), Move::Fold(1), "an open folder folds first");
        assert_eq!(walk(&items, Some(0), Key::Right), Move::To(1), "into a folder");
        assert_eq!(walk(&items, Some(1), Key::Right), Move::To(2));
        assert_eq!(walk(&items, Some(5), Key::Right), Move::Open(5));
        assert_eq!(walk(&items, Some(6), Key::Right), Move::Stay);
        assert_eq!(walk(&items, Some(6), Key::Left), Move::Stay, "a file at the top has no parent");
        // Once app/ is folded, ← on it goes up to src/.
        let folded = [folder(0, true), folder(1, false), leaf(1)];
        assert_eq!(walk(&folded, Some(1), Key::Left), Move::To(0));
        assert_eq!(walk(&folded, Some(1), Key::Right), Move::Open(1));
    }

    #[test]
    fn history_opens_a_commit_to_its_files() {
        let items = [
            folder(0, true),  // 0 a commit, open
            leaf(1),          // 1   its file
            leaf(1),          // 2   its file
            folder(0, false), // 3 a commit
            folder(0, false), // 4 a commit
        ];
        assert_eq!(walk(&items, Some(3), Key::Right), Move::Open(3));
        assert_eq!(walk(&items, Some(0), Key::Right), Move::To(1));
        assert_eq!(walk(&items, Some(2), Key::Left), Move::To(0), "a file goes to its commit");
        assert_eq!(walk(&items, Some(0), Key::Left), Move::Fold(0));
        assert_eq!(walk(&items, Some(2), Key::Down), Move::To(3), "on to the next commit");
        assert_eq!(walk(&items, Some(4), Key::Left), Move::Stay);
    }

    /// On a section's band (Next steps, Notes, Changes, Untracked) Space folds and opens it; ↩
    /// does nothing there. → and ← fold it as before.
    #[test]
    fn only_space_folds_a_section_band() {
        assert!(band_folds(Press::Space));
        assert!(!band_folds(Press::Return));
        let bands = [band(true), leaf(1), band(false)];
        assert_eq!(walk(&bands, Some(0), Key::Left), Move::Fold(0));
        assert_eq!(walk(&bands, Some(2), Key::Right), Move::Open(2));
    }

    const NAMES: [&str; 4] = ["server", "Shell", "scratch", "API"];

    #[test]
    fn letters_typed_within_the_window_add_up_and_a_pause_starts_afresh() {
        let t0 = Instant::now();
        let mut typed = TypeAhead::default();
        assert_eq!(typed.push('s', t0), "s");
        assert_eq!(typed.push('e', t0 + 600 * MS), "se");
        let t1 = t0 + 600 * MS + TYPE_AHEAD_WINDOW;
        assert_eq!(typed.push('r', t1), "ser", "the window runs from the last letter");
        assert_eq!(typed.push('a', t1 + TYPE_AHEAD_WINDOW + MS), "a", "a longer pause starts afresh");
        typed.clear();
        assert_eq!(typed.push('b', t1 + TYPE_AHEAD_WINDOW + 2 * MS), "b");
    }

    #[test]
    fn type_ahead_ignores_case() {
        assert_eq!(type_ahead(&NAMES, None, "a"), Some(3));
        assert_eq!(type_ahead(&NAMES, None, "SH"), Some(1));
        assert_eq!(type_ahead(&NAMES, None, "sErV"), Some(0));
    }

    #[test]
    fn a_name_that_still_matches_keeps_the_focus() {
        assert_eq!(type_ahead(&NAMES, Some(0), "se"), Some(0));
        assert_eq!(type_ahead(&NAMES, Some(0), "sc"), Some(2));
        assert_eq!(type_ahead(&NAMES, Some(2), "se"), Some(0), "wrapping round to it");
        // Spelling out "sc" from "server": "s" moves on to the next s-name, "sc" finds its own.
        let t0 = Instant::now();
        let mut typed = TypeAhead::default();
        let at = type_ahead(&NAMES, Some(0), typed.push('s', t0));
        assert_eq!(at, Some(1));
        assert_eq!(type_ahead(&NAMES, at, typed.push('c', t0 + 200 * MS)), Some(2));
    }

    #[test]
    fn the_same_letter_again_steps_through_its_names() {
        assert_eq!(type_ahead(&NAMES, None, "s"), Some(0));
        assert_eq!(type_ahead(&NAMES, Some(0), "ss"), Some(1));
        assert_eq!(type_ahead(&NAMES, Some(1), "sss"), Some(2));
        assert_eq!(type_ahead(&NAMES, Some(2), "ssss"), Some(0), "round again, past API");
        assert_eq!(type_ahead(&NAMES, Some(0), "sS"), Some(1), "in either case");
        assert_eq!(type_ahead(&NAMES, Some(0), "s"), Some(1), "one letter looks past the focused name");
    }

    #[test]
    fn type_ahead_wraps_and_may_find_nothing() {
        assert_eq!(type_ahead(&NAMES, Some(3), "s"), Some(0));
        assert_eq!(type_ahead(&NAMES, Some(3), "a"), Some(3), "the only a-name is the focused one");
        assert_eq!(type_ahead(&NAMES, Some(9), "s"), Some(0), "a focus past the end is the last item");
        assert_eq!(type_ahead(&NAMES, Some(1), "z"), None);
        assert_eq!(type_ahead(&NAMES, Some(1), "sx"), None);
        assert_eq!(type_ahead(&NAMES, Some(1), ""), None);
        assert_eq!(type_ahead(&[], None, "s"), None);
    }
}
