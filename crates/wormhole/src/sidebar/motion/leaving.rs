//! The rows that are leaving the list: a terminal that closed, or the rows of a group that folded, stay
//! drawn for as long as their motion takes and are dropped when it is over.
//!
//! A leaving row is drawn from what the list held when it was still there (`Snapshot`: the window's
//! facts and the listing, kept by the list when it last changed), set in its card as it stood, and does
//! nothing: it has no click, drag, menu, close button, tooltip or keyboard stop, it is not a stop of
//! the keys' walk (that goes by the listing, which no longer has it), and it is not a tree item for
//! VoiceOver, which is told of the rows that are in the list. Only what the row showed is drawn.

use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::view::sidebar::{Card, Entry, Listing};

use super::list::ItemKey;
use super::row::{Natural, RowMotion};
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

    pub(super) fn get(&self, key: &ItemKey) -> Option<&Leaving> {
        self.0.iter().find(|leaving| leaving.key == *key)
    }

    /// Drops the rows of change `batch`, which are over. What made room for them, if it is a spring,
    /// is the spring the lines below move on to close the gap.
    pub(super) fn retire(&mut self, batch: u64) -> Option<RowMotion> {
        let mut gone = None;
        self.0.retain(|leaving| {
            let stays = leaving.batch != batch;
            if !stays {
                gone = Some(leaving.motion);
            }
            stays
        });
        gone
    }

    /// Where each leaving row is drawn among the lines `keys`: the index it is put before, and which
    /// row it is, in the order to put them (a row stands after the line it stood after; where that
    /// line has gone too, last). Put them in from the end, so each index stays what it says.
    pub(super) fn slots(&self, keys: &[ItemKey]) -> Vec<(usize, usize)> {
        let mut slots: Vec<(usize, usize)> = self
            .0
            .iter()
            .enumerate()
            .map(|(which, leaving)| {
                let at = match &leaving.after {
                    None => 0,
                    Some(after) => keys.iter().position(|key| key == after).map_or(keys.len(), |at| at + 1),
                };
                (at, which)
            })
            .collect();
        slots.sort_unstable();
        slots
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
            let Some(&Entry::Row { session, band, position, of, number, marks }) =
                snapshot.listing.entries.get(leaving.at)
            else {
                continue;
            };
            let Some(info) = snapshot.input.rows.get(session) else { continue };
            let card = snapshot.listing.card(leaving.at);
            let place = Place {
                selected: session == snapshot.input.active,
                solo: frame.metrics.cards && matches!(card, Some(Card::Loose(_))),
                position,
                of,
                number,
                grouped: band.is_some(),
                marks,
                changes_above: false,
            };
            let face = row_face(info, &place, frame, None, window, cx);
            // A card shared with rows that stay already has its glass; a card of its own leaves with the row.
            let glass = matches!(card, Some(Card::Loose(_))).then_some(self.input.glass.as_ref()).flatten();
            let cards = Cards {
                listing: &snapshot.listing,
                infos: &snapshot.input.rows,
                ui: frame.ui,
                metrics: frame.metrics,
                glass,
                wash,
                active: snapshot.input.active,
            };
            let line = cards.line(leaving.at, face.row.into_any_element()).into_any_element();
            lines.push((put, self.with_list_motion(leaving.key.clone(), line, false)));
        }
        lines
    }
}
