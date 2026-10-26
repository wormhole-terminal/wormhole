//! A line of the list in a pose: what a row or a group's head does while it comes into the list or
//! goes from it, as one value ([`RowMotion`]) that asks core's poses (`view::motion::pose`) what the
//! line looks like at each moment, and [`animate`], which draws the line there. The Settings previews
//! ask the same poses, so what they show is what this does. Nothing here knows about the list's
//! state or when a motion starts: `list` decides that, this only draws a line at a time of its own.
//! Reduce Motion never reaches it (the list plans no motion then).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::pose::{self, Phase, Pose, Room, fold::Slot};
use wormhole_core::view::motion::{FoldMotion, InsertMotion};

/// What a line is doing as it comes or goes, and so which core pose draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::sidebar) enum RowMotion {
    /// A terminal opening or closing (or a group's head appearing, which is always Classic's fade).
    Insert(InsertMotion),
    /// One of the rows of a group that folds or opens, in its place among them.
    Fold(FoldMotion, Slot),
}

impl RowMotion {
    /// How long the line's own motion takes: the time to come in, or the time a leaving line stays.
    pub(in crate::sidebar) fn duration(self, phase: Phase) -> Duration {
        match self {
            Self::Insert(variant) => pose::insert::duration(variant, phase),
            Self::Fold(variant, slot) => pose::fold::duration(variant, phase, slot.of),
        }
    }

    /// The line `t` (0.0..=1.0) through its [`Self::duration`].
    pub(in crate::sidebar) fn pose(self, phase: Phase, t: f32) -> Pose {
        match self {
            Self::Insert(variant) => pose::insert::pose(variant, phase, t),
            Self::Fold(variant, slot) => pose::fold::pose(variant, phase, t, slot),
        }
    }

    /// What makes room for the line, and closes the gap it leaves.
    pub(in crate::sidebar) fn room(self) -> Room {
        match self {
            Self::Insert(variant) => pose::insert::room(variant),
            Self::Fold(variant, _) => pose::fold::room(variant),
        }
    }

    /// The line's own height opens and closes, so the lines round it follow it in the layout.
    pub(in crate::sidebar) fn opens_by_layout(self) -> bool {
        self.room() == Room::Layout
    }
}

/// How tall a line is when it is whole, as the line itself measured it: a pose that opens the height
/// takes a share of this, which the layout cannot say before the line is laid out. It is read a frame
/// after it is measured, which is why a line starts at no height.
#[derive(Clone, Default)]
pub(in crate::sidebar) struct Natural(Rc<Cell<f32>>);

impl Natural {
    /// Measures what it is a child of: put it in the line's content.
    fn probe(&self) -> impl IntoElement {
        let height = self.0.clone();
        canvas(move |bounds, _, _| height.set(f32::from(bounds.size.height)), |_, _, _, _| {})
            .absolute()
            .size_full()
    }

    fn get(&self) -> f32 {
        self.0.get()
    }
}

/// `line` in the pose `motion` gives it, `t` of the way through its duration, for as long as the
/// animation `id` runs (an id new to this motion, or it would carry on from an earlier one). The line
/// is a child of what moves and keeps the height it has; a pose that opens the height is a box of its
/// own height with the line inside, cut where the box ends.
pub(in crate::sidebar) fn animate(
    line: AnyElement,
    id: impl Into<ElementId>,
    motion: RowMotion,
    phase: Phase,
    natural: &Natural,
) -> AnyElement {
    let by_layout = motion.opens_by_layout();
    let content = div()
        .relative()
        .w_full()
        .flex_none()
        .child(line)
        .when(by_layout, |content| content.child(natural.probe()));
    let natural = natural.clone();
    div()
        .relative()
        .w_full()
        .flex_none()
        .child(content)
        .with_animation(id, Animation::new(motion.duration(phase)), move |held, t| {
            let pose = motion.pose(phase, t);
            held.when(pose.opacity < 1.0, |held| held.opacity(pose.opacity))
                .when(pose.dx != 0.0, |held| held.left(px(pose.dx)))
                .when(pose.dy != 0.0, |held| held.top(px(pose.dy)))
                .when(pose.clip < 1.0, |held| held.h(px(pose.clip * natural.get())).overflow_hidden())
        })
        .into_any_element()
}
