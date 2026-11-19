//! A split opening (`wormhole_core::view::motion::split`): whether a change of layout is one pane giving half its room
//! to a new one, where the new pane's edge is `elapsed` into the spring, and how long the spring takes. The window
//! keeps the terminals at one size while it moves and tells them the layout's once, when it ends. The rules are the
//! core's; the window hands over the rectangles it already has and draws what comes back.

use std::time::Duration;

use wormhole_core::view::motion::split::{self, Opening};
use wormhole_core::view::panes::{Axis, Rect};

/// A rectangle in points from the area's top left.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SplitRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A pane and its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SplitPlace {
    pub id: i64,
    pub rect: SplitRect,
}

/// One pane giving half of its room to the new one.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SplitOpeningRecord {
    /// The pane that was split.
    pub target: i64,
    /// The new pane is to the right of it (a row); else below it.
    pub row: bool,
    /// The room the pane that was split had.
    pub whole: SplitRect,
    /// The room the new pane ends in.
    pub new_pane: SplitRect,
}

/// Where the two are `elapsed_ms` into the opening.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SplitFrameRecord {
    /// What is left of the pane that was split's room: it is clipped to this, never sized to it.
    pub kept: SplitRect,
    /// The new pane's room at its own size, with its leading edge where the spring has it.
    pub new_pane: SplitRect,
}

fn rect(record: SplitRect) -> Rect {
    Rect::new(record.x, record.y, record.w, record.h)
}

fn record(rect: Rect) -> SplitRect {
    SplitRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }
}

fn places(list: &[SplitPlace]) -> Vec<(i64, Rect)> {
    list.iter().map(|place| (place.id, rect(place.rect))).collect()
}

fn opening(record: &SplitOpeningRecord) -> Opening {
    Opening {
        target: record.target,
        axis: if record.row { Axis::Row } else { Axis::Column },
        whole: rect(record.whole),
        new_pane: rect(record.new_pane),
    }
}

/// The opening that takes the panes `before` (a lone pane's is the whole area) to those `after`, `new` being the one that
/// came; `None` when the change is anything else, which is a cut. The caller says no pane was zoomed in either.
#[uniffi::export]
pub fn split_opening(
    before: Vec<SplitPlace>,
    after: Vec<SplitPlace>,
    new: i64,
) -> Option<SplitOpeningRecord> {
    let found = Opening::of_rects(&places(&before), &places(&after), new)?;
    Some(SplitOpeningRecord {
        target: found.target,
        row: found.axis == Axis::Row,
        whole: record(found.whole),
        new_pane: record(found.new_pane),
    })
}

/// How long the opening takes, in milliseconds: until the spring is at rest.
#[uniffi::export]
pub fn split_opening_duration_ms() -> f64 {
    split::duration().as_secs_f64() * 1000.0
}

/// The opening `elapsed_ms` after it began; a time that is no number or is negative counts as the start.
#[uniffi::export]
pub fn split_opening_frame(opening_record: SplitOpeningRecord, elapsed_ms: f64) -> SplitFrameRecord {
    let seconds = if elapsed_ms.is_finite() { (elapsed_ms / 1000.0).clamp(0.0, 3600.0) } else { 0.0 };
    let travelled = split::travelled(Duration::from_secs_f64(seconds));
    let opening = opening(&opening_record);
    SplitFrameRecord { kept: record(opening.kept_at(travelled)), new_pane: record(opening.new_at(travelled)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(id: i64, x: f32, w: f32) -> SplitPlace {
        SplitPlace { id, rect: SplitRect { x, y: 0.0, w, h: 600.0 } }
    }

    #[test]
    fn a_lone_pane_split_in_two_is_an_opening_that_starts_whole_and_ends_in_the_layout() {
        let opening =
            split_opening(vec![place(1, 0.0, 1000.0)], vec![place(1, 0.0, 500.0), place(2, 500.0, 500.0)], 2)
                .expect("an opening");
        assert_eq!((opening.target, opening.row), (1, true));
        let start = split_opening_frame(opening, 0.0);
        assert_eq!(start.kept, opening.whole);
        let end = split_opening_frame(opening, split_opening_duration_ms() + 1.0);
        assert_eq!(end.kept, SplitRect { x: 0.0, y: 0.0, w: 500.0, h: 600.0 });
        assert_eq!(end.new_pane, opening.new_pane);
        assert_eq!(split_opening_frame(opening, f64::NAN).kept, opening.whole, "no number is the start");
    }

    #[test]
    fn two_panes_moving_are_not_an_opening() {
        let moved = vec![place(1, 0.0, 400.0), place(2, 400.0, 600.0)];
        assert_eq!(split_opening(vec![place(1, 0.0, 500.0), place(2, 500.0, 500.0)], moved, 2), None);
    }
}
