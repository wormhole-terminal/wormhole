//! The rows that are leaving the list: a terminal that closed, or the rows of a group that folded, stay
//! drawn for as long as their motion takes and are dropped when it is over.
//!
//! A leaving row is drawn from what the list held when it was still there (`Snapshot`: the window's
//! facts and the listing, kept by the list when it last changed), set in its card as it stood, and does
//! nothing: it has no click, drag, menu, close button, tooltip or keyboard stop, it is not a stop of
//! the keys' walk (that goes by the listing, which no longer has it), and it is not a tree item for
//! VoiceOver, which is told of the rows that are in the list. Only what the row showed is drawn.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::view::sidebar::{Entry, Listing};

use super::list::ItemKey;
use super::pose::{Natural, RowMotion};
use crate::sidebar::card::Cards;
use crate::sidebar::row::{Frame, Place, row_face};
use crate::sidebar::{Sidebar, SidebarInput};

/// The window's facts and the listing at one moment: what a row that has left the list was drawn from.
pub(in crate::sidebar) struct Snapshot {
    pub(super) input: Rc<SidebarInput>,
    pub(super) listing: Listing,
}

/// A row that has left the list and is still drawn.
pub(super) struct Leaving {
    pub(super) key: ItemKey,
    /// The line it stood after, which is still in the list; `None` when it was the first.
    after: Option<ItemKey>,
    pub(super) motion: RowMotion,
    /// Its place in the listing of `snapshot`.
    at: usize,
    snapshot: Rc<Snapshot>,
    /// The change that made it leave, whose timer takes it away.
    pub(super) batch: u64,
    pub(super) natural: Natural,
}

impl Leaving {
    /// Whether the row was the terminal on screen when it was last drawn in the list.
    #[cfg(test)]
    pub(super) fn selected(&self) -> bool {
        Place::of(
            &self.snapshot.listing,
            &self.snapshot.input.rows,
            self.at,
            self.snapshot.input.active,
            false,
        )
        .is_some_and(|place| place.selected)
    }

    pub(super) fn new(
        key: ItemKey,
        after: Option<ItemKey>,
        motion: RowMotion,
        at: usize,
        snapshot: Rc<Snapshot>,
        batch: u64,
    ) -> Self {
        Self { key, after, motion, at, snapshot, batch, natural: Natural::default() }
    }
}

/// Every row that is leaving now.
#[derive(Default)]
pub(super) struct Leavers(Vec<Leaving>);

impl Leavers {
    pub(super) fn push(&mut self, leaving: Leaving) {
        self.0.push(leaving);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    /// The terminals whose rows are leaving.
    #[cfg(test)]
    pub(super) fn ids(&self) -> Vec<i64> {
        self.0
            .iter()
            .filter_map(|leaving| if let ItemKey::Row(id) = leaving.key { Some(id) } else { None })
            .collect()
    }

    /// Whether each row that is leaving was the terminal on screen when it was last drawn.
    #[cfg(test)]
    pub(super) fn selected(&self) -> Vec<(i64, bool)> {
        self.0
            .iter()
            .filter_map(|leaving| {
                if let ItemKey::Row(id) = leaving.key { Some((id, leaving.selected())) } else { None }
            })
            .collect()
    }

    pub(super) fn get(&self, key: &ItemKey) -> Option<&Leaving> {
        self.0.iter().find(|leaving| leaving.key == *key)
    }

    /// Drops the rows of change `batch`, which are over. What made room for them, if it is a spring,
    /// is the spring the lines below move on to close the gap.
    pub(super) fn retire(&mut self, batch: u64) -> Option<RowMotion> {
        let mut gone = None;
        let mut dropped: Vec<(ItemKey, Option<ItemKey>)> = Vec::new();
        self.0.retain(|leaving| {
            let stays = leaving.batch != batch;
            if !stays {
                gone = Some(leaving.motion);
                dropped.push((leaving.key.clone(), leaving.after.clone()));
            }
            stays
        });
        // A row that stood after one that is gone now stands where that one did.
        for leaving in &mut self.0 {
            while let Some((_, after)) = dropped.iter().find(|(key, _)| Some(key) == leaving.after.as_ref()) {
                leaving.after = after.clone();
            }
        }
        gone
    }

    /// Drops the rows whose lines are in the list again (`standing`): they came back before they were
    /// gone, and a line is never drawn twice.
    pub(super) fn forget(&mut self, standing: &HashSet<&ItemKey>) {
        self.0.retain(|leaving| !standing.contains(&leaving.key));
    }

    /// Where each leaving row is drawn among the lines `keys`: the index it is put before, how many
    /// leaving rows it follows there, and which row it is, in the order to put them. A row stands after
    /// the line it stood after; where that is another leaving row, after that one; where that is gone
    /// from the list, last. Put them in from the end, so each index stays what it says.
    pub(super) fn slots(&self, keys: &[ItemKey]) -> Vec<(usize, usize)> {
        let mut slots: Vec<(usize, usize, usize)> = (0..self.0.len())
            .map(|which| {
                let (at, hops) = self.anchor(keys, which);
                (at, hops, which)
            })
            .collect();
        slots.sort_unstable();
        slots.into_iter().map(|(at, _, which)| (at, which)).collect()
    }

    /// The line row `which` is put before, and through how many leaving rows it is found.
    fn anchor(&self, keys: &[ItemKey], which: usize) -> (usize, usize) {
        let (mut after, mut hops) = (&self.0[which].after, 0);
        while let Some(key) = after {
            if let Some(at) = keys.iter().position(|line| line == key) {
                return (at + 1, hops);
            }
            // Not a line of the list: another row that is leaving, or one that is gone altogether.
            match self.0.iter().find(|leaving| leaving.key == *key) {
                Some(ahead) if hops < self.0.len() => (after, hops) = (&ahead.after, hops + 1),
                _ => return (keys.len(), hops),
            }
        }
        (0, hops)
    }

    /// The lines as they are drawn: `keys` with the leaving rows among them.
    pub(super) fn drawn(&self, keys: &[ItemKey]) -> Vec<ItemKey> {
        let mut drawn = keys.to_vec();
        for (at, which) in self.slots(keys).into_iter().rev() {
            drawn.insert(at, self.0[which].key.clone());
        }
        drawn
    }
}

impl Sidebar {
    /// The leaving rows, drawn, each with the index among `keys`' lines to put it before (from the
    /// end of the list to its start: insert them in that order).
    pub(in crate::sidebar) fn leaving_lines(
        &self,
        keys: &[ItemKey],
        frame: &Frame,
        wash: &HashMap<u32, Hsla>,
        window: &Window,
        cx: &App,
    ) -> Vec<(usize, AnyElement)> {
        let leavers = &self.motion.leaving;
        if leavers.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        for (put, which) in leavers.slots(keys).into_iter().rev() {
            let Some(leaving) = leavers.0.get(which) else { continue };
            let snapshot = &*leaving.snapshot;
            let (infos, active) = (&snapshot.input.rows, snapshot.input.active);
            let Some(place) = Place::of(&snapshot.listing, infos, leaving.at, active, frame.metrics.cards)
            else {
                continue;
            };
            let Entry::Row { session, .. } = snapshot.listing.entries[leaving.at] else { continue };
            let face = row_face(&infos[session], &place, frame, None, window, cx);
            // Native glass cannot fade, so a row that is going takes none with it.
            let cards = Cards {
                listing: &snapshot.listing,
                infos,
                ui: frame.ui,
                metrics: frame.metrics,
                glass: None,
                wash,
                active,
            };
            let line = cards.line(leaving.at, face.row.into_any_element(), false).into_any_element();
            lines.push((
                put,
                self.with_list_motion(leaving.key.clone(), line, false, div()).into_any_element(),
            ));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::rc::Rc;

    use wormhole_core::view::motion::InsertMotion;
    use wormhole_core::view::sidebar::Listing;

    use super::{ItemKey, Leavers, Leaving, RowMotion, Snapshot};

    fn row(id: i64) -> ItemKey {
        ItemKey::Row(id)
    }

    fn leaving(id: i64, after: Option<i64>, batch: u64) -> Leaving {
        let snapshot = Rc::new(Snapshot { input: Rc::default(), listing: Listing::default() });
        let motion = RowMotion::Insert(InsertMotion::FromSide);
        Leaving::new(row(id), after.map(row), motion, 0, snapshot, batch)
    }

    fn order(leavers: &Leavers, keys: &[ItemKey]) -> Vec<i64> {
        leavers
            .drawn(keys)
            .into_iter()
            .map(|key| match key {
                ItemKey::Row(id) => id,
                ItemKey::Band(_) => -1,
            })
            .collect()
    }

    #[test]
    fn a_row_stays_where_it_was_and_after_the_row_it_stood_after() {
        let keys = [row(1), row(2)];
        let mut leavers = Leavers::default();
        leavers.push(leaving(9, Some(1), 1));
        assert_eq!(order(&leavers, &keys), [1, 9, 2]);
        leavers.push(leaving(8, None, 1));
        assert_eq!(order(&leavers, &keys), [8, 1, 9, 2]);
    }

    #[test]
    fn a_row_that_stood_after_a_row_that_left_after_it_stays_after_that_one() {
        // A, Y, X, B: X closes (it stood after Y), then Y closes (it stood after A) within X's time.
        let keys = [row(1), row(4)];
        let mut leavers = Leavers::default();
        leavers.push(leaving(3, Some(2), 1));
        leavers.push(leaving(2, Some(1), 2));
        assert_eq!(order(&leavers, &keys), [1, 2, 3, 4], "X is not sent to the end of the list");
    }

    #[test]
    fn a_row_that_stood_after_one_that_is_gone_for_good_stands_where_that_one_did() {
        let keys = [row(1), row(4)];
        let mut leavers = Leavers::default();
        leavers.push(leaving(3, Some(2), 1));
        leavers.push(leaving(2, Some(1), 2));
        assert!(leavers.retire(2).is_some());
        assert_eq!(order(&leavers, &keys), [1, 3, 4]);
    }

    #[test]
    fn a_row_that_is_back_in_the_list_is_forgotten_not_drawn_twice() {
        let keys = [row(1), row(2), row(3)];
        let mut leavers = Leavers::default();
        leavers.push(leaving(2, Some(1), 1));
        let standing: HashSet<&ItemKey> = keys.iter().collect();
        leavers.forget(&standing);
        assert!(leavers.is_empty());
        assert_eq!(order(&leavers, &keys), [1, 2, 3]);
    }
}
