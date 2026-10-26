//! What dropping a row on another row of the terminal list does. The middle of a row joins the two
//! in a split; the top and bottom quarters of a row reorder, before or after it. A row dragged out
//! of its split's bracket leaves the split, and one dropped between two rows of another split lands
//! outside that bracket: a drop never ends up inside a split by accident.
//!
//! A drag inside a bracket reorders the sidebar only, never the window's layout. Which pane a row
//! joins, and where, is `panes`' (`TabPanes::join`, which can refuse for want of room).

use super::split::{SplitId, split_of};

/// Where on the target row the drop is: the top or bottom quarter of it, or the middle half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Top,
    Middle,
    Bottom,
}

impl Part {
    /// The part of a line a pointer `fraction` of the way down it (0.0 its top edge, 1.0 its bottom) is
    /// over: the top quarter, the bottom quarter, or the middle half between them.
    #[must_use]
    pub fn at(fraction: f32) -> Self {
        if fraction < 0.25 {
            Self::Top
        } else if fraction > 0.75 {
            Self::Bottom
        } else {
            Self::Middle
        }
    }
}

/// What a drop does. `to` is where the dragged row is in `order` afterwards: take it out, put it
/// back at index `to` (as `order::move_in_block` does). Keeping a pinned terminal among the pinned
/// is not decided here: the caller clamps `to` to the block as `order::drop_position` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowDrop {
    /// The row moves, and stays in its split if it has one: it was dropped on a row of its own
    /// split, which only changes the sidebar's order, or it is in none.
    Reorder { to: usize },
    /// The dragged terminal joins `target`'s split, or makes one with it, leaving its own split if
    /// it had one. It lands right after the bracket it joins.
    Join { target: usize, to: usize },
    /// The dragged terminal leaves its split and lands at `to`, outside the bracket it was in.
    Leave { to: usize },
}

/// Resolve the drop of `dragged` on `target`, both terminals of `order` (a band's terminals as
/// listed, or the whole flat list), `ids` saying who is in which split. `None` when nothing would
/// change: the terminal dropped on itself or into the place it has (a member of a split dropped
/// outside it still leaves, wherever it lands), a row dropped on the middle of one it is in a split
/// with, or a terminal that isn't in `order`.
pub fn resolve_row_drop(
    order: &[usize],
    ids: &[Option<SplitId>],
    dragged: usize,
    target: usize,
    part: Part,
) -> Option<RowDrop> {
    resolve_drop(order, ids, dragged, target, part, false)
}

/// [`resolve_row_drop`], `across` saying the drop moves the row to another group even where it
/// keeps its place in the list (the first row under a group's head dropped on its top): then a
/// move that leaves every row where it is still counts.
pub(super) fn resolve_drop(
    order: &[usize],
    ids: &[Option<SplitId>],
    dragged: usize,
    target: usize,
    part: Part,
    across: bool,
) -> Option<RowDrop> {
    let from = order.iter().position(|&ix| ix == dragged)?;
    if dragged == target || !order.contains(&target) {
        return None;
    }
    let mut rest = order.to_vec();
    rest.remove(from);
    let at = rest.iter().position(|&ix| ix == target)?;
    let own = split_of(ids, dragged);
    let together = own.is_some() && own == split_of(ids, target);

    let slot = match part {
        Part::Middle if together => return None,
        Part::Middle => return Some(RowDrop::Join { target, to: run_end(&rest, ids, at) }),
        Part::Top => at,
        Part::Bottom => at + 1,
    };
    // Inside its own bracket the row goes exactly where it was dropped; anywhere else the drop
    // lands outside whatever bracket the slot is in.
    let to = if together { slot } else { outside_bracket(&rest, ids, slot) };
    // A member dropped outside its own bracket leaves the split even where it keeps its place in
    // the list: the last one on the row under the bracket, the first on the row above it.
    let leaves = own.is_some() && !together;
    if to == from && !leaves && !across {
        return None;
    }
    Some(if leaves { RowDrop::Leave { to } } else { RowDrop::Reorder { to } })
}

/// The index just after the run of `list[at]`'s split that `at` is in (`at + 1` for a terminal in
/// none).
fn run_end(list: &[usize], ids: &[Option<SplitId>], at: usize) -> usize {
    let Some(id) = split_of(ids, list[at]) else { return at + 1 };
    let ahead = list[at + 1..].iter().take_while(|&&ix| split_of(ids, ix) == Some(id)).count();
    at + 1 + ahead
}

/// `slot` of `list` (the index a row would be put back at), moved to the nearer end of the run of
/// one split it is strictly inside, the start on a tie; `slot` as it is otherwise.
fn outside_bracket(list: &[usize], ids: &[Option<SplitId>], slot: usize) -> usize {
    if slot == 0 || slot >= list.len() {
        return slot;
    }
    let Some(id) = split_of(ids, list[slot - 1]).filter(|&id| split_of(ids, list[slot]) == Some(id)) else {
        return slot;
    };
    let before = list[..slot].iter().rev().take_while(|&&ix| split_of(ids, ix) == Some(id)).count();
    let after = list[slot..].iter().take_while(|&&ix| split_of(ids, ix) == Some(id)).count();
    if before <= after { slot - before } else { slot + after }
}

#[cfg(test)]
mod tests {
    use super::{Part, RowDrop, resolve_row_drop};
    use crate::view::sidebar::order::move_in_block;

    /// 0 and 4 and 7 are alone; 1, 2, 3 are one split; 5 and 6 another.
    const IDS: [Option<i64>; 8] = [None, Some(10), Some(10), Some(10), None, Some(20), Some(20), None];
    const ORDER: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    fn dropped(dragged: usize, target: usize, part: Part) -> Option<RowDrop> {
        resolve_row_drop(&ORDER, &IDS, dragged, target, part)
    }

    /// The list after a `to` is applied.
    fn applied(dragged: usize, to: usize) -> Vec<usize> {
        let from = ORDER.iter().position(|&ix| ix == dragged).unwrap_or(0);
        move_in_block(&ORDER, &[false; 8], from, to)
    }

    #[test]
    fn the_pointer_is_over_the_top_quarter_the_middle_half_or_the_bottom_quarter() {
        assert_eq!(Part::at(-0.1), Part::Top, "above the line counts as its top");
        assert_eq!(Part::at(0.0), Part::Top);
        assert_eq!(Part::at(0.249), Part::Top);
        assert_eq!(Part::at(0.25), Part::Middle, "the middle half starts at a quarter");
        assert_eq!(Part::at(0.5), Part::Middle);
        assert_eq!(Part::at(0.75), Part::Middle, "and ends at three quarters");
        assert_eq!(Part::at(0.751), Part::Bottom);
        assert_eq!(Part::at(1.2), Part::Bottom);
    }

    #[test]
    fn a_terminal_with_no_split_dropped_beside_an_ordinary_row_just_moves() {
        assert_eq!(dropped(0, 4, Part::Top), Some(RowDrop::Reorder { to: 3 }));
        assert_eq!(applied(0, 3), [1, 2, 3, 0, 4, 5, 6, 7]);
        assert_eq!(dropped(0, 4, Part::Bottom), Some(RowDrop::Reorder { to: 4 }));
        assert_eq!(dropped(7, 4, Part::Top), Some(RowDrop::Reorder { to: 4 }));
        assert_eq!(dropped(4, 0, Part::Top), Some(RowDrop::Reorder { to: 0 }));
    }

    #[test]
    fn dropped_between_two_members_of_another_split_it_lands_outside_that_bracket() {
        // Between 1 and 2, the nearer end is the top; between 2 and 3, the bottom.
        assert_eq!(dropped(7, 2, Part::Top), Some(RowDrop::Reorder { to: 1 }));
        assert_eq!(applied(7, 1), [0, 7, 1, 2, 3, 4, 5, 6]);
        assert_eq!(dropped(7, 2, Part::Bottom), Some(RowDrop::Reorder { to: 4 }));
        assert_eq!(dropped(7, 1, Part::Bottom), Some(RowDrop::Reorder { to: 1 }), "same slot as above 2");
        assert_eq!(dropped(7, 3, Part::Top), Some(RowDrop::Reorder { to: 4 }));
        // A split of two: the middle is a tie, and a tie goes above.
        assert_eq!(dropped(0, 5, Part::Bottom), Some(RowDrop::Reorder { to: 4 }));
        assert_eq!(applied(0, 4), [1, 2, 3, 4, 0, 5, 6, 7]);
        assert_eq!(dropped(0, 6, Part::Top), Some(RowDrop::Reorder { to: 4 }));
        // Next to a bracket rather than in it is just a move.
        assert_eq!(dropped(7, 1, Part::Top), Some(RowDrop::Reorder { to: 1 }));
        assert_eq!(dropped(0, 3, Part::Bottom), Some(RowDrop::Reorder { to: 3 }));
    }

    #[test]
    fn the_middle_of_a_row_makes_a_split_of_the_two_or_joins_its_split() {
        assert_eq!(dropped(0, 4, Part::Middle), Some(RowDrop::Join { target: 4, to: 4 }));
        assert_eq!(applied(0, 4), [1, 2, 3, 4, 0, 5, 6, 7], "right after the row it joined");
        // Onto a member of a split: after the whole bracket.
        assert_eq!(dropped(7, 2, Part::Middle), Some(RowDrop::Join { target: 2, to: 4 }));
        assert_eq!(dropped(7, 1, Part::Middle), Some(RowDrop::Join { target: 1, to: 4 }));
        assert_eq!(dropped(0, 6, Part::Middle), Some(RowDrop::Join { target: 6, to: 6 }));
        // A member of one split onto another's row: it leaves the first as it joins.
        assert_eq!(dropped(2, 5, Part::Middle), Some(RowDrop::Join { target: 5, to: 6 }));
        // From below the target too.
        assert_eq!(dropped(7, 4, Part::Middle), Some(RowDrop::Join { target: 4, to: 5 }));
    }

    #[test]
    fn the_middle_of_a_row_of_its_own_split_does_nothing() {
        assert_eq!(dropped(1, 2, Part::Middle), None);
        assert_eq!(dropped(3, 1, Part::Middle), None);
    }

    #[test]
    fn a_row_dragged_out_of_its_bracket_leaves_the_split_where_it_lands() {
        assert_eq!(dropped(2, 4, Part::Top), Some(RowDrop::Leave { to: 3 }));
        assert_eq!(applied(2, 3), [0, 1, 3, 2, 4, 5, 6, 7]);
        assert_eq!(dropped(3, 4, Part::Bottom), Some(RowDrop::Leave { to: 4 }));
        assert_eq!(dropped(2, 0, Part::Bottom), Some(RowDrop::Leave { to: 1 }), "above its own bracket");
        assert_eq!(dropped(1, 0, Part::Top), Some(RowDrop::Leave { to: 0 }));
        assert_eq!(dropped(2, 7, Part::Bottom), Some(RowDrop::Leave { to: 7 }));
        // Into another bracket it lands outside it, and is still leaving its own.
        assert_eq!(dropped(2, 6, Part::Top), Some(RowDrop::Leave { to: 4 }));
        assert_eq!(dropped(2, 5, Part::Top), Some(RowDrop::Leave { to: 4 }));
    }

    #[test]
    fn the_edge_member_dropped_just_outside_its_bracket_leaves_it_where_it_is() {
        // The last member of 1 2 3, dropped on the top of the row under it: the line between them is
        // outside the bracket, so the row stays where it is in the list and is no longer in the split.
        assert_eq!(dropped(3, 4, Part::Top), Some(RowDrop::Leave { to: 3 }), "the last member, just below");
        assert_eq!(applied(3, 3), ORDER, "and the list is as it was");
        // The first, dropped on the bottom of the row above it.
        assert_eq!(dropped(1, 0, Part::Bottom), Some(RowDrop::Leave { to: 1 }), "the first, just above");
        assert_eq!(applied(1, 1), ORDER);
        // A split of two, at both ends.
        assert_eq!(dropped(6, 7, Part::Top), Some(RowDrop::Leave { to: 6 }));
        assert_eq!(dropped(5, 4, Part::Bottom), Some(RowDrop::Leave { to: 5 }));
        // A row that is in no split dropped where it is still changes nothing, and so does a member
        // dropped back inside its own bracket.
        assert_eq!(dropped(4, 3, Part::Bottom), None);
        assert_eq!(dropped(3, 2, Part::Bottom), None, "just below 2, where it is, inside the bracket");
    }

    #[test]
    fn a_drop_inside_the_own_bracket_reorders_the_sidebar_and_keeps_the_split() {
        assert_eq!(dropped(1, 3, Part::Bottom), Some(RowDrop::Reorder { to: 3 }));
        assert_eq!(applied(1, 3), [0, 2, 3, 1, 4, 5, 6, 7]);
        assert_eq!(dropped(3, 1, Part::Top), Some(RowDrop::Reorder { to: 1 }));
        assert_eq!(dropped(1, 3, Part::Top), Some(RowDrop::Reorder { to: 2 }));
        assert_eq!(dropped(3, 2, Part::Top), Some(RowDrop::Reorder { to: 2 }));
    }

    #[test]
    fn a_drop_that_leaves_every_row_where_it_is_does_nothing() {
        assert_eq!(dropped(1, 1, Part::Top), None, "onto itself");
        assert_eq!(dropped(1, 2, Part::Top), None, "just above 2, where it is");
        assert_eq!(dropped(2, 1, Part::Bottom), None);
        assert_eq!(dropped(7, 6, Part::Bottom), None, "a solo row already last");
        assert_eq!(dropped(4, 3, Part::Bottom), None);
        assert_eq!(dropped(4, 5, Part::Top), None);
    }

    #[test]
    fn rows_that_are_not_in_the_list_are_not_dropped() {
        assert_eq!(dropped(9, 1, Part::Top), None);
        assert_eq!(dropped(1, 9, Part::Top), None);
        assert_eq!(resolve_row_drop(&[], &IDS, 1, 2, Part::Top), None);
        // A band's list holds only its own terminals.
        assert_eq!(resolve_row_drop(&[1, 2, 3], &IDS, 1, 5, Part::Middle), None);
    }

    #[test]
    fn the_list_is_the_one_given_so_a_band_decides_only_among_its_own() {
        // Band: 7, 1, 2, 3 (a split of three after a lone terminal).
        let band = [7, 1, 2, 3];
        assert_eq!(resolve_row_drop(&band, &IDS, 7, 2, Part::Bottom), Some(RowDrop::Reorder { to: 3 }));
        assert_eq!(
            resolve_row_drop(&band, &IDS, 7, 3, Part::Middle),
            Some(RowDrop::Join { target: 3, to: 3 })
        );
        // Another band's member of the same split isn't here to be in the way.
        assert_eq!(
            resolve_row_drop(&[1, 4, 5], &IDS, 4, 1, Part::Middle),
            Some(RowDrop::Join { target: 1, to: 1 })
        );
    }
}
