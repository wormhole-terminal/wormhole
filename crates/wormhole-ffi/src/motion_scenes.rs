//! What Settings → Motion's preview needs beyond the rows' own poses (`row_motion`): how much room a coming or going row
//! leaves the rows around it, when a folded group's summary shows, and a side column at a moment of its slide
//! (`wormhole_core::view::motion::pose`). Times are milliseconds since the motion began; a time that is no number counts
//! as the motion's start.

use std::time::Duration;

use wormhole_core::view::motion::pose::{Phase, column, fold, insert};
use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice};

/// The option called `name`, or the default one.
fn choice<T: MotionChoice + Default>(name: &str) -> T {
    T::from_name(name).unwrap_or_default()
}

fn phase(entering: bool) -> Phase {
    if entering { Phase::Enter } else { Phase::Exit }
}

fn elapsed(ms: f32) -> Duration {
    Duration::try_from_secs_f32(ms.max(0.0) / 1000.0).unwrap_or_default()
}

/// How much of a coming (`entering`) or going row's height is open to the rows around it (0 to 1), under the
/// `motion.insert` choice `variant`.
#[uniffi::export]
pub fn insert_room_at(variant: String, entering: bool, elapsed_ms: f32) -> f32 {
    insert::room_at(choice::<InsertMotion>(&variant), phase(entering), elapsed(elapsed_ms))
}

/// How much of the height of a group of `rows` is open to the lines below it (0 to 1) while it opens (`entering`) or
/// folds, under the `motion.fold` choice `variant`.
#[uniffi::export]
pub fn fold_room_at(variant: String, entering: bool, rows: u32, elapsed_ms: f32) -> f32 {
    fold::room_at(choice::<FoldMotion>(&variant), phase(entering), rows as usize, elapsed(elapsed_ms))
}

/// How much of a folded group's summary shows, `elapsed_ms` after the fold began.
#[uniffi::export]
pub fn fold_summary_opacity(variant: String, rows: u32, elapsed_ms: f32) -> f32 {
    fold::summary_opacity(choice::<FoldMotion>(&variant), rows as usize, elapsed(elapsed_ms))
}

/// A side column at a moment: how much of its width is in, and how much of what is in it shows.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct ColumnPoseRecord {
    pub shown: f32,
    pub content: f32,
}

/// How long a column takes to come in (`opening`) or go out under the `motion.column` choice `variant`, in milliseconds.
#[uniffi::export]
pub fn column_motion_duration(variant: String, opening: bool) -> f32 {
    column::duration(choice::<ColumnMotion>(&variant), opening).as_secs_f32() * 1000.0
}

/// The column `elapsed_ms` after it began to come in or go out, from the rest it left.
#[uniffi::export]
pub fn column_motion_pose(variant: String, opening: bool, elapsed_ms: f32) -> ColumnPoseRecord {
    let from = column::ColumnPose::rest(!opening);
    let pose = column::pose(choice::<ColumnMotion>(&variant), opening, from, elapsed(elapsed_ms));
    ColumnPoseRecord { shown: pose.shown, content: pose.content }
}
