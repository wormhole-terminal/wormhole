//! The order of the terminal list: the user's (manual) order, pins on top of it, moving a
//! terminal by drag and drop, and what ⌘1–9 count along.
//!
//! The manual order is the order the window keeps its sessions in (and saves them in). A
//! group is a folder band, or the whole list under "No grouping"; pinned terminals stay at
//! the top of their group and keep their manual order among themselves. A drag moves a
//! terminal inside its group, and inside its pinned or unpinned block of it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How the terminal list keeps its order (Settings → Terminals & sidebar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarOrder {
    /// Pinned terminals first, then first created to last; under "By folder" the bands keep the
    /// order they appeared in. Nothing moves when a terminal is created, changes folder or status.
    #[default]
    Stable,
    /// Up to 0.4: a split pulls its new terminal next to it, and the bands are ordered by their
    /// first terminal as things are now, so a terminal that changes folder can move other bands.
    Classic,
}

impl SidebarOrder {
    pub const ALL: [SidebarOrder; 2] = [SidebarOrder::Stable, SidebarOrder::Classic];
}

/// What ⌘1–9 count along (Settings → Terminals & sidebar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarNumbers {
    /// A terminal keeps its number: they are dealt top to bottom and frozen (`FrozenNumbers`), so a
    /// new terminal takes the next free one and nothing the list does on its own renumbers anyone.
    #[default]
    Terminal,
    /// Up to 0.4: the first nine rows you can see, top to bottom, so a number follows the list.
    List,
}

impl SidebarNumbers {
    pub const ALL: [SidebarNumbers; 2] = [SidebarNumbers::Terminal, SidebarNumbers::List];
}

/// The ⌘ numbers when they stay with the terminal: dealt top to bottom as the list stands, then
/// frozen. A terminal made later takes the next free number and keeps it (even in a band in the
/// middle); a folder change, a status change, the Show filter, a fold or a split's gathering never
/// renumber anyone, and a hidden terminal keeps its number. They are dealt again, top to bottom,
/// only when the person changes the order (a pin, an unpin, a drag), a terminal is closed (so
/// there is no gap), or a setting that orders the list changes.
#[derive(Clone, Debug)]
pub struct FrozenNumbers<K> {
    /// Who holds ⌘1, ⌘2 … in that order.
    held: Vec<K>,
    /// The window's terminals and their pins when last settled, to tell a new terminal from a
    /// changed order.
    seen: Option<Vec<(K, bool)>>,
    /// What the settings that order the list said then.
    tag: u8,
}

impl<K> Default for FrozenNumbers<K> {
    fn default() -> Self {
        FrozenNumbers { held: Vec::new(), seen: None, tag: 0 }
    }
}

impl<K: Clone + PartialEq> FrozenNumbers<K> {
    /// The holders of ⌘1… now. `window` is every terminal in the window's order with whether it is
    /// pinned, `tag` stands for the settings that order the list (grouping, order, numbers) and
    /// `dealt` is every terminal top to bottom as the list stands (used when the numbers are dealt).
    pub fn settle(&mut self, window: &[(K, bool)], tag: u8, dealt: &[K], count: usize) -> Vec<K> {
        let unchanged_prefix = |seen: &Vec<(K, bool)>| {
            seen.len() <= window.len() && seen.iter().zip(window).all(|(before, now)| before == now)
        };
        match &self.seen {
            Some(seen) if self.tag == tag && unchanged_prefix(seen) => {
                // Only terminals made since, at the end: the next free numbers.
                let made = &window[seen.len()..];
                for (key, _) in made {
                    if self.held.len() < count {
                        self.held.push(key.clone());
                    }
                }
            }
            _ => self.held = dealt.iter().take(count).cloned().collect(),
        }
        self.seen = Some(window.to_vec());
        self.tag = tag;
        self.held.clone()
    }

    /// Forget the numbers: the next settle deals them again.
    pub fn reset(&mut self) {
        self.seen = None;
        self.held.clear();
    }
}

/// The order folder bands keep, given the order they had (`previous`) and the folder each
/// terminal is in now: a band stays where it was while it has a terminal, a new one goes last
/// (the first-seen order among new ones), one with no terminal left is forgotten. Terminals
/// with no known folder have no band of their own here (they are last, always).
pub fn settle_bands(previous: &[PathBuf], folders: &[Option<PathBuf>]) -> Vec<PathBuf> {
    let mut settled: Vec<PathBuf> = Vec::new();
    for folder in previous {
        if folders.iter().flatten().any(|present| present == folder) && !settled.contains(folder) {
            settled.push(folder.clone());
        }
    }
    for folder in folders.iter().flatten() {
        if !settled.contains(folder) {
            settled.push(folder.clone());
        }
    }
    settled
}

/// The session ⌘`n` (1-based) selects among `numbered`, the sessions that carry a number, top to
/// bottom: the `n`-th, or the last one when there are fewer (⌘9 is "the last", as in a browser).
pub fn shortcut_target(numbered: &[usize], n: usize) -> Option<usize> {
    numbered.get(n.checked_sub(1)?).or(numbered.last()).copied()
}

/// `order` with the pinned sessions first, each block keeping its order. The single place
/// pins are applied.
pub fn with_pins(order: &[usize], pinned: impl Fn(usize) -> bool) -> Vec<usize> {
    let (mut first, rest): (Vec<usize>, Vec<usize>) = order.iter().partition(|&&ix| pinned(ix));
    first.extend(rest);
    first
}

/// Where a terminal dragged from position `from` of its group lands when dropped on the row at
/// position `onto` (positions in the group as listed, pins first; `pinned[i]` says whether the
/// one at position `i` is pinned). A drop outside the dragged terminal's own block lands at the
/// nearest end of that block: an unpinned terminal dropped on a pinned one lands just below the
/// pinned block. `None` when nothing would move.
pub fn drop_position(pinned: &[bool], from: usize, onto: usize) -> Option<usize> {
    if from >= pinned.len() || onto >= pinned.len() {
        return None;
    }
    let block = pinned[from];
    let first = pinned.iter().position(|&p| p == block)?;
    let last = pinned.iter().rposition(|&p| p == block)?;
    let to = onto.clamp(first, last);
    (to != from).then_some(to)
}

/// The dragged terminal's block (pinned or not) of a group listed as `members`, in its new order
/// after the terminal at position `from` moved to position `to` ([`drop_position`]). Only this
/// block is handed to [`regroup`]: the other block keeps its places, so a pinned terminal still
/// goes back to its own place when it is unpinned.
pub fn move_in_block(members: &[usize], pinned: &[bool], from: usize, to: usize) -> Vec<usize> {
    let mut moved = members.to_vec();
    if from < moved.len() && to < moved.len() {
        let item = moved.remove(from);
        moved.insert(to, item);
    }
    let block = pinned.get(from).copied().unwrap_or(false);
    // Positions inside one block keep their pinned-ness after the move (a drag never leaves it).
    moved.iter().zip(pinned).filter(|&(_, &p)| p == block).map(|(&ix, _)| ix).collect()
}

/// The window's new manual order after a group's sessions were reordered to `members` (session
/// indices, in their new order): a permutation where entry `k` is the session that goes to place
/// `k`. The group keeps the places it had, so every other session, and the order of the bands
/// (the place of each folder's first terminal), stays as it was.
pub fn regroup(len: usize, members: &[usize]) -> Vec<usize> {
    let mut places: Vec<usize> = members.to_vec();
    places.sort_unstable();
    let mut order: Vec<usize> = (0..len).collect();
    for (&place, &session) in places.iter().zip(members) {
        if place < len {
            order[place] = session;
        }
    }
    order
}

/// Where `ix` is after [`regroup`] gave `order`: keeps an index on its session (the selected one).
pub fn follow(order: &[usize], ix: usize) -> usize {
    order.iter().position(|&session| session == ix).unwrap_or(ix)
}

#[cfg(test)]
mod tests {
    use super::{
        FrozenNumbers, drop_position, follow, move_in_block, regroup, settle_bands, shortcut_target,
        with_pins,
    };
    use std::path::PathBuf;

    #[test]
    fn command_digits_count_down_the_list_and_past_the_end_mean_the_last() {
        let listed = [2, 0, 1];
        assert_eq!(shortcut_target(&listed, 1), Some(2));
        assert_eq!(shortcut_target(&listed, 3), Some(1));
        assert_eq!(shortcut_target(&listed, 9), Some(1), "⌘9 with three terminals is the last one");
        assert_eq!(shortcut_target(&listed, 0), None);
        assert_eq!(shortcut_target(&[], 1), None);
    }

    #[test]
    fn pinned_terminals_go_first_and_each_block_keeps_its_order() {
        let pinned = |ix: usize| ix == 3 || ix == 1;
        assert_eq!(with_pins(&[0, 1, 2, 3, 4], pinned), [1, 3, 0, 2, 4]);
        assert_eq!(with_pins(&[4, 3, 2, 1, 0], pinned), [3, 1, 4, 2, 0], "manual order among the pinned");
        assert_eq!(with_pins(&[2, 0], |_| false), [2, 0], "none pinned");
        assert_eq!(with_pins(&[2, 0], |_| true), [2, 0], "all pinned");
        assert!(with_pins(&[], |_| true).is_empty());
    }

    #[test]
    fn a_drag_stays_inside_its_pinned_or_unpinned_block() {
        // Two pinned, then three not.
        let pinned = [true, true, false, false, false];
        assert_eq!(drop_position(&pinned, 4, 2), Some(2), "within the unpinned block");
        assert_eq!(drop_position(&pinned, 4, 0), Some(2), "above a pinned one: just below the pins");
        assert_eq!(drop_position(&pinned, 0, 4), Some(1), "a pinned one stays among the pins");
        assert_eq!(drop_position(&pinned, 1, 0), Some(0));
        assert_eq!(drop_position(&pinned, 2, 1), None, "already the first unpinned one");
        assert_eq!(drop_position(&pinned, 3, 3), None, "onto itself");
        assert_eq!(drop_position(&pinned, 9, 0), None, "out of range");
        assert_eq!(drop_position(&[false, false], 0, 1), Some(1), "nothing pinned");
        assert_eq!(drop_position(&[true, true], 1, 0), Some(0), "everything pinned");
    }

    #[test]
    fn a_drag_rewrites_only_its_own_block_so_unpinning_restores_a_place() {
        // A flat list of four, the last pinned: listed 3, 0, 1, 2. Drag 2 above 0.
        let members = [3, 0, 1, 2];
        let pinned = [true, false, false, false];
        let to = drop_position(&pinned, 3, 1).expect("lands above 0");
        let order = regroup(4, &move_in_block(&members, &pinned, 3, to));
        assert_eq!(order, [2, 0, 1, 3], "the pinned terminal keeps its own place");
        // Unpinned again, it is back where it was: last.
        assert_eq!(with_pins(&order, |_| false).last(), Some(&3));
        // A pinned terminal dragged within the pins moves only among them.
        let members = [2, 3, 0, 1];
        let pinned = [true, true, false, false];
        assert_eq!(move_in_block(&members, &pinned, 1, 0), [3, 2]);
    }

    #[test]
    fn reordering_a_folder_keeps_its_places_and_everyone_else() {
        // Sessions 0, 2 and 4 are one folder, 1 and 3 another; 4 is dragged to the top of its folder.
        let order = regroup(5, &[4, 0, 2]);
        assert_eq!(order, [4, 1, 0, 3, 2]);
        // The other folder and the folders' order (whose first terminal comes first) don't move.
        assert_eq!(order.iter().filter(|&&s| s == 1 || s == 3).copied().collect::<Vec<_>>(), [1, 3]);
        // The selection follows its session.
        assert_eq!(follow(&order, 4), 0);
        assert_eq!(follow(&order, 1), 1);
        assert_eq!(follow(&order, 2), 4);
        // The flat list is one group with every place.
        assert_eq!(regroup(3, &[2, 0, 1]), [2, 0, 1]);
        assert_eq!(regroup(3, &[]), [0, 1, 2]);
    }

    #[test]
    fn numbers_are_dealt_top_to_bottom_then_frozen_until_the_order_changes() {
        // Window order 1..=4, shown 1, 3, 2, 4 (two bands); nobody pinned.
        let window = |ids: &[i32]| ids.iter().map(|&id| (id, false)).collect::<Vec<_>>();
        let mut numbers = FrozenNumbers::default();
        assert_eq!(numbers.settle(&window(&[1, 2, 3, 4]), 0, &[1, 3, 2, 4], 9), [1, 3, 2, 4]);
        // A terminal made later takes the next free number, wherever the list puts it.
        assert_eq!(numbers.settle(&window(&[1, 2, 3, 4, 5]), 0, &[1, 5, 3, 2, 4], 9), [1, 3, 2, 4, 5]);
        // The list moving on its own (a folder change) renumbers nobody.
        assert_eq!(numbers.settle(&window(&[1, 2, 3, 4, 5]), 0, &[5, 4, 3, 2, 1], 9), [1, 3, 2, 4, 5]);
        // A pin is the person's: dealt again from the list as it stands.
        let pinned = [(1, false), (2, false), (3, false), (4, false), (5, true)];
        assert_eq!(numbers.settle(&pinned, 0, &[5, 1, 3, 2, 4], 9), [5, 1, 3, 2, 4]);
        // So is a drag, which changes the window's order ...
        assert_eq!(numbers.settle(&window(&[2, 1, 3, 4, 5]), 0, &[2, 1, 3, 4, 5], 9), [2, 1, 3, 4, 5]);
        // ... and a close, so no number is left empty.
        assert_eq!(numbers.settle(&window(&[2, 3, 4, 5]), 0, &[2, 3, 4, 5], 9), [2, 3, 4, 5]);
        // A setting that orders the list.
        assert_eq!(numbers.settle(&window(&[2, 3, 4, 5]), 1, &[5, 4, 3, 2], 9), [5, 4, 3, 2]);
        numbers.reset();
        assert_eq!(numbers.settle(&window(&[2, 3, 4, 5]), 1, &[3, 2, 4, 5], 9), [3, 2, 4, 5]);
    }

    #[test]
    fn only_the_first_nine_carry_a_number_and_a_new_one_past_them_has_none() {
        let window = |n: i32| (0..n).map(|id| (id, false)).collect::<Vec<_>>();
        let order = |n: i32| (0..n).collect::<Vec<_>>();
        let mut numbers = FrozenNumbers::default();
        assert_eq!(numbers.settle(&window(8), 0, &order(8), 9).len(), 8);
        assert_eq!(numbers.settle(&window(10), 0, &order(10), 9), order(9), "the ninth, not the tenth");
        assert_eq!(numbers.settle(&window(11), 0, &order(11), 9).len(), 9);
    }

    #[test]
    fn bands_keep_their_order_while_terminals_come_and_go() {
        let dirs = |paths: &[Option<&str>]| -> Vec<Option<PathBuf>> {
            paths.iter().map(|p| p.map(PathBuf::from)).collect()
        };
        let paths = |paths: &[&str]| -> Vec<PathBuf> { paths.iter().map(PathBuf::from).collect() };
        let first = settle_bands(&[], &dirs(&[Some("/w/b"), Some("/w/a"), Some("/w/b")]));
        assert_eq!(first, paths(&["/w/b", "/w/a"]), "the first time: as first seen");
        // The first terminal of /w/b goes to /w/c: /w/b keeps its place above /w/a, /w/c is last.
        let moved = settle_bands(&first, &dirs(&[Some("/w/c"), Some("/w/a"), Some("/w/b")]));
        assert_eq!(moved, paths(&["/w/b", "/w/a", "/w/c"]));
        // A band nobody is in is forgotten, and comes back last.
        let gone = settle_bands(&moved, &dirs(&[Some("/w/c"), Some("/w/a")]));
        assert_eq!(gone, paths(&["/w/a", "/w/c"]));
        let back = settle_bands(&gone, &dirs(&[Some("/w/b"), Some("/w/c"), Some("/w/a")]));
        assert_eq!(back, paths(&["/w/a", "/w/c", "/w/b"]));
        assert!(settle_bands(&first, &dirs(&[None])).is_empty());
    }
}
