//! A terminal opening and closing in the list: which lines came and which went when the list changed,
//! and the motion each gets from the person's choice (`InsertMotion`). A row that is one of a group's
//! that folded or opened is a fold's (`fold`), not an insert's. Pure bookkeeping over keys and the
//! listings they came from; drawing is `row`'s and `leaving`'s, timing `list`'s.

use std::collections::HashSet;

use wormhole_core::view::motion::pose::{Phase, fold::Slot};
use wormhole_core::view::motion::{InsertMotion, MotionChoices};
use wormhole_core::view::sidebar::{Entry, Listing};

use super::fold::FoldNote;
use super::list::ItemKey;
use super::pose::RowMotion;

/// A line that came into the list or went from it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Change {
    pub(super) key: ItemKey,
    /// Its place among the lines of the list it was in (before, for one that went; now, for one that came).
    pub(super) at: usize,
    /// The group it was a row of there, if it was one.
    pub(super) group: Option<u32>,
}

/// The group of the row at `at` of `listing`.
fn group_at(listing: &Listing, at: usize) -> Option<u32> {
    match listing.entries.get(at)? {
        Entry::Row { band: Some(band), .. } => listing.bands.get(*band).map(|band| band.group),
        Entry::Row { band: None, .. } | Entry::Band { .. } => None,
    }
}

/// The lines of `before` (listed in `was`, if it is known) that are not in `now`, and the lines of
/// `now` (listed in `listing`) that were not in `before`.
pub(super) fn changes(
    before: &[ItemKey],
    was: Option<&Listing>,
    now: &[ItemKey],
    listing: &Listing,
) -> (Vec<Change>, Vec<Change>) {
    let (had, has): (HashSet<&ItemKey>, HashSet<&ItemKey>) = (before.iter().collect(), now.iter().collect());
    let gone = before
        .iter()
        .enumerate()
        .filter(|(_, key)| !has.contains(key))
        .map(|(at, key)| Change { key: key.clone(), at, group: was.and_then(|was| group_at(was, at)) })
        .collect();
    let came = now
        .iter()
        .enumerate()
        .filter(|(_, key)| !had.contains(key))
        .map(|(at, key)| Change { key: key.clone(), at, group: group_at(listing, at) })
        .collect();
    (gone, came)
}

/// The motion each of `changes` gets, all of them going (`Exit`) or coming (`Enter`): a row of the group
/// that `fold` says was just folded or opened is one of that fold's rows, in its place among them; any
/// other row is a terminal opening or closing; a group's head, which has no choice of its own, comes as
/// Classic does.
pub(super) fn motions(
    changes: &[Change],
    phase: Phase,
    fold: Option<&FoldNote>,
    choices: MotionChoices,
) -> Vec<RowMotion> {
    let of_fold = |change: &Change| fold.is_some_and(|note| note.owns(change.group, phase));
    let rows =
        changes.iter().filter(|change| matches!(change.key, ItemKey::Row(_)) && of_fold(change)).count();
    let mut index = 0;
    changes
        .iter()
        .map(|change| match change.key {
            ItemKey::Band(_) => RowMotion::Insert(InsertMotion::Classic),
            ItemKey::Row(_) if of_fold(change) => {
                let slot = Slot { index, of: rows };
                index += 1;
                RowMotion::Fold(choices.fold, slot)
            }
            ItemKey::Row(_) => RowMotion::Insert(choices.insert),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::pose::Phase;
    use wormhole_core::view::motion::{FoldMotion, InsertMotion, MotionChoices};

    use super::{Change, ItemKey, RowMotion, changes, motions};
    use crate::sidebar::motion::fold::FoldNote;

    fn row(id: i64) -> ItemKey {
        ItemKey::Row(id)
    }

    fn keys(ids: &[i64]) -> Vec<ItemKey> {
        ids.iter().map(|&id| row(id)).collect()
    }

    #[test]
    fn lines_that_are_in_one_list_and_not_the_other_are_the_changes() {
        let listing = wormhole_core::view::sidebar::Listing::default();
        let (gone, came) = changes(&keys(&[1, 2, 3]), None, &keys(&[1, 3, 4]), &listing);
        assert_eq!(gone.iter().map(|change| (&change.key, change.at)).collect::<Vec<_>>(), [(&row(2), 1)]);
        assert_eq!(came.iter().map(|change| (&change.key, change.at)).collect::<Vec<_>>(), [(&row(4), 2)]);
        let (gone, came) = changes(&keys(&[1, 2]), None, &keys(&[1, 2]), &listing);
        assert!(gone.is_empty() && came.is_empty());
    }

    fn change(id: i64, group: Option<u32>) -> Change {
        Change { key: row(id), at: 0, group }
    }

    #[test]
    fn a_terminal_opening_or_closing_is_the_inserts_choice_wherever_it_is() {
        let choices = MotionChoices { insert: InsertMotion::Reveal, ..MotionChoices::CLASSIC };
        let got = motions(&[change(1, None), change(2, Some(7))], Phase::Enter, None, choices);
        assert_eq!(got, [RowMotion::Insert(InsertMotion::Reveal); 2]);
    }

    #[test]
    fn the_rows_of_the_group_that_folded_are_its_fold_in_their_places_and_the_rest_are_inserts() {
        let choices = MotionChoices {
            fold: FoldMotion::Tuck,
            insert: InsertMotion::FromSide,
            ..MotionChoices::CLASSIC
        };
        let fold = FoldNote { group: 7, folding: true };
        let going = [change(1, Some(7)), change(2, Some(7)), change(3, Some(9)), change(4, Some(7))];
        let got = motions(&going, Phase::Exit, Some(&fold), choices);
        let slot = |index| wormhole_core::view::motion::pose::fold::Slot { index, of: 3 };
        assert_eq!(
            got,
            [
                RowMotion::Fold(FoldMotion::Tuck, slot(0)),
                RowMotion::Fold(FoldMotion::Tuck, slot(1)),
                RowMotion::Insert(InsertMotion::FromSide),
                RowMotion::Fold(FoldMotion::Tuck, slot(2)),
            ]
        );
        // The rows of a group that opens come, not go: folding it does not claim them.
        let coming = motions(&[change(1, Some(7))], Phase::Enter, Some(&fold), choices);
        assert_eq!(coming, [RowMotion::Insert(InsertMotion::FromSide)]);
    }

    #[test]
    fn a_group_head_appears_as_classic_does() {
        let choices = MotionChoices { insert: InsertMotion::Reveal, ..MotionChoices::CLASSIC };
        let head = Change { key: ItemKey::Band("g1".into()), at: 0, group: None };
        assert_eq!(motions(&[head], Phase::Enter, None, choices), [RowMotion::Insert(InsertMotion::Classic)]);
    }
}
