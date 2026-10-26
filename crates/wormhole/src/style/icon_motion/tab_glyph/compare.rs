//! Changes: git-compare's two commits fill one after the other, the second starting before the
//! first is full, and then both empty together.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::icons::part;
use crate::style::icon_motion::canvas::Canvas;
use crate::style::icon_motion::timeline::span;

const DURATION: Duration = Duration::from_millis(600);

/// How full each circle is at progress `t`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    first: f32,
    second: f32,
}

fn frame(t: f32) -> Frame {
    let empty = span(t, 0.7, 1.0);
    Frame { first: span(t, 0.0, 0.35) * (1.0 - empty), second: span(t, 0.25, 0.6) * (1.0 - empty) }
}

pub(super) fn glyph(canvas: Canvas, run: u64) -> AnyElement {
    canvas
        .frame()
        .with_animation(("tab-compare", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::GitCompare));
            }
            let now = frame(t);
            // The circles at (6, 6) and (18, 18) are the one dot, which stands at (12, 12).
            frame_box
                .child(canvas.part_at(part::DOT, (-6.0, -6.0)).opacity(now.first))
                .child(canvas.part_at(part::DOT, (6.0, 6.0)).opacity(now.second))
                .child(canvas.icon(IconName::GitCompare))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Frame, frame};

    #[test]
    fn both_circles_start_and_end_empty_and_are_full_in_between() {
        assert_eq!(frame(0.0), Frame { first: 0.0, second: 0.0 });
        assert_eq!(frame(1.0), Frame { first: 0.0, second: 0.0 });
        assert_eq!(frame(0.6), Frame { first: 1.0, second: 1.0 });
    }

    #[test]
    fn the_second_follows_the_first() {
        let early = frame(0.2);
        assert!(early.first > early.second && early.second < 0.1);
    }
}
