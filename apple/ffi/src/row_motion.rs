//! What a row of the sidebar looks like at a moment of its coming or going, and of its group folding or opening
//! (`wormhole_core::view::motion::pose`): the core's per-variant tables, handed over as numbers. The person's choice
//! (`motion.insert`, `motion.fold`) is a name; the view draws the pose at the motion's progress, so the list does what
//! the Settings page previews, from one source.

use wormhole_core::view::motion::pose::fold::{self, Slot};
use wormhole_core::view::motion::pose::{Phase, Pose, Room, insert, status};
use wormhole_core::view::motion::{FoldMotion, InsertMotion, MotionChoice, StatusMotion};
use wormhole_core::view::sidebar::Urgency;

/// How a row stands: how much shows through, how far it is moved, and the share of its height that is open.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct RowPoseRecord {
    pub opacity: f32,
    pub dx: f32,
    pub dy: f32,
    pub clip: f32,
}

impl From<Pose> for RowPoseRecord {
    fn from(pose: Pose) -> Self {
        Self { opacity: pose.opacity, dx: pose.dx, dy: pose.dy, clip: pose.clip }
    }
}

/// How long a motion takes and what makes room for the rows around.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RowMotionRecord {
    /// A row coming in: the time it takes; a row going: the time it stays in the list drawn as a pose (0: it is cut).
    pub duration_ms: f32,
    /// `Some((stiffness, damping))`: the rows around move on that spring; `None`: the row's own height opens or closes
    /// along its pose's `clip` and the rows around follow frame by frame.
    pub room_spring: Option<Vec<f32>>,
}

fn phase(entering: bool) -> Phase {
    if entering { Phase::Enter } else { Phase::Exit }
}

fn record(duration: std::time::Duration, room: Room) -> RowMotionRecord {
    RowMotionRecord {
        duration_ms: duration.as_secs_f32() * 1000.0,
        room_spring: match room {
            Room::Layout => None,
            Room::Spring(spring) => Some(vec![spring.stiffness, spring.damping]),
        },
    }
}

fn insert_variant(name: &str) -> InsertMotion {
    InsertMotion::from_name(name).unwrap_or_default()
}

fn fold_variant(name: &str) -> FoldMotion {
    FoldMotion::from_name(name).unwrap_or_default()
}

/// A terminal arriving in the list (`entering`) or leaving it, by the `motion.insert` choice `variant`.
#[uniffi::export]
pub fn insert_row_motion(variant: String, entering: bool) -> RowMotionRecord {
    let variant = insert_variant(&variant);
    record(insert::duration(variant, phase(entering)), insert::room(variant))
}

/// The arriving or leaving row at `t` (0 to 1) of its motion.
#[uniffi::export]
pub fn insert_row_pose(variant: String, entering: bool, t: f32) -> RowPoseRecord {
    insert::pose(insert_variant(&variant), phase(entering), t).into()
}

/// The rows of a group of `rows` folding away or opening, by the `motion.fold` choice `variant`.
#[uniffi::export]
pub fn fold_row_motion(variant: String, entering: bool, rows: u32) -> RowMotionRecord {
    let variant = fold_variant(&variant);
    record(fold::duration(variant, phase(entering), rows as usize), fold::room(variant))
}

/// Row `index` of `rows` in a group's fold, at `t` (0 to 1) of the group's motion.
#[uniffi::export]
pub fn fold_row_pose(variant: String, entering: bool, t: f32, index: u32, rows: u32) -> RowPoseRecord {
    let slot = Slot { index: index as usize, of: rows as usize };
    fold::pose(fold_variant(&variant), phase(entering), t, slot).into()
}

/// How a tile and its row stand part-way through a status change (`pose::status`): how much of the old status's light, edge
/// and badge still shows (`leaving`), the new light or edge (`ring`), the new badge, the ring a flare sends out of the
/// tile's edge, the card's wash, the row's sideways nudge in points, and how far the second line has rolled.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct StatusPoseRecord {
    pub leaving: f32,
    pub ring: f32,
    pub badge_opacity: f32,
    pub badge_scale: f32,
    pub flare_reach: f32,
    pub flare_strength: f32,
    pub wash: f32,
    pub dx: f32,
    pub roll: f32,
}

/// A status by its place in `Urgency::ORDER` (loudest first: needs you, failed, working, bell, new output, running, done,
/// idle), which is the order the sidebar's tile states are in.
fn urgency(index: u8) -> Urgency {
    Urgency::ORDER.get(usize::from(index)).copied().unwrap_or(Urgency::Idle)
}

fn status_variant(name: &str) -> StatusMotion {
    StatusMotion::from_name(name).unwrap_or_default()
}

/// How long the change from status `from` to `to` plays under the `motion.status` choice `variant`, in milliseconds (0: a
/// cut, which is what a variant with nothing to say for that change is).
#[uniffi::export]
pub fn status_motion_duration(variant: String, from: u8, to: u8) -> f32 {
    status::duration(status_variant(&variant), urgency(from), urgency(to)).as_secs_f32() * 1000.0
}

/// The tile and row `t` (0 to 1) of the way through that change.
#[uniffi::export]
pub fn status_motion_pose(variant: String, from: u8, to: u8, t: f32) -> StatusPoseRecord {
    let pose = status::pose(status_variant(&variant), urgency(from), urgency(to), t);
    StatusPoseRecord {
        leaving: pose.leaving,
        ring: pose.ring,
        badge_opacity: pose.badge.opacity,
        badge_scale: pose.badge.scale,
        flare_reach: pose.flare.reach,
        flare_strength: pose.flare.strength,
        wash: pose.wash,
        dx: pose.dx,
        roll: pose.roll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_s_tables_come_through_by_name() {
        let from_side = insert_row_motion("from-side".into(), true);
        assert_eq!((from_side.duration_ms, from_side.room_spring), (340.0, None));
        assert_eq!(
            insert_row_pose("from-side".into(), true, 1.0),
            RowPoseRecord { opacity: 1.0, dx: 0.0, dy: 0.0, clip: 1.0 }
        );
        assert!(insert_row_pose("from-side".into(), true, 0.0).clip == 0.0);
        assert_eq!(insert_row_motion("classic".into(), false).duration_ms, 0.0, "a cut");
        assert_eq!(insert_row_motion("soft-spring".into(), true).room_spring, Some(vec![170.0, 26.0]));
        assert_eq!(
            insert_row_motion("nonsense".into(), true),
            insert_row_motion("from-side".into(), true),
            "the default"
        );
    }

    #[test]
    fn a_tucking_row_rises_and_the_bottom_one_goes_first() {
        let rows = 3;
        assert_eq!(fold_row_motion("tuck".into(), false, rows).duration_ms, 2.0 * 30.0 + 120.0 + 140.0);
        let (top, bottom) = (
            fold_row_pose("tuck".into(), false, 0.2, 0, rows),
            fold_row_pose("tuck".into(), false, 0.2, 2, rows),
        );
        assert!(bottom.opacity < top.opacity && bottom.dy < 0.0);
    }

    #[test]
    fn a_flare_and_a_nudge_are_for_a_terminal_that_asks() {
        // needs you is 0, idle 7.
        assert_eq!(status_motion_duration("flare".into(), 7, 0), 700.0);
        assert_eq!(status_motion_duration("nudge".into(), 7, 0), 420.0);
        assert_eq!(status_motion_duration("flare".into(), 0, 7), 0.0, "going quiet is a cut");
        assert_eq!(status_motion_duration("nudge".into(), 7, 7), 0.0, "no change is a cut");
        assert!(status_motion_pose("flare".into(), 7, 0, 0.3).flare_strength > 0.0);
        assert_eq!(status_motion_pose("flare".into(), 7, 0, 1.0).flare_strength, 0.0);
        let aside = (1..20).map(|i| status_motion_pose("nudge".into(), 7, 0, i as f32 / 20.0).dx).fold(0.0, f32::max);
        assert!(aside > 3.0 && aside <= 4.0, "{aside}");
    }
}
