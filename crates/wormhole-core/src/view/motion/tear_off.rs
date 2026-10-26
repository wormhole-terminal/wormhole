//! A window growing from the chip of the terminal that was torn off, which the design fixed (the picks of
//! the motion plan: no choice of variant, so no `MotionChoices` field). The window opens at the size it will
//! have, so its terminal is sized once; what grows is the part of it that shows: a rectangle from the chip's
//! to the whole window, its corners from the chip's to the window's, and the content fading in once the
//! rectangle is big enough to hold it. GPUI has no scale, so nothing is ever drawn smaller than it is.

use std::time::Duration;

use super::ease::EASE_OUT;
use crate::view::panes::Rect;

/// How long the window takes to grow.
pub const DURATION: Duration = Duration::from_millis(280);

/// How far through the growth the content starts to fade in, as a share of it: before, the rectangle is
/// too small for the content to be more than a smear.
const CONTENT_FROM: f32 = 0.3;

/// The corners of the chip the window grows from.
pub const CHIP_RADIUS: f32 = 6.0;

/// What shows of the window at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grow {
    /// The part of the window that shows, in the window's own space.
    pub shown: Rect,
    /// Its corners' radius, from the chip's to `window_radius`.
    pub radius: f32,
    /// How much of the content shows (0.0 to 1.0).
    pub content: f32,
    /// Whether the growth is over: the window is whole, with its shadow and its lights.
    pub done: bool,
}

/// The window, whose content is `width` by `height`, `elapsed` after it began to grow from `chip` (in the
/// window's space), its own corners being `window_radius` round.
#[must_use]
pub fn pose(chip: Rect, width: f32, height: f32, window_radius: f32, elapsed: Duration) -> Grow {
    if elapsed >= DURATION {
        return Grow {
            shown: Rect::new(0.0, 0.0, width, height),
            radius: window_radius,
            content: 1.0,
            done: true,
        };
    }
    let t = elapsed.as_secs_f32() / DURATION.as_secs_f32();
    let eased = EASE_OUT.ease(t);
    let lerp = |from: f32, to: f32| from + (to - from) * eased;
    Grow {
        shown: Rect::new(lerp(chip.x, 0.0), lerp(chip.y, 0.0), lerp(chip.w, width), lerp(chip.h, height)),
        radius: lerp(CHIP_RADIUS, window_radius),
        content: EASE_OUT.ease(((t - CONTENT_FROM) / (1.0 - CONTENT_FROM)).clamp(0.0, 1.0)),
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHIP: Rect = Rect { x: 130.0, y: 80.0, w: 160.0, h: 28.0 };

    fn at(ms: u64) -> Grow {
        pose(CHIP, 900.0, 600.0, 10.0, Duration::from_millis(ms))
    }

    #[test]
    fn it_starts_as_the_chip_and_ends_as_the_window() {
        let first = at(0);
        assert_eq!(first.shown, CHIP);
        assert_eq!((first.radius, first.content, first.done), (CHIP_RADIUS, 0.0, false));
        let last = at(280);
        assert_eq!(last.shown, Rect::new(0.0, 0.0, 900.0, 600.0));
        assert_eq!((last.radius, last.content, last.done), (10.0, 1.0, true));
    }

    #[test]
    fn it_only_grows_and_never_leaves_the_window() {
        let mut before = at(0);
        for ms in (10..=280).step_by(10) {
            let now = at(ms);
            assert!(now.shown.w >= before.shown.w && now.shown.h >= before.shown.h, "at {ms}");
            assert!(now.shown.x <= before.shown.x && now.shown.y <= before.shown.y, "at {ms}");
            assert!(now.shown.right() <= 900.0 + 0.01 && now.shown.bottom() <= 600.0 + 0.01, "at {ms}");
            assert!(now.content >= before.content, "at {ms}");
            before = now;
        }
    }

    #[test]
    fn the_content_waits_for_room() {
        assert_eq!(at(60).content, 0.0, "a fifth of the way in: still the chip");
        assert!(at(140).content > 0.0 && at(140).content < 1.0);
    }
}
