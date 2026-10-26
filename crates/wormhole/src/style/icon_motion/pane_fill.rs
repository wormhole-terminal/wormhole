//! The split buttons filling the pane they make: the half of the frame the new pane takes (the
//! right half of Split right, the bottom half of Split down) grows from the divider outward and
//! then lets go, the way a pane opens. The frame's strokes stay over it, so the fill is clipped to
//! the rounded frame by sitting inside it.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::canvas::{Canvas, Cut};
use super::timeline::{eased, span};
use crate::icons::part;
use crate::style::motion::{EASE_IN, EASE_OUT};

const DURATION: Duration = Duration::from_millis(480);
/// The fill has grown to its half by here (decelerating), and lets go after it (accelerating).
const GROWN_BY: f32 = 0.45;

/// Where the new pane goes: the half of the split button's frame that fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    /// Split right: the right half, growing away from the divider at x 12.
    Right,
    /// Split down: the bottom half, growing away from the divider at y 12.
    Below,
}

/// The frame's inside (grid units) and its divider.
const NEAR: f32 = 3.0;
const FAR: f32 = 21.0;
const MIDDLE: f32 = 12.0;

/// How far the fill has grown (0 to 1 of its half) and how strong it is at progress `t`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    grown: f32,
    strength: f32,
}

fn frame(t: f32) -> Frame {
    Frame {
        grown: eased(EASE_OUT, span(t, 0.0, GROWN_BY)),
        strength: 1.0 - eased(EASE_IN, span(t, GROWN_BY, 1.0)),
    }
}

/// The part of the frame's fill showing at `grown`.
fn cut(half: Half, grown: f32) -> Cut {
    let edge = MIDDLE + (FAR - MIDDLE) * grown;
    match half {
        Half::Right => Cut { left: MIDDLE, top: NEAR, right: edge, bottom: FAR },
        Half::Below => Cut { left: NEAR, top: MIDDLE, right: FAR, bottom: edge },
    }
}

/// The split icon, or on its `run`th click the fill over it.
pub(super) fn glyph(icon: IconName, size: f32, half: Half, run: u64) -> AnyElement {
    let canvas = Canvas::new(size);
    if run == 0 {
        return canvas.icon(icon).into_any_element();
    }
    canvas
        .frame()
        .with_animation(("icon-pane-fill", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(icon));
            }
            let now = frame(t);
            frame_box
                .child(canvas.cut(part::FRAME_FILL, (0.0, 0.0), cut(half, now.grown)).opacity(now.strength))
                .child(canvas.layer(icon))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Half, cut, frame};

    #[test]
    fn the_fill_starts_empty_grows_to_its_half_and_ends_gone() {
        assert_eq!(frame(0.0).grown, 0.0);
        let full = frame(0.45);
        assert!((full.grown - 1.0).abs() < 1e-3 && (full.strength - 1.0).abs() < 1e-3);
        assert_eq!(frame(1.0).strength, 0.0);
        // It is strong while it grows and fades only after.
        assert_eq!(frame(0.2).strength, 1.0);
    }

    #[test]
    fn it_grows_away_from_the_divider_and_never_past_the_frame() {
        let (empty, full) = (cut(Half::Right, 0.0), cut(Half::Right, 1.0));
        assert_eq!((empty.left, empty.right), (12.0, 12.0));
        assert_eq!((full.left, full.right, full.top, full.bottom), (12.0, 21.0, 3.0, 21.0));
        let down = cut(Half::Below, 1.0);
        assert_eq!((down.top, down.bottom, down.left, down.right), (12.0, 21.0, 3.0, 21.0));
    }
}
