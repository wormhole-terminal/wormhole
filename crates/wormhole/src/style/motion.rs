//! Motion: how long things take and how they ease. Short and quiet: nothing
//! for what follows the pointer (hover, selection), 120–160 ms for what changes in place,
//! 250 ms for panels; exits run at about 0.8× their entrance. GPUI's animations already
//! hold still under Reduce Motion (a loop shows its first frame, a one-shot its last);
//! a component that draws something else then (the spinner's still dots) checks
//! `cx.reduce_motion()` itself.

use std::time::Duration;

/// Hover, selection, a tab switch: no animation.
pub const MOTION_INSTANT: Duration = Duration::ZERO;
/// The ⌘ hints going away.
pub const MOTION_EXIT_FAST: Duration = Duration::from_millis(80);
/// Fades in, a drag's lift, the ⌘ hints coming in.
pub const MOTION_FAST: Duration = Duration::from_millis(120);
/// Menus, the palette and tooltips coming in (GPUI Kit's dropdown).
pub const MOTION_POPOVER: Duration = Duration::from_millis(150);
/// A band or note folding, a chevron turning, a drop settling.
pub const MOTION_BASE: Duration = Duration::from_millis(160);
/// A row sliding to its new place in the terminal list (pinned, dropped, one coming or going).
pub const MOTION_MOVE: Duration = Duration::from_millis(180);
/// Panels going away (GPUI Kit's sidebar): 0.8× their entrance.
pub const MOTION_PANEL_EXIT: Duration = Duration::from_millis(200);
/// The sidebar and the right panel coming in.
pub const MOTION_PANEL: Duration = Duration::from_millis(250);

/// One turn of the working spinner (linear, at most [`ANIMATION_FPS`] frames a second).
pub const SPIN_PERIOD: Duration = Duration::from_millis(800);
/// One turn of the light round a working terminal's status tile.
pub const TILE_ORBIT: Duration = Duration::from_millis(1600);
/// One breath of the needs-you halo; the dot itself never fades.
pub const STATUS_PULSE: Duration = Duration::from_millis(1400);
/// The halo's opacity breathes between these.
pub const PULSE_RANGE: (f32, f32) = (0.15, 1.0);
/// Looping status animations redraw no more often than this: they are small and
/// always on screen, and a full frame rate would keep the window busy for nothing.
pub const ANIMATION_FPS: f32 = 30.0;

/// Before a tooltip shows (GPUI's default, said out loud). Rows show one only when their
/// text is cut off; icon buttons always do.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);
/// Holding ⌘ alone this long shows the ⌘1–9 hints, so a quick ⌘C never flashes them.
pub const HINT_HOLD: Duration = wormhole_core::sidebar::hints::HOLD_DELAY;
/// The pointer rests on a resize handle this long before its line shows.
pub const RESIZE_HANDLE_HOVER_DELAY: Duration = Duration::from_millis(150);

/// The path's hover card opens after the pointer has rested on the folder this long, and closes
/// this long after it has left the folder and the card (so crossing the gap doesn't close it).
pub const PATH_CARD_OPEN_DELAY: Duration = Duration::from_millis(400);
pub const PATH_CARD_CLOSE_DELAY: Duration = Duration::from_millis(200);
/// How long Copy path says "Copied".
pub const PATH_CARD_COPIED_FOR: Duration = Duration::from_millis(1200);

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub f32, pub f32, pub f32, pub f32);

impl CubicBezier {
    /// The curve as an easing function for `Animation::with_easing`.
    pub fn easing(self) -> impl Fn(f32) -> f32 + 'static {
        let CubicBezier(x1, y1, x2, y2) = self;
        gpui_kit::base::animation::cubic_bezier(x1, y1, x2, y2)
    }
}

/// Decelerates: things arriving.
pub const EASE_OUT: CubicBezier = CubicBezier(0.2, 0.0, 0.0, 1.0);
/// Accelerates: things leaving.
pub const EASE_IN: CubicBezier = CubicBezier(0.3, 0.0, 1.0, 1.0);
/// Size changes in place.
pub const EASE_IN_OUT: CubicBezier = CubicBezier(0.4, 0.0, 0.2, 1.0);
/// The spinner's rotation, and nothing else.
pub const LINEAR: CubicBezier = CubicBezier(0.0, 0.0, 1.0, 1.0);

#[cfg(test)]
mod tests {
    use super::{EASE_IN, EASE_IN_OUT, EASE_OUT, LINEAR};

    #[test]
    fn curves_start_at_rest_end_in_place_and_never_turn_back() {
        for curve in [EASE_OUT, EASE_IN, EASE_IN_OUT, LINEAR] {
            let ease = curve.easing();
            assert!(ease(0.0).abs() < 1e-3, "{curve:?}");
            assert!((ease(1.0) - 1.0).abs() < 1e-3, "{curve:?}");
            let mut last = 0.0;
            for step in 1..=100 {
                let value = ease(step as f32 / 100.0);
                assert!(value >= last - 1e-4, "{curve:?} turns back at {step}");
                last = value;
            }
        }
    }

    #[test]
    fn arriving_is_quick_at_first_and_leaving_slow() {
        assert!(EASE_OUT.easing()(0.25) > 0.5);
        assert!(EASE_IN.easing()(0.25) < 0.15);
        assert!((LINEAR.easing()(0.25) - 0.25).abs() < 1e-3);
    }
}
