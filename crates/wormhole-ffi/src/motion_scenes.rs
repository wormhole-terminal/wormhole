//! What Settings → Motion's preview needs beyond the rows' own poses (`row_motion`): how much room a coming or going row
//! leaves the rows around it, when a folded group's summary shows, and a side column at a moment of its slide
//! (`wormhole_core::view::motion::pose`). Times are milliseconds since the motion began; a time that is no number counts
//! as the motion's start.

use std::time::Duration;

use wormhole_core::view::motion::pose::{Phase, column, fold, insert};
use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, drag};
use wormhole_core::view::sidebar::motion::Drawn;
use wormhole_core::view::sidebar::room::make_room;

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

/// How long a split's bracket takes to draw down its rows once a drop has made the split, in milliseconds.
#[uniffi::export]
pub fn bracket_draw_ms() -> f32 {
    drag::BRACKET.as_secs_f32() * 1000.0
}

/// How much of a split's bracket shows `t` (0 to 1) of the way through its drawing, for the stretch drawn from `from` to
/// `to` of that time (a first row's is 0 to 0.5, a middle one's 0.25 to 0.75, a last one's 0.5 to 1).
#[uniffi::export]
pub fn bracket_share(t: f32, from: f32, to: f32) -> f32 {
    drag::bracket(t, from, to)
}

/// A line of the list as laid out with nothing pushed: where its top is (from the content's start) and how tall it is.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct RoomLine {
    pub top: f32,
    pub height: f32,
}

/// A line that steps aside to make room for a dragged row: which line (its index) and how far, down being positive.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct RoomPush {
    pub line: u32,
    pub by: f32,
}

/// How far each line steps aside to open a gap where the dragged lines `run_start..run_end` would land, before line
/// `slot` (`lines.len()` for the end): the lines it passes, by the run's own height (`view::sidebar::room::make_room`).
#[uniffi::export]
pub fn room_pushes(lines: Vec<RoomLine>, run_start: u32, run_end: u32, slot: u32) -> Vec<RoomPush> {
    let drawn: Vec<Drawn<u32>> = lines
        .iter()
        .enumerate()
        .map(|(at, line)| Drawn { key: at as u32, top: line.top, height: line.height })
        .collect();
    make_room(&drawn, run_start as usize..run_end as usize, slot as usize)
        .into_iter()
        .map(|(line, by)| RoomPush { line, by })
        .collect()
}
