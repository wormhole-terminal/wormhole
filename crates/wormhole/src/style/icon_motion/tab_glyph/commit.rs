//! History: the commit's dot fills, holds and empties while its two lines draw out from it, as a
//! commit joins the line of history.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::icons::part;
use crate::style::icon_motion::canvas::{Canvas, Cut};
use crate::style::icon_motion::timeline::{eased, span};
use crate::style::motion::EASE_OUT;

const DURATION: Duration = Duration::from_millis(480);

/// How full the dot is and how much of the lines is drawn at progress `t`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    dot: f32,
    lines: f32,
}

fn frame(t: f32) -> Frame {
    Frame { dot: span(t, 0.0, 0.3) * (1.0 - span(t, 0.65, 1.0)), lines: eased(EASE_OUT, span(t, 0.0, 0.6)) }
}

/// How far a line reaches from the dot, with its round end: the lines run x 3–9 and 15–21 (the ring
/// is x 8–16).
const REACH: f32 = 8.0;
/// Where the lines start, inside the ring's stroke, so a line not yet drawn is hidden by it.
const LEFT_FROM: f32 = 10.0;
const RIGHT_FROM: f32 = 14.0;

pub(super) fn glyph(canvas: Canvas, run: u64) -> AnyElement {
    canvas
        .frame()
        .with_animation(("tab-commit", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::GitCommitHorizontal));
            }
            let now = frame(t);
            let reach = REACH * now.lines;
            let band = |left, right| Cut { left, top: 0.0, right, bottom: 24.0 };
            frame_box
                .child(canvas.part(part::DOT).opacity(now.dot))
                .child(canvas.part(part::RING))
                .child(canvas.cut(part::COMMIT_LEFT, (0.0, 0.0), band(LEFT_FROM - reach, LEFT_FROM)))
                .child(canvas.cut(part::COMMIT_RIGHT, (0.0, 0.0), band(RIGHT_FROM, RIGHT_FROM + reach)))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Frame, frame};

    #[test]
    fn the_dot_starts_and_ends_empty_and_the_lines_start_in_and_end_out() {
        assert_eq!(frame(0.0), Frame { dot: 0.0, lines: 0.0 });
        assert_eq!(frame(1.0).dot, 0.0);
        assert!((frame(1.0).lines - 1.0).abs() < 1e-3);
    }

    #[test]
    fn the_dot_is_full_while_it_holds() {
        assert_eq!(frame(0.5).dot, 1.0);
        assert!(frame(0.8).dot < 1.0);
    }
}
