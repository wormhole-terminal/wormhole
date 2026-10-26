//! What a row of the sidebar looks like at a moment of its coming or going, or of its status changing, as
//! plain numbers: one pure function per motion (`insert`, `fold`, `status`, `column`) from the person's choice, the
//! phase and how far along the motion is, to a [`Pose`] (a status change has a [`status::StatusPose`] of
//! its own: its parts are not a row's). The sidebar draws the pose of the animation's progress and the
//! Settings previews draw the pose of their loop's time, so the preview shows exactly what the list does
//! and the motion has one source. No GPUI, no clock: a test pins every variant's start, end and stages.
//!
//! Durations live here, with the poses, and not in the app's `style::motion::timing`: a pose is a
//! function of the motion's own time, whose stages (a gap that opens, then a row that slides in) are
//! cut at those durations, so a second home for them could only drift from the poses that use them.
//! The few that other things share (the list's move) are named here and used from here.
//!
//! Springs are the one thing a pose does not sample. The rows *around* a row that comes or goes move
//! on the list's own spring (`list::move`, which keeps a row's position and velocity when a second
//! change retargets it), so the sidebar hands that to GPUI and the pose says only which spring
//! ([`Room::Spring`]); a preview, which has no element to hold a spring, reads it with
//! [`SpringSpec::step`] through [`Room::open_at`]. A row's own fade and slide are timed, and those a
//! pose gives.
//!
//! Like the rest of `view` it uses only `theme` below it.

pub mod column;
pub mod fold;
pub mod insert;
pub mod status;

use std::time::Duration;

use super::SpringSpec;

/// How a row stands at a moment of its motion. At rest it is [`Pose::REST`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// How much of the row shows through (0.0 nothing, 1.0 all of it).
    pub opacity: f32,
    /// How far it is moved sideways from its place, in points (negative: to the left).
    pub dx: f32,
    /// How far it is moved down from its place, in points (negative: up).
    pub dy: f32,
    /// The share of its height that is open (1.0 the whole row, 0.0 none of it). Where the motion
    /// moves the rows around by layout ([`Room::Layout`]) the row's height is this share of its own
    /// and the rows below follow it; otherwise it is 1.0 throughout.
    pub clip: f32,
}

impl Pose {
    /// A row standing in its place.
    pub const REST: Self = Self { opacity: 1.0, dx: 0.0, dy: 0.0, clip: 1.0 };
}

/// Whether a row is coming into the list or going from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Enter,
    Exit,
}

/// How long a row that appears fades in under Classic: what the list did before the choice existed
/// (a fade of its move's length, easing out).
pub(super) const CLASSIC_FADE: f32 = 180.0;

/// The spring the list's rows move on when they change place (a reorder, a pin, a row made room for):
/// quick, with a hint of overshoot.
pub const LIST_MOVE: SpringSpec = SpringSpec::SNAP;

/// What makes room for a row that comes, or closes the gap of one that goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Room {
    /// The row's own height opens (or closes) along its pose's `clip`, and the rows around follow
    /// it frame by frame.
    Layout,
    /// The row has its whole height at once and the rows around slide to their places on this spring.
    Spring(SpringSpec),
}

impl Room {
    /// How much of the row's height is open to the rows around, `elapsed` after the motion began: 0.0
    /// closed, 1.0 open. `clip` is the row's pose at that time (what a layout opens along) and `stays`
    /// is how long a leaving row stays in the list before the rows around close the gap (a spring
    /// starts then).
    #[must_use]
    pub fn open_at(self, phase: Phase, stays: Duration, elapsed: Duration, clip: f32) -> f32 {
        match (self, phase) {
            (Self::Layout, _) => clip,
            (Self::Spring(spring), Phase::Enter) => spring.step(elapsed.as_secs_f32()),
            (Self::Spring(spring), Phase::Exit) => {
                1.0 - spring.step(elapsed.saturating_sub(stays).as_secs_f32())
            }
        }
    }
}

/// How far `at` milliseconds into a motion is through its stage that starts at `start` and lasts
/// `len` (both in milliseconds): 0.0 before it, 1.0 after it, linear between.
pub(super) fn progress(at: f32, start: f32, len: f32) -> f32 {
    if len <= 0.0 { f32::from(at >= start) } else { ((at - start) / len).clamp(0.0, 1.0) }
}

/// A length of time in milliseconds.
pub(super) fn millis(ms: f32) -> Duration {
    Duration::from_micros((ms * 1000.0).round() as u64)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{LIST_MOVE, Phase, Room, progress};
    use crate::view::motion::SpringSpec;

    #[test]
    fn a_stage_is_before_inside_and_after() {
        assert_eq!(progress(10.0, 20.0, 40.0), 0.0);
        assert_eq!(progress(40.0, 20.0, 40.0), 0.5);
        assert_eq!(progress(90.0, 20.0, 40.0), 1.0);
        assert_eq!(
            (progress(9.0, 10.0, 0.0), progress(10.0, 10.0, 0.0)),
            (0.0, 1.0),
            "an empty stage is a cut"
        );
    }

    #[test]
    fn a_layout_follows_the_clip_and_a_spring_the_time() {
        let (stays, elapsed) = (Duration::from_millis(100), Duration::from_millis(250));
        assert_eq!(Room::Layout.open_at(Phase::Enter, stays, elapsed, 0.3), 0.3);
        let spring = Room::Spring(SpringSpec::SOFT);
        assert_eq!(spring.open_at(Phase::Enter, stays, Duration::ZERO, 1.0), 0.0, "closed at the start");
        assert!(spring.open_at(Phase::Enter, stays, Duration::from_secs(2), 1.0) > 0.999);
        // A leaving row keeps the gap open until it goes, then it closes on the spring.
        assert_eq!(spring.open_at(Phase::Exit, stays, Duration::from_millis(100), 1.0), 1.0);
        assert!(spring.open_at(Phase::Exit, stays, Duration::from_secs(2), 1.0) < 0.001);
    }

    #[test]
    fn the_list_moves_on_the_quick_spring() {
        assert_eq!(LIST_MOVE, SpringSpec::SNAP);
    }
}
