//! Switching the right panel's tab, which the design fixed (the picks of the motion plan: no choice of
//! variant, so no `MotionChoices` field): the selection springs to the new tab on the quick spring
//! (400/30) and the content slides 10 pt in from the side it came from, fading in, in 140 ms.
//!
//! The selection is one spring on a coordinate that counts tabs (0.0 the first, 1.0 the second, and so on),
//! so a second switch mid-way carries on from where the first was and the spring's overshoot is a
//! fraction of a tab, not a distance the app has to know. [`selection_at`] turns that coordinate and the
//! places the tabs were laid out in into the selection's own place; the app measures the tabs and plays the
//! spring, and a test here pins what the numbers say.

use std::time::Duration;

use super::SpringSpec;
use super::ease::EASE_OUT;

/// The spring the selection moves on.
pub const SELECTION: SpringSpec = SpringSpec::SNAP;
/// The new content sliding and fading in.
pub const CONTENT_IN: Duration = Duration::from_millis(140);
/// How far from its place the new content starts, in points.
pub const CONTENT_SLIDE: f32 = 10.0;

/// Where a tab sits in its bar: from the bar's start, and how wide it is, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub x: f32,
    pub w: f32,
}

/// The selection's place when the spring is at `at` tabs from the first: a tab's own place at a whole
/// number, the one between its neighbours' between two, and past the first or the last tab (a spring
/// that overshoots) the way the nearest two were heading. `None` for no tabs; one tab is where it is.
#[must_use]
pub fn selection_at(slots: &[Slot], at: f32) -> Option<Slot> {
    let last = slots.len().checked_sub(1)?;
    if last == 0 {
        return Some(slots[0]);
    }
    // The pair the coordinate lies between, or the nearest pair when it lies outside them all.
    let lower = (at.floor().max(0.0) as usize).min(last - 1);
    let (from, to) = (slots[lower], slots[lower + 1]);
    let share = at - lower as f32;
    Some(Slot { x: from.x + (to.x - from.x) * share, w: from.w + (to.w - from.w) * share })
}

/// The side of the panel the new content comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// From the left: the tab switched to is before the one left.
    Left,
    /// From the right: the tab switched to is after the one left.
    Right,
}

impl Side {
    /// The side for a switch from the tab at `from` to the one at `to` (their places in the bar).
    #[must_use]
    pub fn of_switch(from: usize, to: usize) -> Self {
        if to < from { Self::Left } else { Self::Right }
    }
}

/// The new content `t` (0.0..=1.0) of the way through [`CONTENT_IN`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Content {
    /// How much of it shows through.
    pub opacity: f32,
    /// How far it is from its place, in points (negative: to the left).
    pub dx: f32,
}

/// The content that came from `side`, `t` of the way through its slide, easing out.
#[must_use]
pub fn content(side: Side, t: f32) -> Content {
    let eased = EASE_OUT.ease(t.clamp(0.0, 1.0));
    let start = match side {
        Side::Left => -CONTENT_SLIDE,
        Side::Right => CONTENT_SLIDE,
    };
    Content { opacity: eased, dx: start * (1.0 - eased) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOTS: [Slot; 3] = [Slot { x: 0.0, w: 28.0 }, Slot { x: 32.0, w: 28.0 }, Slot { x: 64.0, w: 44.0 }];

    #[test]
    fn the_selection_is_the_spring_the_design_picked() {
        assert_eq!(SELECTION, SpringSpec::SNAP);
        assert_eq!(SELECTION, SpringSpec::new(400.0, 30.0));
        assert_eq!(CONTENT_IN, Duration::from_millis(140));
        assert_eq!(CONTENT_SLIDE, 10.0);
    }

    #[test]
    fn at_a_whole_tab_the_selection_is_that_tab() {
        for (at, slot) in SLOTS.iter().enumerate() {
            assert_eq!(selection_at(&SLOTS, at as f32), Some(*slot));
        }
    }

    #[test]
    fn between_two_tabs_it_is_between_their_places_and_their_widths() {
        let half = selection_at(&SLOTS, 1.5).expect("a place");
        assert_eq!(half, Slot { x: 48.0, w: 36.0 });
    }

    #[test]
    fn a_spring_that_overshoots_either_end_goes_on_the_way_it_was_heading() {
        let before = selection_at(&SLOTS, -0.25).expect("a place");
        assert_eq!(before, Slot { x: -8.0, w: 28.0 });
        let after = selection_at(&SLOTS, 2.25).expect("a place");
        assert_eq!(after, Slot { x: 72.0, w: 48.0 });
    }

    #[test]
    fn a_switch_runs_from_the_old_tabs_place_to_the_new_ones_and_ends_there() {
        let (from, to) = (0.0, 2.0);
        let at = |seconds: f32| from + (to - from) * SELECTION.step(seconds);
        assert_eq!(selection_at(&SLOTS, at(0.0)), Some(SLOTS[0]), "it starts where the old tab is");
        let end = selection_at(&SLOTS, at(2.0)).expect("a place");
        assert!((end.x - SLOTS[2].x).abs() < 0.1 && (end.w - SLOTS[2].w).abs() < 0.1, "{end:?}");
    }

    #[test]
    fn no_tabs_have_no_selection_and_one_tab_is_where_it_is() {
        assert_eq!(selection_at(&[], 0.0), None);
        assert_eq!(selection_at(&SLOTS[..1], 3.0), Some(SLOTS[0]));
    }

    #[test]
    fn the_content_comes_from_the_side_of_the_tab() {
        assert_eq!(Side::of_switch(0, 2), Side::Right);
        assert_eq!(Side::of_switch(2, 1), Side::Left);
        let start = content(Side::Right, 0.0);
        assert_eq!(start, Content { opacity: 0.0, dx: 10.0 });
        assert_eq!(content(Side::Left, 0.0), Content { opacity: 0.0, dx: -10.0 });
        assert_eq!(content(Side::Right, 1.0), Content { opacity: 1.0, dx: 0.0 }, "it ends at rest");
        let middle = content(Side::Right, 0.5);
        assert!(middle.opacity > 0.5 && middle.dx < 5.0, "it eases out: {middle:?}");
        // However it is asked, it stays between where it starts and where it rests.
        assert_eq!(content(Side::Right, 7.0), content(Side::Right, 1.0));
        assert_eq!(content(Side::Right, -1.0), start);
    }
}
