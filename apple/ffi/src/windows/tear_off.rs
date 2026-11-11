//! Letting go of a terminal outside its window (core's `view::tear_off`): which window of the app lies under the
//! pointer, and where the window of a terminal torn off opens. Swift reads the windows and the screens from AppKit and
//! hands them over as values; this decides. Every point is in AppKit's global space (y up) except `tear_off_window`'s,
//! which are in the space of one display (from its top left, y down: `view::tear_off` module doc).

use wormhole_core::view::motion::tear_off as grow;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::tear_off::{Landing, Occupant, Placed, landing, place};

/// A rectangle, `x` and `y` being its corner nearest the origin of the space it is in.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct TearRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<TearRect> for Rect {
    fn from(r: TearRect) -> Self {
        Rect::new(r.x, r.y, r.w, r.h)
    }
}

/// A window of the app on the screens, front to back in the stack handed over.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct TearWindow {
    /// The id of the window when it shows terminals that can take one (the same profile as the window the drag began
    /// in, in sight); `None` for any other window of the app (Settings, the Theme Studio), which hides what lies
    /// behind it and takes nothing.
    pub window: Option<i64>,
    pub bounds: TearRect,
}

/// What letting go does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TearLanding {
    /// Nothing: over its own window, or one that takes nothing.
    Stay,
    /// Over no window of the app: the terminal opens a window of its own there.
    NewWindow,
    /// Over another window that shows terminals: the terminal goes into it.
    Into { window: i64 },
}

/// What a release at the global point (`x`, `y`) does for a terminal dragged out of window `source`. `covered`: another
/// application's window is the front-most there, which is outside Wormhole whatever lies behind it.
#[uniffi::export]
pub fn tear_off_landing(stack: Vec<TearWindow>, x: f32, y: f32, source: i64, covered: bool) -> TearLanding {
    let stack: Vec<Placed<i64>> = stack
        .into_iter()
        .map(|w| Placed {
            occupant: w.window.map_or(Occupant::Other, Occupant::Terminals),
            bounds: w.bounds.into(),
        })
        .collect();
    match landing(&stack, x, y, source, covered) {
        Landing::Stay => TearLanding::Stay,
        Landing::NewWindow => TearLanding::NewWindow,
        Landing::Into(window) => TearLanding::Into { window },
    }
}

/// Where in a torn-off window the pointer lies: over the new terminal's row, which was under it a moment ago (the
/// sidebar's width and the title row's height, about). The GPUI app's `window/tear_off.rs` `ANCHOR`.
const ANCHOR: (f32, f32) = (120.0, 90.0);

/// A torn-off window as it opens: its bounds in the display's space and the dragged chip in the window's own.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct TearPlaced {
    pub bounds: TearRect,
    pub chip: TearRect,
}

/// The window of `width` by `height` that a terminal let go of at (`x`, `y`) of a display's own space opens as, kept
/// inside that display's `visible` part (menu bar and Dock taken off) and made smaller where the display is. `chip` is
/// the dragged chip in the display's space; it comes back in the window's.
#[uniffi::export]
pub fn tear_off_place(
    x: f32,
    y: f32,
    chip: TearRect,
    width: f32,
    height: f32,
    visible: TearRect,
) -> TearPlaced {
    let placed = place((x, y), chip.into(), (width, height), ANCHOR, visible.into());
    let rect = |r: Rect| TearRect { x: r.x, y: r.y, w: r.w, h: r.h };
    TearPlaced { bounds: rect(placed.bounds), chip: rect(placed.chip) }
}

/// What shows of a window growing from its chip (core `view::motion::tear_off::pose`), in the window's own space.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct TearGrow {
    pub shown: TearRect,
    pub radius: f32,
    /// How much of the window's content shows (0 to 1).
    pub content: f32,
    /// How much of the chip's face (its tile and name) shows (1 to 0).
    pub face: f32,
    pub done: bool,
}

/// How long the window takes to grow, in milliseconds.
#[uniffi::export]
pub fn tear_off_grow_ms() -> u32 {
    u32::try_from(grow::DURATION.as_millis()).unwrap_or(u32::MAX)
}

/// The window, whose content is `width` by `height` with corners `radius` round, `elapsed_ms` after it began to grow from `chip`.
#[uniffi::export]
pub fn tear_off_grow(chip: TearRect, width: f32, height: f32, radius: f32, elapsed_ms: u32) -> TearGrow {
    let g = grow::pose(
        chip.into(),
        width,
        height,
        radius,
        std::time::Duration::from_millis(u64::from(elapsed_ms)),
    );
    let s = g.shown;
    TearGrow {
        shown: TearRect { x: s.x, y: s.y, w: s.w, h: s.h },
        radius: g.radius,
        content: g.content,
        face: g.face,
        done: g.done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> TearRect {
        TearRect { x, y, w, h }
    }

    #[test]
    fn the_front_most_window_under_the_pointer_takes_the_terminal() {
        let stack = [
            TearWindow { window: Some(2), bounds: rect(500.0, 400.0, 600.0, 500.0) },
            TearWindow { window: Some(1), bounds: rect(0.0, 300.0, 800.0, 600.0) },
        ];
        let at = |x, y, source, covered| tear_off_landing(stack.to_vec(), x, y, source, covered);
        assert_eq!(at(600.0, 700.0, 1, false), TearLanding::Into { window: 2 });
        assert_eq!(at(100.0, 700.0, 1, false), TearLanding::Stay, "its own window");
        assert_eq!(at(1500.0, 700.0, 1, false), TearLanding::NewWindow, "the desktop");
        assert_eq!(at(600.0, 700.0, 1, true), TearLanding::NewWindow, "another application on top");
        let settings = [TearWindow { window: None, bounds: rect(0.0, 0.0, 800.0, 600.0) }];
        assert_eq!(tear_off_landing(settings.to_vec(), 10.0, 10.0, 1, false), TearLanding::Stay);
    }

    #[test]
    fn the_new_window_lies_under_the_pointer_inside_the_display() {
        let chip = rect(508.0, 392.0, 160.0, 28.0);
        let tall = rect(0.0, 38.0, 1440.0, 1000.0);
        assert_eq!(
            tear_off_place(500.0, 400.0, chip, 900.0, 600.0, tall).bounds,
            rect(380.0, 310.0, 900.0, 600.0)
        );
        let visible = rect(0.0, 38.0, 1440.0, 800.0);
        // Let go at the bottom right, the window comes back up and left.
        assert_eq!(
            tear_off_place(1400.0, 830.0, chip, 900.0, 600.0, visible).bounds,
            rect(540.0, 238.0, 900.0, 600.0)
        );
        assert_eq!(
            tear_off_place(500.0, 400.0, chip, 2000.0, 2000.0, visible).bounds,
            visible,
            "never bigger than the display"
        );
    }
}
