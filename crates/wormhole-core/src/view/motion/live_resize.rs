//! A window being dragged to another size, and what the program in its terminal is told meanwhile,
//! which the design fixed (the picks of the motion plan: no choice of variant, so no `MotionChoices`
//! field). A full-screen program (an agent, vim) draws its whole screen again for every size it is told,
//! so while the window is live-resizing the terminal's PTY is resized at most every [`INTERVAL`], the last
//! size always getting through (trailing), and once more the moment the live resize ends. The grid that is
//! drawn stays at the PTY's size until then. Outside a live resize every size is told at once.
//!
//! Also here: the window's resize steps ([`steps`]) and the arithmetic of the strip a window that is not on
//! the cell grid leaves at its right and bottom edges when the drag ends ([`settle`], [`settled_frame`],
//! [`tiled`]). They are pure timing decisions and geometry: asking the system whether a live resize is going
//! on, hearing it end, and the timers, are the app's.

use std::time::{Duration, Instant};

/// The least time between two sizes told to the program during a live resize.
pub const INTERVAL: Duration = Duration::from_millis(100);

/// A leftover strip narrower than this, in points, is not worth moving the window for: it is the rounding of
/// a fractional cell to the screen's pixels.
pub const SETTLE_MIN: f32 = 1.0;

/// How near a whole number of points a cell's width must be for the window to step by it: AppKit keeps its
/// steps in whole points, so any other width would be followed wrongly on most steps.
pub const WHOLE_POINT: f32 = 0.05;

/// How near a screen's edges a window's sides are for it to count as tiled or snapped to them.
pub const TILE_MARGIN: f32 = 24.0;

/// What the system says about a window being dragged to another size: it began, or it ended (with the edges
/// the person held). `window` is the system's number of the window (`platform::window::number`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Started { window: i64 },
    Ended { window: i64, dragged: Dragged },
}

/// What the gate says about one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Tell the program the size the window gives it now.
    pub apply: bool,
    /// Draw this terminal again after this long, whether or not anything else asks: a size is held back, and
    /// this is when it is due. `None` when nothing is held back.
    pub wake_in: Option<Duration>,
}

/// Decides, frame by frame, whether the program is told the window's size.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResizeGate {
    applied: Option<Instant>,
}

impl ResizeGate {
    /// The frame at `now`: `live` says whether the window is being dragged, `differs` whether the size the
    /// window gives the terminal is not the one the program has. The caller does what [`Step::apply`]
    /// says; the gate counts it as done. The end of a live resize is not seen here: the frame after it has
    /// `live` false and a size that differs, which is told at once.
    pub fn step(&mut self, now: Instant, live: bool, differs: bool) -> Step {
        let since = self.applied.map(|at| now.saturating_duration_since(at));
        let due = since.is_none_or(|since| since >= INTERVAL);
        let apply = differs && (!live || due);
        // Only what is told during a live resize counts towards the interval: a size told just before the
        // person took hold of the window's edge does not make the first one of the drag wait.
        if apply {
            self.applied = live.then_some(now);
        }
        let wake_in = match since {
            Some(since) if live && differs && !apply => Some(INTERVAL.saturating_sub(since)),
            _ => None,
        };
        Step { apply, wake_in }
    }
}

/// The steps the window resizes in for a terminal whose cell is `cell` (width, height in points). AppKit keeps
/// them in whole points, so the width steps only when the cell's width is a whole point already (within
/// [`WHOLE_POINT`]); otherwise it is left free (one point) and [`settle`] takes the strip off at the end. The
/// height is whole already.
#[must_use]
pub fn steps(cell: (f32, f32)) -> (f32, f32) {
    let width = if (cell.0 - cell.0.round()).abs() <= WHOLE_POINT { cell.0.round().max(1.0) } else { 1.0 };
    (width, cell.1.round().max(1.0))
}

/// How far the window is to be made smaller, in points across and down, so that it holds whole cells with no
/// strip left over: `spare` is what the terminal's area has beyond its grid. `None` when nothing worth moving
/// for is left, and never as much as a whole cell (that would be a column or a row less than the person
/// dragged to).
#[must_use]
pub fn settle(spare: (f32, f32), cell: (f32, f32)) -> Option<(f32, f32)> {
    let axis = |spare: f32, cell: f32| if spare >= SETTLE_MIN && spare < cell { spare } else { 0.0 };
    let (dx, dy) = (axis(spare.0, cell.0), axis(spare.1, cell.1));
    (dx > 0.0 || dy > 0.0).then_some((dx, dy))
}

/// A window's frame in AppKit's space: `y` grows up, so it is the bottom edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    #[must_use]
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn top(&self) -> f32 {
        self.y + self.h
    }
}

/// Which edges a drag moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dragged {
    /// The left edge moved (so the right one is the one to keep).
    pub left: bool,
    /// The top edge moved (so the bottom one is the one to keep).
    pub top: bool,
}

/// The edges that moved between the frame the live resize began at and the one it ended at.
#[must_use]
pub fn dragged(start: Frame, end: Frame) -> Dragged {
    Dragged { left: (end.x - start.x).abs() > 0.5, top: (end.top() - start.top()).abs() > 0.5 }
}

/// `frame` made `shrink` smaller across and down, the edges that were not dragged staying where they are: a
/// strip is taken off from the side the person was not holding, and a corner dragged to the top left leaves
/// the bottom right where it is.
#[must_use]
pub fn settled_frame(frame: Frame, shrink: (f32, f32), dragged: Dragged) -> Frame {
    Frame {
        x: if dragged.left { frame.x + shrink.0 } else { frame.x },
        // The bottom edge is `y`: the top is kept by moving the bottom up.
        y: if dragged.top { frame.y } else { frame.y + shrink.1 },
        w: frame.w - shrink.0,
        h: frame.h - shrink.1,
    }
}

/// Whether `frame` sits against both sides, or both the top and the bottom, of the screen's usable area
/// `visible` (within [`TILE_MARGIN`]): the shape the system's tiling and the green button's fill leave, which
/// are its sizes, not the person's, and which a window is not made smaller from. A window the person dragged
/// to the full height or width of the screen by hand counts too; it is left as it is.
#[must_use]
pub fn tiled(frame: Frame, visible: Frame) -> bool {
    let across = frame.x <= visible.x + TILE_MARGIN && frame.right() >= visible.right() - TILE_MARGIN;
    let down = frame.y <= visible.y + TILE_MARGIN && frame.top() >= visible.top() - TILE_MARGIN;
    across || down
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn outside_a_live_resize_every_size_is_told_at_once() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        for ms in [0, 1, 2, 3] {
            assert_eq!(
                gate.step(after(start, ms), false, true),
                Step { apply: true, wake_in: None },
                "at {ms}"
            );
        }
        assert!(!gate.step(after(start, 4), false, false).apply, "and nothing is told when nothing differs");
    }

    #[test]
    fn a_live_resize_tells_the_first_size_at_once_and_then_at_most_every_interval() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        assert!(gate.step(after(start, 0), true, true).apply, "the first size is not made to wait");
        let held = gate.step(after(start, 16), true, true);
        assert!(!held.apply, "a frame later it is held");
        assert_eq!(held.wake_in, Some(Duration::from_millis(84)), "and told when the interval is up");
        let late = gate.step(after(start, 100), true, true);
        assert!(late.apply, "at the interval the size that is there is told");
        assert_eq!(late.wake_in, None, "and nothing is held back then");
    }

    #[test]
    fn a_size_told_just_before_the_drag_does_not_make_its_first_size_wait() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        assert!(gate.step(after(start, 0), false, true).apply, "a resize of any other kind");
        assert!(gate.step(after(start, 10), true, true).apply, "the drag's first size is told at once");
        assert!(!gate.step(after(start, 20), true, true).apply);
    }

    #[test]
    fn the_program_is_told_at_most_ten_times_a_second_over_a_long_drag() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        let told = (0..=1000).step_by(8).filter(|ms| gate.step(after(start, *ms), true, true).apply).count();
        assert!((10..=11).contains(&told), "told {told} times in a second");
    }

    #[test]
    fn the_last_size_is_told_when_the_drag_stops_even_if_no_frame_comes_before_the_wake() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        gate.step(after(start, 0), true, true);
        let held = gate.step(after(start, 30), true, true);
        assert!(!held.apply);
        // The pointer rests and the wake draws the terminal again: still live, the interval is up.
        let wake = gate.step(after(start, 30) + held.wake_in.unwrap(), true, true);
        assert!(wake.apply, "the trailing size gets through without another drag");
    }

    #[test]
    fn the_frame_after_the_end_of_a_live_resize_tells_the_size_at_once() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        gate.step(after(start, 0), true, true);
        assert!(!gate.step(after(start, 10), true, true).apply);
        assert_eq!(gate.step(after(start, 20), false, true), Step { apply: true, wake_in: None });
    }

    #[test]
    fn a_live_resize_that_changes_nothing_asks_for_no_wake() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        assert_eq!(gate.step(after(start, 0), true, false), Step { apply: false, wake_in: None });
    }

    #[test]
    fn only_a_cell_of_whole_points_steps_the_width() {
        assert_eq!(steps((8.0, 18.0)), (8.0, 18.0));
        assert_eq!(steps((8.04, 18.0)), (8.0, 18.0), "a hair off a whole point is whole");
        assert_eq!(steps((8.4, 18.0)), (1.0, 18.0), "8.4 would be followed as 8: left free");
        assert_eq!(steps((7.8, 17.0)), (1.0, 17.0), "and 7.8 as 8");
        assert_eq!(steps((8.4287, 18.4)), (1.0, 18.0));
        assert_eq!(steps((0.2, 0.2)), (1.0, 1.0), "never less than a point");
    }

    #[test]
    fn a_strip_narrower_than_a_cell_is_taken_off_and_a_whole_cell_never() {
        let cell = (8.4, 18.0);
        assert_eq!(settle((3.0, 0.0), cell), Some((3.0, 0.0)));
        assert_eq!(settle((0.0, 11.0), cell), Some((0.0, 11.0)));
        assert_eq!(settle((3.0, 11.0), cell), Some((3.0, 11.0)));
        assert_eq!(settle((0.4, 0.0), cell), None, "rounding to the screen's pixels is left alone");
        assert_eq!(settle((-2.0, 0.0), cell), None, "a grid that overhangs has no strip");
        assert_eq!(settle((8.4, 18.0), cell), None, "a whole cell is a size the person chose");
    }

    const START: Frame = Frame { x: 100.0, y: 100.0, w: 800.0, h: 600.0 };

    #[test]
    fn the_edges_that_moved_are_the_ones_dragged() {
        assert_eq!(
            dragged(START, Frame::new(100.0, 100.0, 850.0, 640.0)),
            Dragged { left: false, top: false }
        );
        assert_eq!(
            dragged(START, Frame::new(100.0, 60.0, 850.0, 640.0)),
            Dragged { left: false, top: false }
        );
        assert_eq!(dragged(START, Frame::new(60.0, 100.0, 840.0, 600.0)), Dragged { left: true, top: false });
        assert_eq!(
            dragged(START, Frame::new(100.0, 100.0, 800.0, 640.0)),
            Dragged { left: false, top: true }
        );
        assert_eq!(dragged(START, Frame::new(60.0, 100.0, 840.0, 640.0)), Dragged { left: true, top: true });
    }

    #[test]
    fn the_strip_comes_off_the_side_that_was_not_held() {
        let frame = Frame::new(100.0, 100.0, 803.0, 611.0);
        let shrink = (3.0, 11.0);
        // The bottom right was dragged: the top left stays (the bottom edge, y, moves up).
        let kept_top_left = settled_frame(frame, shrink, Dragged { left: false, top: false });
        assert_eq!(kept_top_left, Frame::new(100.0, 111.0, 800.0, 600.0));
        assert_eq!(kept_top_left.top(), frame.top(), "the top edge is where it was");
        // The top left was dragged: the bottom right stays.
        let kept_bottom_right = settled_frame(frame, shrink, Dragged { left: true, top: true });
        assert_eq!(kept_bottom_right, Frame::new(103.0, 100.0, 800.0, 600.0));
        assert_eq!(kept_bottom_right.right(), frame.right());
        assert_eq!(kept_bottom_right.y, frame.y);
    }

    #[test]
    fn a_window_against_both_sides_or_the_full_height_is_tiled() {
        let screen = Frame::new(0.0, 0.0, 1600.0, 1000.0);
        assert!(tiled(Frame::new(0.0, 0.0, 800.0, 990.0), screen), "a half tile: the full height");
        assert!(tiled(Frame::new(0.0, 300.0, 1590.0, 400.0), screen), "the full width");
        assert!(!tiled(Frame::new(200.0, 100.0, 900.0, 700.0), screen), "a window of the person's own size");
        assert!(!tiled(Frame::new(0.0, 100.0, 900.0, 700.0), screen), "against one side only");
    }
}
