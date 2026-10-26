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
use std::hash::{DefaultHasher, Hash, Hasher};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::drag::ROOM;
use wormhole_core::view::sidebar::Listing;
use wormhole_core::view::sidebar::motion::{Drawn, Move, Moves};
use wormhole_core::view::sidebar::place::Drop;
use wormhole_core::view::sidebar::room::{make_room, run_of};

use super::list::{ItemKey, ListMotion, Spring, measured};
use crate::settings::settings_in;
use crate::sidebar::Sidebar;
use crate::sidebar::drag::DropMark;
use crate::sidebar::terminal_drag::DragOrigin;
use crate::style::motion::SPRING_SNAP;
use wormhole_core::config::features::FeatureFlag;

/// The gap a dragged row has the list open for it.
pub(super) struct Gap {
    /// Every line as laid out with nothing pushed: the places the springs run in.
    lines: Vec<Drawn<ItemKey>>,
    /// How far each line that has been pushed in this drag is pushed now (0 for one that has gone back).
    /// A line stays here until the drag is over and at rest, so its spring is never dropped while it moves.
    pushed: HashMap<ItemKey, f32>,
    /// The dragged lines (the row, and the rows of its split that move with it), by whether they are out
    /// of the list now: while a gap is open for them, which is when the drop would move them.
    lifted: HashMap<ItemKey, bool>,
    /// What the drop in flight does is make a split: the rows settle on the soft spring.
    joining: bool,
    /// The terminal being dragged.
    dragged: i64,
    /// The drag has ended, and the timer of this batch ends the gap.
    over: Option<u64>,
}

impl Gap {
    /// The room of a drag that begins now, over `lines` as laid out.
    fn begin(lines: Vec<Drawn<ItemKey>>, dragged: i64) -> Self {
        Self { lines, pushed: HashMap::new(), lifted: HashMap::new(), joining: false, dragged, over: None }
    }

    fn top(&self, key: &ItemKey) -> Option<f32> {
        self.lines.iter().find(|line| line.key == *key).map(|line| line.top)
    }

    /// The spring a pushed line runs on: from its place, to its place and how far it is pushed.
    pub(super) fn spring_of(&self, key: &ItemKey) -> Option<Spring> {
        let (&by, top) = (self.pushed.get(key)?, self.top(key)?);
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

    /// Whether the drop that ended this drag made a split.
    pub(super) fn joined(&self) -> bool {
        self.joining
    }

    /// The terminal that was dragged.
    pub(super) fn dragged(&self) -> i64 {
        self.dragged
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

    /// `lines`, as drawn, become where they look to be when the drag ends: pushed lines where they were
    /// pushed to, the dragged lines (stacked) where the pointer let go, at `pointer` in the content.
    pub(super) fn seen(&self, lines: &mut [Drawn<ItemKey>], pointer: f32) {
        let height: f32 =
            lines.iter().filter(|line| self.lifted.contains_key(&line.key)).map(|l| l.height).sum();
        let mut at = (pointer - height / 2.0).max(0.0);
        for line in lines.iter_mut() {
            if self.lifted.contains_key(&line.key) {
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
        let Some(gap) = &self.gap else { return Vec::new() };
        gap.lines
            .iter()
            .filter_map(|line| match (&line.key, gap.pushed.get(&line.key)) {
                (ItemKey::Row(id), Some(&by)) if by != 0.0 => Some((*id, by)),
                _ => None,
            })
            .collect()
    }

    /// The terminals whose rows are out of the list while a gap is open for them.
    pub(in crate::sidebar) fn lifted_rows(&self) -> Vec<i64> {
        let Some(gap) = &self.gap else { return Vec::new() };
        let mut ids: Vec<i64> = gap
            .lifted_keys()
            .filter_map(|key| match key {
                ItemKey::Row(id) => Some(*id),
                ItemKey::Band(_) => None,
            })
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Whether a dragged row's gap is still held (drawn, or on its way back to rest).
    pub(in crate::sidebar) fn has_gap(&self) -> bool {
        self.gap.is_some()
    }
}

impl ListMotion {
    /// The room has been at rest since the drag ended in `batch`: it is not drawn any more.
    pub(super) fn retire_room(&mut self, batch: u64) -> bool {
        let ended = self.gap.as_ref().is_some_and(|gap| gap.over == Some(batch));
        if ended {
            self.gap = None;
        }
        // A split's bracket drawn while the lines went back is drawn.
        let drawn = self.join.retire(batch);
        ended || drawn
    }
}

/// `line` (a dragged row) with the spring that takes it out of the list and back: it is gone while a gap
/// is open for it, because the lines that stepped aside stand where it was.
pub(super) fn lifted(line: AnyElement, key: &ItemKey, lifted: bool) -> AnyElement {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    div()
        .relative()
        .w_full()
        .child(line)
        .with_spring(
            ("drag-lift", hasher.finish()),
            SpringAnimation::new(SPRING_SNAP).to(lifted).from(false),
            |line, gone| line.opacity(1.0 - gone.0.clamp(0.0, 1.0)),
        )
        .into_any_element()
}

impl Sidebar {
    /// Whether the person's Motion feature is on: the drag moves as it always did without it.
    pub(in crate::sidebar) fn motion_on(&self, cx: &App) -> bool {
        settings_in(&self.input.profile, cx).feature_enabled(FeatureFlag::Motion)
    }

    /// The terminal being dragged by its row, while the list moves for it.
    pub(in crate::sidebar) fn row_in_flight(&self, cx: &App) -> Option<i64> {
        let (id, origin) = self.input.terminal_drag?;
        (cx.has_active_drag() && origin == DragOrigin::Row && self.motion_on(cx)).then_some(id)
    }

    /// Opens the gap the drag in flight asks for and shuts the one it left: call it once per frame of
    /// the list, before its lines are drawn, with `mark` where the drop would land. Plans nothing while
    /// no row is dragged.
    pub(in crate::sidebar) fn follow_drag(
        &mut self,
        keys: &[ItemKey],
        listing: &Listing,
        mark: Option<&DropMark>,
        cx: &mut Context<Self>,
    ) {
        let Some(dragged) = self.row_in_flight(cx) else { return };
        let mut gap = match self.motion.gap.take() {
            Some(gap) if !gap.is_over() => gap,
            _ => match measured(&self.motion.drawn, &self.scroll) {
                Some(lines) => Gap::begin(lines, dragged),
                None => return,
            },
        };
        let input = self.input.clone();
        let plan = self.over.and_then(|over| Self::terminal_drop_plan(&input, listing, dragged, over));
        gap.pushed.values_mut().for_each(|by| *by = 0.0);
        gap.lifted.values_mut().for_each(|lifted| *lifted = false);
        gap.joining = matches!(plan, Some(Drop::Join { .. }));
        let own = input.find(dragged);
        if let (Some(own), Some(Drop::Move { leave_split, with_mates, .. })) = (own, plan) {
            // The rows of its split go with it when the drop takes them along, else it goes alone.
            let split_of = if with_mates && !leave_split { input.split_of() } else { Vec::new() };
            let run = run_of(listing, &split_of, own).and_then(|run| keys.get(run));
            let slot = mark.and_then(|mark| mark.slot(keys));
            if let (Some(run), Some(slot)) = (run, slot) {
                let at = |key: &ItemKey| gap.lines.iter().position(|line| line.key == *key);
                let slot = keys.get(slot).map_or(Some(gap.lines.len()), at);
                let place = run.iter().filter_map(at).fold(None, |span: Option<(usize, usize)>, at| {
                    Some(span.map_or((at, at + 1), |(start, end)| (start.min(at), end.max(at + 1))))
                });
                for key in run {
                    gap.lifted.insert(key.clone(), true);
                }
                if let (Some(slot), Some((start, end))) = (slot, place) {
                    for (key, by) in make_room(&gap.lines, start..end, slot) {
                        gap.pushed.insert(key, by);
                    }
                }
            }
        }
        // The held row is known from the first frame, so the spring that lifts it starts from the list.
        if let Some(own) =
            own.and_then(|own| keys.get(listing.entries.iter().position(|e| entry_is(e, own))?))
        {
            gap.lifted.entry(own.clone()).or_insert(false);
        }
        self.motion.gap = Some(gap);
    }

    /// The drag ended without the list changing: the lines go back to where they were, the held row
    /// comes back, and the room is gone when they are at rest.
    pub(super) fn put_back_room(&mut self, mut gap: Gap, cx: &mut Context<Self>) {
        gap.put_back();
        self.motion.batch += 1;
        let batch = self.motion.batch;
        if gap.joined() && !cx.reduce_motion() {
            self.motion.join.draw_bracket(gap.dragged(), batch);
        }
        gap.over = Some(batch);
        self.motion.gap = Some(gap);
        self.later(ROOM.settle(super::plan::REST) + super::plan::GRACE, cx, move |this, cx| {
            if this.motion.retire_room(batch) {
                cx.notify();
            }
        });
    }
}

/// Whether `entry` is the row of the terminal at `session`.
fn entry_is(entry: &wormhole_core::view::sidebar::Entry, session: usize) -> bool {
    matches!(entry, wormhole_core::view::sidebar::Entry::Row { session: at, .. } if *at == session)
}
