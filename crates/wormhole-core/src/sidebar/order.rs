//! The order of the terminal list: the user's (manual) order, pins on top of it, moving a
//! terminal by drag and drop, and what ⌘1–9 count along.
//!
//! The manual order is the order the window keeps its sessions in (and saves them in). A
//! group is a folder band, or the whole list under "No grouping"; pinned terminals stay at
//! the top of their group and keep their manual order among themselves. A drag moves a
//! terminal inside its group, and inside its pinned or unpinned block of it.

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
    use super::{drop_position, follow, move_in_block, regroup, shortcut_target, with_pins};

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
}
