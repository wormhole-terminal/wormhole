//! Moving one item of a list the user orders by hand (a session's tasks, its notes), as
//! drag and drop and ⌥⌘↑ / ⌥⌘↓ do. A list's order is all there is to save: the store
//! writes each item's position from it, and `notes.md` lists the items in it.

/// Which edge of the item under the pointer a dragged item would land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropEdge {
    Above,
    Below,
}

/// Move `items[from]` so that it ends up at index `to` (an index into the list after the
/// move, so dropping onto item `to` puts the dragged item exactly where that one was).
/// Returns whether anything moved: an index past the end or a move onto itself doesn't.
pub fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from == to || from >= items.len() || to >= items.len() {
        return false;
    }
    let item = items.remove(from);
    items.insert(to, item);
    true
}

/// Where the drop indicator goes while item `from` is dragged over item `target`: dragging
/// down lands below the target, dragging up above it (matching [`move_item`]), and an item
/// over itself lands nowhere.
pub fn drop_edge(from: usize, target: usize) -> Option<DropEdge> {
    match from.cmp(&target) {
        std::cmp::Ordering::Less => Some(DropEdge::Below),
        std::cmp::Ordering::Greater => Some(DropEdge::Above),
        std::cmp::Ordering::Equal => None,
    }
}

/// Where item `ix` of `len` goes one step up (`delta < 0`) or down; `None` at either end,
/// where a keyboard move stops rather than wrapping around.
pub fn step(ix: usize, delta: isize, len: usize) -> Option<usize> {
    let to = ix.checked_add_signed(delta)?;
    (to < len && ix < len && to != ix).then_some(to)
}

/// Where an item that was at `ix` is after [`move_item`] moved `from` to `to`: keeps an
/// index that points at an item (the one being edited, the focused note) on that item.
pub fn follow(ix: usize, from: usize, to: usize) -> usize {
    if ix == from {
        to
    } else if from < ix && ix <= to {
        ix - 1
    } else if to <= ix && ix < from {
        ix + 1
    } else {
        ix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved(from: usize, to: usize) -> Vec<char> {
        let mut items = vec!['a', 'b', 'c', 'd'];
        move_item(&mut items, from, to);
        items
    }

    #[test]
    fn moving_down_and_up_puts_the_item_where_the_target_was() {
        assert_eq!(moved(0, 2), ['b', 'c', 'a', 'd']);
        assert_eq!(moved(3, 1), ['a', 'd', 'b', 'c']);
        assert_eq!(moved(0, 3), ['b', 'c', 'd', 'a']);
        assert_eq!(moved(3, 0), ['d', 'a', 'b', 'c']);
        assert_eq!(moved(1, 2), ['a', 'c', 'b', 'd'], "neighbours swap");
    }

    #[test]
    fn moves_that_go_nowhere_change_nothing() {
        let mut items = vec![1, 2, 3];
        assert!(!move_item(&mut items, 1, 1));
        assert!(!move_item(&mut items, 3, 0), "from past the end");
        assert!(!move_item(&mut items, 0, 3), "to past the end");
        assert!(!move_item(&mut Vec::<u8>::new(), 0, 0));
        assert_eq!(items, [1, 2, 3]);
        assert!(move_item(&mut items, 2, 0));
        assert_eq!(items, [3, 1, 2]);
    }

    #[test]
    fn the_indicator_sits_where_the_item_will_land() {
        assert_eq!(drop_edge(0, 2), Some(DropEdge::Below));
        assert_eq!(drop_edge(3, 1), Some(DropEdge::Above));
        assert_eq!(drop_edge(2, 2), None);
        // Dropping below item 2 while dragging down: item 0 ends up at index 2, just under
        // what was item 2 before the drag started.
        assert_eq!(moved(0, 2)[1], 'c');
    }

    #[test]
    fn keyboard_steps_stop_at_the_ends() {
        assert_eq!(step(1, -1, 3), Some(0));
        assert_eq!(step(1, 1, 3), Some(2));
        assert_eq!(step(0, -1, 3), None);
        assert_eq!(step(2, 1, 3), None);
        assert_eq!(step(0, 1, 1), None);
        assert_eq!(step(5, -1, 3), None, "an index past the end doesn't move");
    }

    #[test]
    fn follow_keeps_an_index_on_its_item() {
        for (from, to) in [(0, 3), (3, 0), (1, 2), (2, 1), (1, 1)] {
            let mut items = vec!['a', 'b', 'c', 'd'];
            let before = items.clone();
            move_item(&mut items, from, to);
            for (ix, item) in before.iter().enumerate() {
                assert_eq!(items[follow(ix, from, to)], *item, "move {from}->{to}, item {ix}");
            }
        }
    }
}
