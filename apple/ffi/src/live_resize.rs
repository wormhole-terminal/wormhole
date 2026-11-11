//! A window dragged to another size (`wormhole_core::view::motion::live_resize`): the sizes its terminal's program is told,
//! where each step of the drag lands, and the strip a window off the cell grid gives up when the drag ends. The rules are
//! the core's; AppKit says whether a drag is on and hears it end, and this only hands the questions over.

use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use wormhole_core::view::motion::live_resize::{self, Dragged, Frame, ResizeGate};

/// A size in points.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SizeRecord {
    pub width: f64,
    pub height: f64,
}

/// A window's frame in AppKit's space: `y` is the bottom edge.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct WindowFrameRecord {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the gate says about one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ResizeStepRecord {
    /// Tell the program the size the window gives it now.
    pub apply: bool,
    /// Draw this terminal again after this many milliseconds: a size is held back and this is when it is due.
    pub wake_in_ms: Option<u64>,
}

fn pair(size: SizeRecord) -> (f32, f32) {
    (size.width as f32, size.height as f32)
}

fn size((width, height): (f32, f32)) -> SizeRecord {
    SizeRecord { width: f64::from(width), height: f64::from(height) }
}

fn frame(record: WindowFrameRecord) -> Frame {
    Frame::new(record.x as f32, record.y as f32, record.width as f32, record.height as f32)
}

/// Where a window dragged to `proposed` lands so the terminal's area keeps whole cells (see `live_resize::snapped`).
#[uniffi::export]
pub fn resize_snapped(
    proposed: SizeRecord,
    from: SizeRecord,
    spare: SizeRecord,
    cell: SizeRecord,
    least: SizeRecord,
) -> SizeRecord {
    size(live_resize::snapped(pair(proposed), pair(from), pair(spare), pair(cell), pair(least)))
}

/// How far the window is to shrink, across and down, to hold whole cells with no strip left; `None` when nothing is.
#[uniffi::export]
pub fn resize_settle(spare: SizeRecord, cell: SizeRecord) -> Option<SizeRecord> {
    live_resize::settle(pair(spare), pair(cell)).map(size)
}

/// The frame a window ends the drag at once `strip` is taken off the side the person did not hold; `None` for a tiled or
/// screen-filling window, whose size is the system's.
#[uniffi::export]
pub fn resize_settled_frame(
    window: WindowFrameRecord,
    strip: SizeRecord,
    visible: WindowFrameRecord,
    held_left: bool,
    held_top: bool,
) -> Option<WindowFrameRecord> {
    let (window, visible) = (frame(window), frame(visible));
    if live_resize::tiled(window, visible) {
        return None;
    }
    let settled = live_resize::settled_frame(window, pair(strip), Dragged { left: held_left, top: held_top });
    Some(WindowFrameRecord {
        x: f64::from(settled.x),
        y: f64::from(settled.y),
        width: f64::from(settled.w),
        height: f64::from(settled.h),
    })
}

/// One terminal's gate: how often the program is told a size during a drag.
#[derive(uniffi::Object)]
pub struct LiveResizeGate {
    gate: Mutex<ResizeGate>,
}

#[uniffi::export]
impl LiveResizeGate {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self { gate: Mutex::new(ResizeGate::default()) }
    }

    /// The frame now: `live` says the window is being dragged, `differs` that the size the window gives the terminal
    /// is not the one the program has.
    pub fn step(&self, live: bool, differs: bool) -> ResizeStepRecord {
        let step =
            self.gate.lock().unwrap_or_else(PoisonError::into_inner).step(Instant::now(), live, differs);
        ResizeStepRecord {
            apply: step.apply,
            wake_in_ms: step.wake_in.map(|wake| u64::try_from(wake.as_millis()).unwrap_or(u64::MAX)),
        }
    }
}

impl Default for LiveResizeGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn at(width: f64, height: f64) -> SizeRecord {
        SizeRecord { width, height }
    }

    #[test]
    fn the_drag_steps_settles_and_gates_come_through() {
        let snapped = resize_snapped(
            at(1030.0, 690.0),
            at(1000.0, 700.0),
            at(3.0, 5.0),
            at(10.0, 20.0),
            at(560.0, 400.0),
        );
        assert_eq!(snapped, at(1027.0, 695.0));
        assert_eq!(resize_settle(at(3.0, 0.5), at(8.0, 17.0)), Some(at(3.0, 0.0)));
        assert_eq!(resize_settle(at(8.0, 17.0), at(8.0, 17.0)), None, "a whole cell is the person's size");

        let screen = WindowFrameRecord { x: 0.0, y: 0.0, width: 1600.0, height: 1000.0 };
        let window = WindowFrameRecord { x: 200.0, y: 100.0, width: 800.0, height: 600.0 };
        let settled = resize_settled_frame(window, at(3.0, 5.0), screen, false, false);
        assert_eq!(settled, Some(WindowFrameRecord { x: 200.0, y: 105.0, width: 797.0, height: 595.0 }));
        let tiled = WindowFrameRecord { x: 0.0, y: 0.0, width: 800.0, height: 990.0 };
        assert_eq!(
            resize_settled_frame(tiled, at(3.0, 5.0), screen, false, false),
            None,
            "a tiled window stays"
        );

        let gate = LiveResizeGate::new();
        assert!(gate.step(true, true).apply, "the first size of a drag is told at once");
        let held = gate.step(true, true);
        assert!(
            !held.apply && held.wake_in_ms.is_some_and(|ms| ms <= 100),
            "the next waits for the interval"
        );
        assert!(gate.step(false, true).apply, "the drag ended: told at once");
    }
}
