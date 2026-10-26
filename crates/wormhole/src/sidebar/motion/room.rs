//! Rows make room: while a terminal is dragged by its row over the list, the lines it passes step
//! aside on a spring to open a gap where it would land, the row it came from goes out of the list, and
//! letting go settles everything on the springs it already runs on. Core works out how far each line
//! steps (`view::sidebar::room`) and where the drop would land is the drop's own answer
//! (`DropMark`); this keeps what is pushed, draws it and ends it.
//!
//! Every line is pushed *inside* the box the layout gave it (`with_list_motion`'s slot), so nothing
//! about the layout changes during a drag, what the pointer is over is worked out where the lines are
//! laid out and not where they have been pushed to (no line runs from the pointer), and a drop that
//! changes the list starts its slide (`plan`) from where the lines look to be. The springs live in
//! the content's own places, like the list's moves, so the one that pushed a line aside carries on
//! as the one that moves it to its new place. Reduce Motion needs nothing of its own: a spring there
//! shows its target at once.

use std::collections::HashMap;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::drag::ROOM;
use wormhole_core::view::sidebar::motion::{Drawn, Move, Moves};
use wormhole_core::view::sidebar::place::Drop;
use wormhole_core::view::sidebar::room::{make_room, run_of};
use wormhole_core::view::sidebar::{Listing, SplitId};

use super::list::{ItemKey, ListMotion, Spring, id_of, measured};
use crate::sidebar::drop::DropMark;
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style::motion::SPRING_SNAP;

/// The room a dragged row has the list make for it.
pub(super) struct DragRoom {
    /// Every line as laid out with nothing pushed: the places the springs run in.
    lines: Vec<Drawn<ItemKey>>,
    /// Where each of `lines` is.
    index: HashMap<ItemKey, usize>,
    /// How far each line that has been pushed in this drag is pushed now (0 for one that has gone back).
    /// A line stays here until the drag is over and at rest, so its spring is never dropped while it moves.
    pushed: HashMap<ItemKey, f32>,
    /// The dragged lines (the row, and the rows of its split that move with it), by whether they are out
    /// of the list now: while a gap is open for them, which is when the drop would move them.
    lifted: HashMap<ItemKey, bool>,
    /// The drop the pointer is over makes a split: the rows settle on the soft spring.
    joining: bool,
    /// The terminal being dragged, and the split it was in when it was picked up: a split's bracket draws
    /// only when the drop leaves it in another.
    dragged: i64,
    split_before: Option<SplitId>,
    /// The drag has ended, and the timer of this batch ends the room.
    over: Option<u64>,
}

impl DragRoom {
    /// The room of a drag that begins now, over `lines` as laid out.
    fn begin(lines: Vec<Drawn<ItemKey>>, dragged: i64, split_before: Option<SplitId>) -> Self {
        let index = lines.iter().enumerate().map(|(at, line)| (line.key.clone(), at)).collect();
        Self {
            lines,
            index,
            pushed: HashMap::new(),
            lifted: HashMap::new(),
            joining: false,
            dragged,
            split_before,
            over: None,
        }
    }

    /// Whether this room was made over the lines `drawn`, in this order.
    fn holds(&self, drawn: &[ItemKey]) -> bool {
        self.lines.len() == drawn.len() && self.lines.iter().zip(drawn).all(|(line, key)| line.key == *key)
    }

    /// A new drag takes over a room that was still settling after the last: its springs go on.
    fn resume(&mut self, dragged: i64, split_before: Option<SplitId>) {
        (self.over, self.joining, self.dragged, self.split_before) = (None, false, dragged, split_before);
    }

    /// The spring a pushed line runs on: from its place, to its place and how far it is pushed.
    pub(super) fn spring_of(&self, key: &ItemKey) -> Option<Spring> {
        let by = *self.pushed.get(key)?;
        let top = self.lines.get(*self.index.get(key)?)?.top;
        Some(Spring { spec: ROOM, from: top, to: top + by, place: top })
    }

    /// Whether a dragged line is out of the list now; `None` for a line that is not being dragged.
    pub(super) fn is_lifted(&self, key: &ItemKey) -> Option<bool> {
        self.lifted.get(key).copied()
    }

    /// Whether the drag is over and the lines are on their way to rest.
    pub(super) fn is_over(&self) -> bool {
        self.over.is_some()
    }

    /// Whether any line is pushed aside now: a gap is open somewhere.
    pub(super) fn has_pushed(&self) -> bool {
        self.pushed.values().any(|by| *by != 0.0)
    }

    /// Whether the drop the pointer was over when the drag ended made a split.
    pub(super) fn joined(&self) -> bool {
        self.joining
    }

    /// The terminal that was dragged, and the split it was in.
    pub(super) fn dragged(&self) -> (i64, Option<SplitId>) {
        (self.dragged, self.split_before)
    }

    /// The dragged lines that were out of the list when the drag ended.
    pub(super) fn lifted_keys(&self) -> impl Iterator<Item = &ItemKey> {
        self.lifted.iter().filter(|(_, lifted)| **lifted).map(|(key, _)| key)
    }

    /// Everything goes back: the lines are not pushed and the dragged row is in the list.
    fn put_back(&mut self) {
        self.pushed.values_mut().for_each(|by| *by = 0.0);
        self.lifted.values_mut().for_each(|lifted| *lifted = false);
    }

    /// Opens the gap for a drop that moves the row: `mark` is where it lands among `keys`, `own` the place
    /// of the dragged terminal in `input`, and `mates` whether the rows of its split next to it go along.
    fn open(
        &mut self,
        keys: &[ItemKey],
        listing: &Listing,
        input: &SidebarInput,
        own: usize,
        mark: Option<&DropMark>,
        mates: bool,
    ) {
        self.put_back();
        let split_of = if mates { input.split_of() } else { Vec::new() };
        let Some(run) = run_of(listing, &split_of, own).and_then(|run| keys.get(run)) else { return };
        for key in run {
            self.lifted.insert(key.clone(), true);
        }
        let Some(slot) = mark.and_then(|mark| mark.slot(keys)) else { return };
        let slot = match keys.get(slot) {
            Some(key) => self.index.get(key).copied(),
            None => Some(self.lines.len()),
        };
        let span = run.iter().filter_map(|key| self.index.get(key)).fold(None, |span, &at| match span {
            None => Some((at, at + 1)),
            Some((start, end)) => Some((start.min(at), end.max(at + 1))),
        });
        if let (Some(slot), Some((start, end))) = (slot, span) {
            for (key, by) in make_room(&self.lines, start..end, slot) {
                self.pushed.insert(key, by);
            }
        }
    }

    /// `lines`, as drawn, become where they look to be when the drag ends: pushed lines where they were
    /// pushed to, the dragged lines (stacked) where the pointer let go, at `pointer` in the content.
    pub(super) fn seen(&self, lines: &mut [Drawn<ItemKey>], pointer: f32) {
        let out = |key: &ItemKey| self.lifted.get(key) == Some(&true);
        let height: f32 = lines.iter().filter(|line| out(&line.key)).map(|line| line.height).sum();
        let mut at = (pointer - height / 2.0).max(0.0);
        for line in lines.iter_mut() {
            if out(&line.key) {
                line.top = at;
                at += line.height;
            } else if let Some(by) = self.pushed.get(&line.key) {
                line.top += by;
            }
        }
    }

    /// The lines that were pushed and need no move of their own now (they are where the list puts them):
    /// they carry on on the spring that pushed them, to the place they stand in, so one that was still
    /// moving does not jump.
    pub(super) fn kept(&self, lines: &[Drawn<ItemKey>], moves: &Moves<ItemKey>) -> Vec<Move<ItemKey>> {
        lines
            .iter()
            .filter(|line| self.pushed.contains_key(&line.key))
            .filter(|line| !moves.moved.iter().any(|mv| mv.key == line.key))
            .map(|line| Move { key: line.key.clone(), from: line.top, to: line.top })
            .collect()
    }
}

#[cfg(test)]
impl ListMotion {
    /// The terminals whose rows are pushed aside now, and how far (down is positive), top to bottom.
    pub(in crate::sidebar) fn pushed_rows(&self) -> Vec<(i64, f32)> {
        let Some(room) = &self.room else { return Vec::new() };
        room.lines
            .iter()
            .filter_map(|line| match (&line.key, room.pushed.get(&line.key)) {
                (ItemKey::Row(id), Some(&by)) if by != 0.0 => Some((*id, by)),
                _ => None,
            })
            .collect()
    }

    /// The terminals whose rows are out of the list while a gap is open for them.
    pub(in crate::sidebar) fn lifted_rows(&self) -> Vec<i64> {
        let Some(room) = &self.room else { return Vec::new() };
        let mut ids: Vec<i64> = room
            .lifted_keys()
            .filter_map(|key| match key {
                ItemKey::Row(id) => Some(*id),
                ItemKey::Band(_) => None,
            })
            .collect();
        ids.sort_unstable();
        ids
    }

    /// What the last plan moved: each row's terminal, where it started and where it goes (places in the content).
    pub(in crate::sidebar) fn planned_rows(&self) -> Vec<(i64, f32, f32)> {
        self.planned
            .iter()
            .filter_map(|mv| match &mv.key {
                ItemKey::Row(id) => Some((*id, mv.from, mv.to)),
                ItemKey::Band(_) => None,
            })
            .collect()
    }

    /// Whether a dragged row's gap is still held (drawn, or on its way back to rest).
    pub(in crate::sidebar) fn has_room(&self) -> bool {
        self.room.is_some()
    }
}

impl ListMotion {
    /// Whether a gap is open for a dragged row now: some line is pushed aside.
    pub(in crate::sidebar) fn gap_open(&self) -> bool {
        self.room.as_ref().is_some_and(DragRoom::has_pushed)
    }

    /// The room has been at rest since the drag ended in `batch`: it is not drawn any more.
    pub(super) fn retire_room(&mut self, batch: u64) -> bool {
        let ended = self.room.as_ref().is_some_and(|room| room.over == Some(batch));
        if ended {
            self.room = None;
        }
        // A split's bracket drawn while the lines went back is drawn.
        let drawn = self.join.retire(batch);
        ended || drawn
    }
}

/// `line` (a dragged row) with the spring that takes it out of the list and back: it is gone while a gap
/// is open for it, because the lines that stepped aside stand where it was.
pub(super) fn lifted(line: AnyElement, key: &ItemKey, lifted: bool) -> AnyElement {
    div()
        .relative()
        .w_full()
        .child(line)
        .with_spring(
            id_of("drag-lift", key, 0),
            SpringAnimation::new(SPRING_SNAP).to(lifted).from(false),
            |line, gone| line.opacity(1.0 - gone.0.clamp(0.0, 1.0)),
        )
        .into_any_element()
}

impl Sidebar {
    /// Opens the gap the drag in flight asks for and shuts the one it left: call it once per frame of
    /// the list, before its lines are drawn, with `drop` what letting go would do and `mark` where it
    /// would land. `fresh` says the lines are the ones last measured: not in the frame a row came or went
    /// in, whose boxes are the old list's.
    ///
    /// Only a drop that moves the row rewrites the room. Over the middle of a row (a split) the room is
    /// held as it was, so a pointer that crosses rows does not shut the gap and open it again at each;
    /// over nothing it takes (the row's own place, off the list) everything goes back.
    pub(in crate::sidebar) fn follow_drag(
        &mut self,
        keys: &[ItemKey],
        listing: &Listing,
        (mark, drop): (Option<&DropMark>, Option<Drop>),
        fresh: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(dragged) = self.row_in_flight(cx) else { return };
        let Some(own) = self.input.find(dragged).filter(|_| fresh) else { return };
        let split_before = self.input.rows[own].split;
        let mut room = match self.motion.room.take() {
            Some(room) if !room.is_over() => room,
            // The last drag's lines are still settling: this one carries on from where they are.
            Some(mut room) if room.holds(&self.motion.drawn) => {
                room.resume(dragged, split_before);
                room
            }
            _ => match measured(&self.motion.drawn, &self.scroll) {
                Some(lines) => DragRoom::begin(lines, dragged, split_before),
                None => return,
            },
        };
        match drop {
            Some(Drop::Join { .. }) => room.joining = true,
            Some(Drop::Move { leave_split, with_mates, .. }) => {
                room.joining = false;
                room.open(keys, listing, &self.input, own, mark, with_mates && !leave_split);
            }
            None => {
                room.joining = false;
                room.put_back();
            }
        }
        // The held row is known from the first frame, so the spring that lifts it starts from the list.
        room.lifted.entry(ItemKey::Row(dragged)).or_insert(false);
        self.motion.room = Some(room);
    }

    /// The drag ended without the list changing: the lines go back to where they were, the held row
    /// comes back, and the room is gone when they are at rest.
    pub(super) fn put_back_room(&mut self, mut room: DragRoom, cx: &mut Context<Self>) {
        room.put_back();
        self.motion.batch += 1;
        let batch = self.motion.batch;
        if room.joined() && !cx.reduce_motion() {
            let (dragged, split_before) = room.dragged();
            self.motion.join.draw_bracket(dragged, split_before, batch);
        }
        room.over = Some(batch);
        self.motion.room = Some(room);
        self.later(ROOM.settle(super::plan::REST) + super::plan::GRACE, cx, move |this, cx| {
            if this.motion.retire_room(batch) {
                cx.notify();
            }
        });
    }
}
