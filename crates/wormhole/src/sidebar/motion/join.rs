//! Dropping a terminal on the middle of another row, which makes a split: the row it would join tints
//! and shows a split mark, the dragged chip snaps onto it on a spring, and on the drop the split's bracket
//! draws down the two rows while they settle. Leaving the row undoes the preview on the same timings.
//! `room` opens the gap for a row that only moves; this is what is shown instead when the pointer is over
//! the middle of a row, and which row that is, is the drop's own answer (`DropMark::Join`).
//!
//! The chip is GPUI's own drag picture, drawn at the pointer in a view of its own that the sidebar cannot
//! reach: what it snaps to is handed over through a [`Snap`] both hold, and the chip reads it every frame.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::EASE_OUT;
use wormhole_core::view::motion::drag::{BRACKET, FADE_OUT, LIFT, bracket};
use wormhole_core::view::sidebar::{Bracket, SplitId};

use super::list::{ItemKey, id_of};
use crate::settings::UiColors;
use crate::sidebar::drop::DropMark;
use crate::sidebar::{Sidebar, SidebarInput};
use crate::style;

/// Where the chip's top left lands on the row it snaps to: this far in from the row's left and down from
/// its top, which puts a one-line chip over the row's first line.
const SNAP_INSET: (f32, f32) = (16.0, 8.0);

/// What the dragged chip snaps onto: the sidebar says where while the pointer is over the middle of a
/// row, the chip (a view of its own) reads it each frame.
#[derive(Clone, Default)]
pub(crate) struct Snap(Rc<SnapAt>);

#[derive(Default)]
struct SnapAt {
    on: Cell<bool>,
    at: Cell<Point<Pixels>>,
}

impl Snap {
    /// Whether the chip is on its way to the row, and where its top left lands (a place in the window).
    /// The place stays what it was when the chip comes off the row, so it can leave it from there.
    pub(crate) fn target(&self) -> (bool, Point<Pixels>) {
        (self.0.on.get(), self.0.at.get())
    }

    fn set(&self, at: Option<Point<Pixels>>) {
        self.0.on.set(at.is_some());
        if let Some(at) = at {
            self.0.at.set(at);
        }
    }
}

/// Which way a preview's tint goes.
#[derive(Clone, Copy)]
enum Tint {
    In,
    Out,
}

/// The preview of a split: which row it is on, and for how long the preview of one that was left is
/// still going out, and the bracket of the split just made.
#[derive(Default)]
pub(super) struct Join {
    /// The row the dragged one would join now.
    target: Option<i64>,
    /// How many rows have been the target, so each one's preview starts afresh.
    epoch: u64,
    /// The row that was the target, and the count it had, while its preview goes out.
    leaving: Option<(i64, u64)>,
    /// The terminal that was dropped on a row, the split it was in before, and the batch that ends the
    /// bracket's drawing, if the drop made a split.
    drawing: Option<(i64, Option<SplitId>, u64)>,
    snap: Snap,
}

impl Join {
    /// The count of the preview on terminal `id`'s row, if it is the target.
    fn epoch_of(&self, id: i64) -> Option<u64> {
        (self.target == Some(id)).then_some(self.epoch)
    }

    /// The target is now `target`. Returns the preview this left, as the row and its count, which goes out
    /// (and which the caller ends later).
    fn aim(&mut self, target: Option<i64>) -> Option<(i64, u64)> {
        if self.target == target {
            return None;
        }
        let left = self.target.map(|id| (id, self.epoch));
        if target.is_some() {
            self.epoch += 1;
        }
        self.target = target;
        if left.is_some() {
            self.leaving = left;
        }
        left
    }

    /// The preview of a row that was left is over: it is not drawn any more.
    fn forget(&mut self, left: (i64, u64)) -> bool {
        let over = self.leaving == Some(left);
        if over {
            self.leaving = None;
        }
        over
    }

    /// A drop of `joined` on a row (it was in the split `before`) has its bracket drawn for `batch`'s time,
    /// if it made a split: whether it did is read off the window's facts when a row asks (`bracket_of`),
    /// not off the preview, which a refused join or a drag let go elsewhere leaves behind.
    pub(super) fn draw_bracket(&mut self, joined: i64, before: Option<SplitId>, batch: u64) {
        self.drawing = Some((joined, before, batch));
    }

    /// The bracket of the batch has been drawn.
    pub(super) fn retire(&mut self, batch: u64) -> bool {
        let over = self.drawing.is_some_and(|(_, _, drawn)| drawn == batch);
        if over {
            self.drawing = None;
        }
        over
    }

    /// The batch the bracket of a row in `split` is drawing in: the split the drop just put the dragged
    /// terminal in, which is not the one it was in before.
    fn bracket_of(&self, input: &SidebarInput, split: Option<SplitId>) -> Option<u64> {
        let (joined, before, batch) = self.drawing?;
        let own = input.rows.get(input.find(joined)?)?.split;
        (own.is_some() && own != before && split == own).then_some(batch)
    }

    /// The preview that is going out on `key`'s row.
    fn leaving_on(&self, key: &ItemKey) -> Option<u64> {
        match (self.leaving, key) {
            (Some((id, epoch)), ItemKey::Row(row)) if id == *row => Some(epoch),
            _ => None,
        }
    }
}

/// `line`, a split's bracket along a row, drawn in as the split is made: down the rows, so the first row's
/// stretch is there before the last one's.
pub(in crate::sidebar) fn draw_bracket(line: Div, id: i64, batch: u64, part: Bracket) -> AnyElement {
    let (from, to) = match part {
        Bracket::Start => (0.0, 0.5),
        Bracket::Mid => (0.25, 0.75),
        Bracket::End => (0.5, 1.0),
    };
    line.with_animation(
        id_of("bracket", &ItemKey::Row(id), batch),
        Animation::new(BRACKET),
        move |line, t| line.opacity(bracket(t, from, to)),
    )
    .into_any_element()
}

/// The tint and the split mark over the row of terminal `id`, the `epoch`th to be the target, coming in
/// or going out.
fn preview(ui: UiColors, id: i64, epoch: u64, tint: Tint) -> AnyElement {
    let (name, length) = match tint {
        Tint::In => ("join-in", LIFT),
        Tint::Out => ("join-out", FADE_OUT),
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(style::row_drop_target(ui))
        .child(style::split_mark(ui))
        .with_animation(id_of(name, &ItemKey::Row(id), epoch), Animation::new(length), move |layer, t| {
            let eased = EASE_OUT.ease(t);
            layer.opacity(match tint {
                Tint::In => eased,
                Tint::Out => 1.0 - eased,
            })
        })
        .into_any_element()
}

#[cfg(test)]
impl Sidebar {
    pub(in crate::sidebar) fn join_preview_state(&self) -> (Option<i64>, Option<i64>, bool) {
        self.motion.join.state()
    }

    pub(in crate::sidebar) fn join_bracket_rows(&self) -> Vec<i64> {
        self.motion.join.bracket_rows(&self.input)
    }
}

#[cfg(test)]
impl Join {
    /// The terminal whose row the dragged one would join, the one whose preview is going out, and whether
    /// the chip is snapping onto a row.
    pub(in crate::sidebar) fn state(&self) -> (Option<i64>, Option<i64>, bool) {
        (self.target, self.leaving.map(|(id, _)| id), self.snap.target().0)
    }

    /// The terminals whose rows draw their bracket in now.
    pub(in crate::sidebar) fn bracket_rows(&self, input: &SidebarInput) -> Vec<i64> {
        input
            .rows
            .iter()
            .filter(|row| self.bracket_of(input, row.split).is_some())
            .map(|row| row.id)
            .collect()
    }
}

impl Sidebar {
    /// What the chip that follows the pointer snaps onto, which the picture of a drag reads (`terminal_drag`).
    pub(in crate::sidebar) fn snap(&self) -> Snap {
        self.motion.join.snap.clone()
    }

    /// The batch a row in `split` draws its bracket in, while the split the drop just made draws it.
    pub(in crate::sidebar) fn bracket_draws(&self, split: Option<SplitId>) -> Option<u64> {
        self.motion.join.bracket_of(&self.input, split)
    }

    /// What lies over the row `key` for a split: the preview coming in on the row the dragged one would
    /// join, or going out on one it has left. `None` for any other line, and when the Motion feature is
    /// off (the row's tint is then the plain one, `mark_element`'s).
    pub(in crate::sidebar) fn join_preview(
        &self,
        mark: Option<&DropMark>,
        key: &ItemKey,
        ui: UiColors,
    ) -> Option<AnyElement> {
        let join = &self.motion.join;
        if let (Some(DropMark::Join(id)), ItemKey::Row(row)) = (mark, key)
            && id == row
        {
            return join.epoch_of(*id).map(|epoch| preview(ui, *id, epoch, Tint::In));
        }
        let epoch = join.leaving_on(key)?;
        let ItemKey::Row(id) = key else { return None };
        Some(preview(ui, *id, epoch, Tint::Out))
    }

    /// Follows the pointer's target for a drag in flight: which row it would join, and where the chip
    /// snaps. Call it once per frame of the list with `mark` where the drop would land.
    pub(in crate::sidebar) fn follow_join(&mut self, mark: Option<&DropMark>, cx: &mut Context<Self>) {
        if self.row_in_flight(cx).is_none() {
            return;
        }
        let target = match mark {
            Some(DropMark::Join(id)) => Some(*id),
            _ => None,
        };
        if let Some(left) = self.motion.join.aim(target) {
            self.end_preview_later(left, cx);
        }
        let at = target.and_then(|id| self.snap_place(id));
        self.motion.join.snap.set(at);
    }

    /// The drag ended: the row that was the target stops being it, and its preview goes out.
    pub(super) fn leave_join(&mut self, cx: &mut Context<Self>) {
        if let Some(left) = self.motion.join.aim(None) {
            self.end_preview_later(left, cx);
        }
        self.motion.join.snap.set(None);
    }

    /// Where the chip's top left lands on terminal `id`'s row, in the window.
    fn snap_place(&self, id: i64) -> Option<Point<Pixels>> {
        let at = self.motion.drawn.iter().position(|key| *key == ItemKey::Row(id))?;
        let row = self.scroll.bounds_for_item(at)?;
        Some(row.origin + point(px(SNAP_INSET.0), px(SNAP_INSET.1)))
    }

    /// The preview of the row `left` (and its count) is over when it has gone out.
    fn end_preview_later(&mut self, left: (i64, u64), cx: &mut Context<Self>) {
        self.later(FADE_OUT + super::plan::GRACE, cx, move |this, cx| {
            if this.motion.join.forget(left) {
                cx.notify();
            }
        });
    }
}
