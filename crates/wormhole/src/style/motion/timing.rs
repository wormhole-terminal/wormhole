//! How long things take: durations, the loops' periods and the delays before something shows.
//! The curves they run along are in [`super::easing`].

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
/// Panels going away (GPUI Kit's sidebar): 0.8× their entrance. The column's motion is core's
/// (`view::motion::pose::column`), which is where its number is.
pub const MOTION_PANEL_EXIT: Duration = wormhole_core::view::motion::pose::column::CLASSIC_OUT;
/// The sidebar and the right panel coming in.
pub const MOTION_PANEL: Duration = wormhole_core::view::motion::pose::column::CLASSIC_IN;

/// How long after an animation should have finished its last frame is kept drawn: the executor's timer
/// that ends it and GPUI's own clock are two clocks, and the picture must not drop before the last frame.
pub const LAST_FRAME_GRACE: Duration = Duration::from_millis(40);

/// How far into an animation of `duration` is `t` (0.0..=1.0) of the way through it: what a pose of core,
/// which is a function of time, is asked at.
#[must_use]
pub(crate) fn elapsed(duration: Duration, t: f32) -> Duration {
    Duration::from_secs_f32(t.clamp(0.0, 1.0) * duration.as_secs_f32())
}

/// One turn of the working spinner (linear, at most [`ANIMATION_FPS`] frames a second).
pub const SPIN_PERIOD: Duration = Duration::from_millis(800);
/// One turn of the light round a working terminal's status tile.
pub const TILE_ORBIT: Duration = Duration::from_millis(1600);
/// One breath of a needs-you status tile's amber edge: slower than the status dot's halo.
pub const TILE_BREATHE: Duration = Duration::from_millis(2400);
/// The tile's amber edge breathes between this share of its strength and the whole of it.
pub const TILE_BREATHE_LEAST: f32 = 0.45;
/// One breath of the needs-you halo; the dot itself never fades.
pub const STATUS_PULSE: Duration = Duration::from_millis(1400);
/// The halo's opacity breathes between these.
pub const PULSE_RANGE: (f32, f32) = (0.15, 1.0);
/// Looping status animations redraw no more often than this: they are small and
/// always on screen, and a full frame rate would keep the window busy for nothing.
pub const ANIMATION_FPS: f32 = 30.0;
/// The tile's light travels its whole edge each turn, several points a frame at
/// [`ANIMATION_FPS`], which reads as stepping: it redraws twice as often.
pub const ORBIT_FPS: f32 = 60.0;

/// Before a tooltip shows (GPUI's default, said out loud). Rows show one only when their
/// text is cut off; icon buttons always do.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);
/// Holding ⌘ alone this long shows the ⌘1–9 hints, so a quick ⌘C never flashes them.
pub const HINT_HOLD: Duration = wormhole_core::view::sidebar::hints::HOLD_DELAY;
/// The pointer rests on a resize handle this long before its line shows.
pub const RESIZE_HANDLE_HOVER_DELAY: Duration = Duration::from_millis(150);

/// The path's hover card opens after the pointer has rested on the folder this long, and closes
/// this long after it has left the folder and the card (so crossing the gap doesn't close it).
pub const PATH_CARD_OPEN_DELAY: Duration = Duration::from_millis(400);
pub const PATH_CARD_CLOSE_DELAY: Duration = Duration::from_millis(200);
/// How long Copy path says "Copied".
pub const PATH_CARD_COPIED_FOR: Duration = Duration::from_millis(1200);
