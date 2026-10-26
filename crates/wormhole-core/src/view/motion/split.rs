//! A split opening (⌘D, ⇧⌘D), which the design fixed (the picks of the motion plan: no choice of variant,
//! so no `MotionChoices` field): the divider springs. The new pane comes in over the end of the pane that was
//! split on the quick spring (400/30), its edge travelling from the far end of that pane to the middle, while
//! the pane that was split keeps the terminal it had and is clipped back to what the edge leaves it. The
//! terminals are never sized while it moves: the pane that was split is resized once, when the layout it ends
//! in replaces the picture, and the new pane is made the size it has there.
//!
//! [`Opening::of`] says whether a split is such a thing at all: the layout it ends in must be the one before
//! it with one pane's room shared with a new one, and nothing else moved. Whatever else a layout change is (a
//! preset, a drop, a split of a zoomed pane) is a cut, as it always was.

use std::time::Duration;

use super::SpringSpec;
use super::pose::status::SETTLED;
use crate::view::panes::{Axis, PaneId, Rect, TabPanes};

/// The spring the new pane's edge travels on.
pub const SPRING: SpringSpec = SpringSpec::SNAP;

/// How close two edges are to be the same one, in points: the layouts' rectangles come from ratios.
const EDGE_EPS: f32 = 0.5;

/// How long the opening takes: until the spring is at rest.
#[must_use]
pub fn duration() -> Duration {
    SPRING.settle(SETTLED)
}

/// How far the edge has travelled `elapsed` after the opening began: 0.0 where it started, 1.0 where it
/// ends, a little past it on the way (the spring's overshoot). Over, it is exactly 1.0.
#[must_use]
pub fn travelled(elapsed: Duration) -> f32 {
    if elapsed >= duration() { 1.0 } else { SPRING.step(elapsed.as_secs_f32()) }
}

/// A split of one pane into two, as an opening sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
    /// The pane that was split: the one that gave up half of its room.
    pub target: PaneId,
    /// The way the pane was split: the new pane is to the right of the other (`Row`) or below it (`Column`).
    pub axis: Axis,
    /// The room the pane that was split had before.
    pub whole: Rect,
    /// The room the new pane has when it is in: the far end of [`Opening::whole`].
    pub new_pane: Rect,
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= EDGE_EPS
}

fn same(a: Rect, b: Rect) -> bool {
    near(a.x, b.x) && near(a.y, b.y) && near(a.w, b.w) && near(a.h, b.h)
}

impl Opening {
    /// The opening that takes `before` (`None`: the pane stood alone) to `after`, where `new` is the pane that
    /// came, in an area `width` × `height`. `None` unless exactly one pane gave half its room to `new` and
    /// no other pane moved or changed size, and neither layout has a pane zoomed.
    #[must_use]
    pub fn of(
        before: Option<&TabPanes>,
        after: &TabPanes,
        new: PaneId,
        width: f32,
        height: f32,
    ) -> Option<Self> {
        if width <= 0.0
            || height <= 0.0
            || after.zoomed().is_some()
            || before.is_some_and(|b| b.zoomed().is_some())
        {
            return None;
        }
        let rects_after = after.tree().rects(width, height);
        let new_pane = rects_after.iter().find(|(id, _)| *id == new)?.1;
        let rects_before = match before {
            Some(before) => before.tree().rects(width, height),
            // A pane that stood alone filled the area; it is the one that is not new.
            None => match rects_after.iter().filter(|(id, _)| *id != new).collect::<Vec<_>>().as_slice() {
                [(id, _)] => vec![(*id, Rect::new(0.0, 0.0, width, height))],
                _ => return None,
            },
        };
        if rects_after.len() != rects_before.len() + 1 {
            return None;
        }
        // Every pane that was there is where it was, but one: the one that gave up half of its room.
        let mut split = None;
        for &(id, was) in &rects_before {
            let now = rects_after.iter().find(|(other, _)| *other == id)?.1;
            if !same(was, now) && split.replace((id, was, now)).is_some() {
                return None;
            }
        }
        let (target, whole, kept) = split?;
        let across = |a: f32, b: f32, c: f32, d: f32| near(a, b) && near(c, d);
        let axis = if across(kept.y, new_pane.y, kept.h, new_pane.h)
            && across(kept.x, whole.x, new_pane.right(), whole.right())
            && near(kept.right(), new_pane.x)
        {
            Axis::Row
        } else if across(kept.x, new_pane.x, kept.w, new_pane.w)
            && across(kept.y, whole.y, new_pane.bottom(), whole.bottom())
            && near(kept.bottom(), new_pane.y)
        {
            Axis::Column
        } else {
            return None;
        };
        Some(Self { target, axis, whole, new_pane })
    }

    /// Where the new pane's leading edge is along the axis (x for a row, y for a column) when the edge has
    /// travelled `s` of the way: the far end of the pane that was split at 0.0, the middle at 1.0.
    #[must_use]
    pub fn edge_at(&self, s: f32) -> f32 {
        let (from, to) = match self.axis {
            Axis::Row => (self.whole.right(), self.new_pane.x),
            Axis::Column => (self.whole.bottom(), self.new_pane.y),
        };
        from + (to - from) * s
    }

    /// The part of the room the pane that was split still has: all of it at 0.0, what it has once the new
    /// pane is in at 1.0. It is clipped to this and never sized to it.
    #[must_use]
    pub fn kept_at(&self, s: f32) -> Rect {
        let edge = self.edge_at(s);
        match self.axis {
            Axis::Row => Rect::new(self.whole.x, self.whole.y, edge - self.whole.x, self.whole.h),
            Axis::Column => Rect::new(self.whole.x, self.whole.y, self.whole.w, edge - self.whole.y),
        }
    }

    /// The new pane's room: its own size, moved so that its leading edge is where [`Opening::edge_at`] says.
    #[must_use]
    pub fn new_at(&self, s: f32) -> Rect {
        let edge = self.edge_at(s);
        match self.axis {
            Axis::Row => Rect { x: edge, ..self.new_pane },
            Axis::Column => Rect { y: edge, ..self.new_pane },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: (f32, f32) = (1000.0, 600.0);

    fn lone_split(axis: Axis) -> (TabPanes, Opening) {
        let mut after = TabPanes::single(1);
        assert!(after.split(axis, 2));
        let opening = Opening::of(None, &after, 2, AREA.0, AREA.1).expect("an opening");
        (after, opening)
    }

    #[test]
    fn a_lone_pane_split_to_the_right_gives_the_new_pane_the_far_half() {
        let (_, opening) = lone_split(Axis::Row);
        assert_eq!(opening.axis, Axis::Row);
        assert_eq!(opening.target, 1, "the pane that stood alone");
        assert_eq!(opening.whole, Rect::new(0.0, 0.0, 1000.0, 600.0));
        assert_eq!(opening.new_pane, Rect::new(500.0, 0.0, 500.0, 600.0));
    }

    #[test]
    fn a_lone_pane_split_down_gives_the_new_pane_the_lower_half() {
        let (_, opening) = lone_split(Axis::Column);
        assert_eq!(opening.axis, Axis::Column);
        assert_eq!(opening.new_pane, Rect::new(0.0, 300.0, 1000.0, 300.0));
    }

    #[test]
    fn the_edge_starts_at_the_far_end_and_ends_in_the_middle_and_the_old_pane_is_clipped_to_it() {
        let (_, opening) = lone_split(Axis::Row);
        assert_eq!(opening.edge_at(0.0), 1000.0);
        assert_eq!(opening.edge_at(1.0), 500.0);
        assert_eq!(opening.kept_at(0.0), opening.whole, "at the start the pane has all of its room");
        assert_eq!(
            opening.kept_at(1.0),
            Rect::new(0.0, 0.0, 500.0, 600.0),
            "at the end, what the layout gives it"
        );
        assert_eq!(opening.new_at(0.0).x, 1000.0, "the new pane waits at the end, out of the room");
        assert_eq!(opening.new_at(1.0), opening.new_pane, "and comes to where the layout puts it");
        // Its size is never touched, only where it is.
        for s in [0.0, 0.3, 1.0, 1.04] {
            let at = opening.new_at(s);
            assert_eq!((at.w, at.h, at.y), (500.0, 600.0, 0.0));
        }
        // The old pane and the new one always meet at the edge.
        for s in [0.0, 0.3, 0.7, 1.0, 1.04] {
            assert!((opening.kept_at(s).right() - opening.new_at(s).x).abs() < 1e-3);
        }
    }

    #[test]
    fn a_pane_split_inside_a_split_opens_over_its_own_room_and_leaves_the_others_alone() {
        let mut before = TabPanes::single(1);
        assert!(before.split(Axis::Row, 2));
        // Pane 2 (focused) is split down: it gives half of its room to 3.
        let mut after = before.clone();
        assert!(after.split(Axis::Column, 3));
        let opening = Opening::of(Some(&before), &after, 3, AREA.0, AREA.1).expect("an opening");
        assert_eq!(opening.axis, Axis::Column);
        assert_eq!(opening.target, 2, "the focused pane was split");
        assert_eq!(opening.whole, Rect::new(500.0, 0.0, 500.0, 600.0), "pane 2's room");
        assert_eq!(opening.new_pane, Rect::new(500.0, 300.0, 500.0, 300.0));
        // Along the parent's own axis the new pane is a sibling and takes half of its target's share.
        let mut after = before.clone();
        assert!(after.split(Axis::Row, 3));
        let opening = Opening::of(Some(&before), &after, 3, AREA.0, AREA.1).expect("an opening");
        assert_eq!(opening.axis, Axis::Row);
        assert_eq!(opening.whole, Rect::new(500.0, 0.0, 500.0, 600.0));
        assert_eq!(opening.new_pane, Rect::new(750.0, 0.0, 250.0, 600.0));
    }

    #[test]
    fn what_is_not_one_pane_giving_half_of_its_room_is_a_cut() {
        let (after, _) = lone_split(Axis::Row);
        // A pane that is not in the layout, or an area that is not known yet.
        assert_eq!(Opening::of(None, &after, 9, AREA.0, AREA.1), None);
        assert_eq!(Opening::of(None, &after, 2, 0.0, 0.0), None);
        // Two panes that were there are not a lone pane.
        assert_eq!(Opening::of(None, &after, 2, AREA.0, AREA.1).map(|o| o.axis), Some(Axis::Row));
        let mut three = after.clone();
        assert!(three.split(Axis::Row, 3));
        assert_eq!(Opening::of(None, &three, 3, AREA.0, AREA.1), None, "three panes from none");
        // A pane zoomed before or after: the others were not on screen.
        let mut zoomed = after.clone();
        assert!(zoomed.toggle_zoom());
        assert_eq!(Opening::of(None, &zoomed, 2, AREA.0, AREA.1), None);
        let mut four = three.clone();
        assert!(four.split(Axis::Row, 4));
        assert_eq!(Opening::of(Some(&after), &four, 4, AREA.0, AREA.1), None, "two panes came");
    }

    #[test]
    fn a_panes_share_that_changed_for_another_reason_is_a_cut() {
        let mut before = TabPanes::single(1);
        assert!(before.split(Axis::Row, 2));
        let mut after = before.clone();
        assert!(after.split(Axis::Row, 3));
        let divider = after.tree().dividers(AREA.0, AREA.1).first().cloned().expect("a divider");
        after.drag(&divider.id, 300.0, AREA.0, AREA.1, 160.0, 96.0);
        assert_eq!(Opening::of(Some(&before), &after, 3, AREA.0, AREA.1), None, "the first pane moved too");
    }

    #[test]
    fn the_edge_travels_on_the_quick_spring_and_the_opening_ends_exactly_at_rest() {
        assert_eq!(SPRING, SpringSpec::SNAP);
        assert_eq!(travelled(Duration::ZERO), 0.0);
        assert_eq!(travelled(duration()), 1.0);
        assert_eq!(travelled(Duration::from_secs(5)), 1.0);
        let quarter = travelled(Duration::from_millis(60));
        assert!((quarter - SPRING.step(0.060)).abs() < 1e-6);
        let peak = (1..100).map(|ms| travelled(Duration::from_millis(ms * 5))).fold(0.0, f32::max);
        assert!((1.005..1.1).contains(&peak), "a hint of overshoot: {peak}");
        let time = duration();
        assert!((0.15..0.6).contains(&time.as_secs_f32()), "{time:?}");
    }
}
