//! Usage: the gauge's needle sweeps up from the far left to its place, a little past it and back,
//! the way a needle settles.

use std::f32::consts::PI;
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::icons::part;
use crate::style::icon_motion::canvas::Canvas;
use crate::style::icon_motion::timeline::{eased, span};
use crate::style::motion::{EASE_IN_OUT, EASE_OUT};

const DURATION: Duration = Duration::from_millis(520);
/// Where the needle starts, turned back from its rest.
const START: f32 = -0.75 * PI;
/// How far past its place it swings, as a share of the way it came, and when it turns back.
const OVERSHOOT: f32 = 0.06;
const TURNS_BACK_AT: f32 = 0.7;
/// The needle's pivot (12, 14) is the centre of its piece's box when the box starts 2 units down.
const PIVOT_SHIFT: (f32, f32) = (0.0, 2.0);

/// The needle's angle from its rest, at progress `t`.
fn needle(t: f32) -> f32 {
    let swung = if t < TURNS_BACK_AT {
        (1.0 + OVERSHOOT) * eased(EASE_OUT, span(t, 0.0, TURNS_BACK_AT))
    } else {
        1.0 + OVERSHOOT * (1.0 - eased(EASE_IN_OUT, span(t, TURNS_BACK_AT, 1.0)))
    };
    START * (1.0 - swung)
}

pub(super) fn glyph(canvas: Canvas, run: u64) -> AnyElement {
    canvas
        .frame()
        .with_animation(("tab-gauge", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::Gauge));
            }
            frame_box.child(canvas.part(part::GAUGE_ARC)).child(
                canvas
                    .part_at(part::GAUGE_NEEDLE, PIVOT_SHIFT)
                    .with_transformation(Transformation::rotate(radians(needle(t)))),
            )
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::needle;

    #[test]
    fn the_needle_starts_far_back_overshoots_a_little_and_ends_in_place() {
        assert!((needle(0.0) + 0.75 * PI).abs() < 1e-5);
        assert!(needle(1.0).abs() < 1e-3);
        // Past its place (positive: turned on) before it settles.
        let past = needle(0.7);
        assert!(past > 0.0 && past < 0.1 * PI, "{past}");
    }
}
