//! A scene that loops, for the previews in Settings → Motion.

use std::time::Duration;

use gpui_kit::*;

use super::ANIMATION_FPS;

/// An animation that runs `period` and starts over, redrawn no more often than every other
/// animation that never ends ([`ANIMATION_FPS`]). Under Reduce Motion GPUI shows its first frame,
/// so a scene's time 0 is the picture before anything moves.
pub(crate) fn cycle(period: Duration) -> Animation {
    Animation::new(period).repeat().with_max_fps(ANIMATION_FPS)
}
