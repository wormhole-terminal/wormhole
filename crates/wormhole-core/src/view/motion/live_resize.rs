//! A window being dragged to another size, and what the program in its terminal is told meanwhile,
//! which the design fixed (the picks of the motion plan: no choice of variant, so no `MotionChoices`
//! field). A full-screen program (an agent, vim) draws its whole screen again for every size it is told,
//! so while the window is live-resizing the terminal's PTY is resized at most every [`INTERVAL`], the last
//! size always getting through (trailing), and once more the moment the live resize ends. The grid that is
//! drawn stays at the PTY's size until then. Outside a live resize every size is told at once.
//!
//! Also here: the rule for the strip a window that is not on the cell grid leaves at its right and bottom
//! edges when the drag ends ([`settle`]). It is a pure timing decision and some arithmetic: asking the
//! system whether a live resize is going on, and the timers, are the app's.

use std::time::{Duration, Instant};

/// The least time between two sizes told to the program during a live resize.
pub const INTERVAL: Duration = Duration::from_millis(100);

/// A leftover strip narrower than this, in points, is not worth moving the window for: it is the rounding of
/// a fractional cell to the screen's pixels.
pub const SETTLE_MIN: f32 = 1.0;

/// What the gate says about one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Tell the program the size the window gives it now.
    pub apply: bool,
    /// The live resize that was going on at the last frame is over.
    pub ended: bool,
    /// Draw this terminal again after this long, whether or not anything else asks: the size that is held
    /// back is told then, and the end of the live resize is noticed. `None` outside a live resize.
    pub wake_in: Option<Duration>,
}

/// Decides, frame by frame, whether the program is told the window's size.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResizeGate {
    live: bool,
    applied: Option<Instant>,
}

impl ResizeGate {
    /// The frame at `now`: `live` says whether the window is being dragged, `differs` whether the size the
    /// window gives the terminal is not the one the program has. The caller does what [`Step::apply`]
    /// says; the gate counts it as done.
    pub fn step(&mut self, now: Instant, live: bool, differs: bool) -> Step {
        let ended = self.live && !live;
        self.live = live;
        let since = self.applied.map(|at| now.saturating_duration_since(at));
        let due = since.is_none_or(|since| since >= INTERVAL);
        let apply = differs && (!live || due);
        // Only what is told during a live resize counts towards the interval: a size told just before the
        // person took hold of the window's edge does not make the first one of the drag wait.
        if apply {
            self.applied = live.then_some(now);
        }
        let wake_in = live.then(|| match since {
            // A size is held back: it is told when the interval since the last one is up.
            Some(since) if differs && !apply => INTERVAL.saturating_sub(since),
            _ => INTERVAL,
        });
        Step { apply, ended, wake_in }
    }

    /// Whether the last frame was in a live resize.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live
    }
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
            let step = gate.step(after(start, ms), false, true);
            assert_eq!(step, Step { apply: true, ended: false, wake_in: None }, "at {ms} ms");
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
        assert_eq!(late.wake_in, Some(INTERVAL));
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
    fn the_end_of_a_live_resize_tells_the_size_at_once_and_says_it_ended() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        gate.step(after(start, 0), true, true);
        assert!(!gate.step(after(start, 10), true, true).apply);
        let end = gate.step(after(start, 20), false, true);
        assert_eq!(end, Step { apply: true, ended: true, wake_in: None });
        assert!(!gate.step(after(start, 30), false, false).ended, "it ends once");
    }

    #[test]
    fn a_live_resize_that_changes_nothing_keeps_watching_for_its_end() {
        let mut gate = ResizeGate::default();
        let start = Instant::now();
        let step = gate.step(after(start, 0), true, false);
        assert_eq!(step, Step { apply: false, ended: false, wake_in: Some(INTERVAL) });
        assert!(gate.step(after(start, 100), false, false).ended);
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
}
