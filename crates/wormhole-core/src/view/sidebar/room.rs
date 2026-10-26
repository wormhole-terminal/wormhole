//! Rows make room: while a terminal is dragged over the list, the lines it passes step aside to open a
//! gap where it would land, and the lines it leaves behind close the one it came from. Pure geometry,
//! like `motion`'s: the app measures where each line sits with nothing displaced, this says how far
//! each has to be pushed, and what moves the lines is the app's.
//!
//! Where the gap is comes from the drop itself (`place`: the line a drop would land before), so the
//! gap and the drop can't disagree; this only works out the distances. A line's pitch, not its
//! height, is what a gap costs: a card's bottom gap is part of the line above it.

use std::ops::Range;

use super::listing::{Entry, Listing};
use super::motion::Drawn;
use super::split::{SplitId, split_of};

/// The lines of `listing` the dragged terminal `dragged` (its place in the window) takes with it: its
/// own row, and the rows of its split that stand next to it, which move together. Entry indices, a run.
/// `None` when the terminal has no row in the list (it is in a folded group).
#[must_use]
pub fn run_of(listing: &Listing, split_of_each: &[Option<SplitId>], dragged: usize) -> Option<Range<usize>> {
    let session_at = |at: usize| match listing.entries.get(at)? {
        Entry::Row { session, .. } => Some(*session),
        Entry::Band { .. } => None,
    };
    let own = (0..listing.entries.len()).find(|&at| session_at(at) == Some(dragged))?;
    let Some(split) = split_of(split_of_each, dragged) else { return Some(own..own + 1) };
    let mates =
        |at: usize| session_at(at).is_some_and(|session| split_of(split_of_each, session) == Some(split));
    let start = (0..own).rev().take_while(|&at| mates(at)).last().unwrap_or(own);
    let end = (own + 1..listing.entries.len()).take_while(|&at| mates(at)).last().unwrap_or(own) + 1;
    Some(start..end)
}

/// Where the line at `at` of `lines` ends: the top of the next one, so what lies between them (a card's
/// gap) counts, or its own bottom for the last.
fn pitch_end<K>(lines: &[Drawn<K>], at: usize) -> f32 {
    match lines.get(at + 1) {
        Some(next) => next.top,
        None => lines.get(at).map_or(0.0, |line| line.top + line.height),
    }
}

/// How far each line that has to move is pushed (down is positive) to open a gap for the lines `run` at
/// slot `slot`: the index of the line the gap opens before, `lines.len()` for the end. Lines of the run
/// stay where they are (the app hides them); a gap where the run already is, or right beside it, moves
/// nothing.
///
/// Dragged down, the lines between the run and the slot step up by the run's height, which opens the
/// gap just above the slot's line; dragged up, the lines from the slot to the run step down.
#[must_use]
pub fn make_room<K: Clone>(lines: &[Drawn<K>], run: Range<usize>, slot: usize) -> Vec<(K, f32)> {
    if run.is_empty() || run.end > lines.len() || slot > lines.len() || (run.start..=run.end).contains(&slot)
    {
        return Vec::new();
    }
    let height = pitch_end(lines, run.end - 1) - lines[run.start].top;
    let (pushed, by) = if slot > run.end { (run.end..slot, -height) } else { (slot..run.start, height) };
    lines[pushed].iter().map(|line| (line.key.clone(), by)).collect()
}

#[cfg(test)]
mod tests {
    use super::{make_room, run_of};
    use crate::view::sidebar::motion::Drawn;
    use crate::view::sidebar::{ListInput, Listing, SidebarNumbers, Splits, WindowSidebar, group_key};

    /// `heights` stacked from 4, with `gap` between each (the pitch is height + gap).
    fn lines(heights: &[f32], gap: f32) -> Vec<Drawn<usize>> {
        let mut top = 4.0;
        heights
            .iter()
            .enumerate()
            .map(|(key, &height)| {
                let line = Drawn { key, top, height };
                top += height + gap;
                line
            })
            .collect()
    }

    fn pushed(lines: &[Drawn<usize>], run: std::ops::Range<usize>, slot: usize) -> Vec<(usize, f32)> {
        make_room(lines, run, slot)
    }

    #[test]
    fn dragged_down_the_lines_it_passes_step_up_by_its_height() {
        let rows = lines(&[40.0; 5], 0.0);
        // Row 0 over the gap before row 3: rows 1 and 2 close up the place it left.
        assert_eq!(pushed(&rows, 0..1, 3), [(1, -40.0), (2, -40.0)]);
        // To the very end, past every row.
        assert_eq!(pushed(&rows, 1..2, 5), [(2, -40.0), (3, -40.0), (4, -40.0)]);
    }

    #[test]
    fn dragged_up_the_lines_it_passes_step_down_by_its_height() {
        let rows = lines(&[40.0; 5], 0.0);
        assert_eq!(pushed(&rows, 3..4, 1), [(1, 40.0), (2, 40.0)]);
        assert_eq!(pushed(&rows, 4..5, 0), [(0, 40.0), (1, 40.0), (2, 40.0), (3, 40.0)], "to the very top");
    }

    #[test]
    fn a_gap_where_the_run_is_or_right_beside_it_moves_nothing() {
        let rows = lines(&[40.0; 4], 0.0);
        for slot in [1, 2] {
            assert!(pushed(&rows, 1..2, slot).is_empty(), "slot {slot} is on the row's own edges");
        }
        assert!(pushed(&rows, 0..1, 1).is_empty(), "first row, the gap after it");
        assert!(pushed(&rows, 3..4, 4).is_empty(), "last row, the end");
        assert!(pushed(&rows, 3..4, 3).is_empty(), "last row, before itself");
    }

    #[test]
    fn the_distance_is_the_runs_pitch_so_a_card_gap_between_lines_counts() {
        // Rows of 40 with 6 between them: a pitch of 46, which is what the lines below must give up.
        let rows = lines(&[40.0, 40.0, 40.0, 40.0], 6.0);
        assert_eq!(pushed(&rows, 0..1, 3), [(1, -46.0), (2, -46.0)]);
        assert_eq!(pushed(&rows, 3..4, 1), [(1, 40.0), (2, 40.0)], "the last line has no gap under it");
    }

    #[test]
    fn lines_of_other_heights_give_the_runs_own_height_not_their_own() {
        // A band's head is shorter than a row: dragging the 40-high row past it still moves it by 40.
        let rows = lines(&[24.0, 40.0, 40.0, 24.0, 40.0], 0.0);
        assert_eq!(pushed(&rows, 1..2, 4), [(2, -40.0), (3, -40.0)]);
        assert_eq!(pushed(&rows, 2..3, 0), [(0, 40.0), (1, 40.0)]);
    }

    #[test]
    fn a_run_of_several_lines_moves_the_lines_it_passes_by_all_of_them() {
        let rows = lines(&[40.0; 6], 0.0);
        assert_eq!(pushed(&rows, 1..3, 5), [(3, -80.0), (4, -80.0)]);
        assert_eq!(pushed(&rows, 3..5, 0), [(0, 80.0), (1, 80.0), (2, 80.0)]);
        assert!(pushed(&rows, 1..3, 2).is_empty(), "inside the run");
    }

    #[test]
    fn a_run_or_slot_that_is_not_in_the_list_moves_nothing() {
        let rows = lines(&[40.0; 3], 0.0);
        assert!(pushed(&rows, 0..0, 2).is_empty());
        assert!(pushed(&rows, 2..4, 0).is_empty());
        assert!(pushed(&rows, 0..1, 4).is_empty());
        assert!(pushed(&[], 0..1, 0).is_empty());
    }

    /// The list of `split_of.len()` terminals, the last `grouped` of them in a group of their own, which is
    /// folded when `folded` says so.
    fn listing(split_of: &[Option<i64>], grouped: usize, folded: bool) -> Listing {
        let ids: Vec<i64> = (1..=split_of.len() as i64).collect();
        let mut sidebar = WindowSidebar::default();
        let members = &ids[ids.len() - grouped..];
        if !members.is_empty() {
            let group = sidebar.new_group(&ids, members);
            if folded {
                sidebar.folded.insert(group_key(group));
            }
        }
        Listing::build(&ListInput {
            ids: &ids,
            pinned: &vec![false; ids.len()],
            sidebar: &sidebar,
            splits: Splits { split_of, selected: None },
            numbers: SidebarNumbers::List,
        })
    }

    #[test]
    fn a_terminal_takes_the_rows_of_its_split_that_stand_next_to_it() {
        // 1 alone; 2, 3, 4 a split; 5 alone in a group (a head, then its row).
        let split = [None, Some(9), Some(9), Some(9), None];
        let listing = listing(&split, 1, false);
        assert_eq!(run_of(&listing, &split, 0), Some(0..1), "a terminal in no split moves alone");
        for dragged in [1, 2, 3] {
            assert_eq!(
                run_of(&listing, &split, dragged),
                Some(1..4),
                "terminal {dragged} moves with its split"
            );
        }
        assert_eq!(run_of(&listing, &split, 4), Some(5..6), "the head of its group is not part of it");
        assert_eq!(run_of(&listing, &split, 9), None, "a terminal the list does not have");
    }

    #[test]
    fn mates_in_another_group_are_not_next_to_it_and_stay_where_they_are() {
        // 1 and 3 are one split, but 3 is in a group under its head: the rows are apart.
        let split = [Some(9), None, Some(9)];
        let listing = listing(&split, 1, false);
        assert_eq!(run_of(&listing, &split, 0), Some(0..1));
        assert_eq!(run_of(&listing, &split, 2), Some(3..4));
    }

    #[test]
    fn a_terminal_in_a_folded_group_has_no_line_to_make_room_for() {
        let split = [None, None, None];
        let listing = listing(&split, 1, true);
        assert_eq!(run_of(&listing, &split, 2), None, "its row is folded away");
        assert_eq!(run_of(&listing, &split, 0), Some(0..1));
    }
}
