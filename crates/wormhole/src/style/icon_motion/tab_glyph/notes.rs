//! Notes: the page's three lines are written in from the left, one after another.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::icons::part;
use crate::style::icon_motion::canvas::{Canvas, Cut};
use crate::style::icon_motion::timeline::{eased, span};
use crate::style::motion::EASE_OUT;

const DURATION: Duration = Duration::from_millis(480);
/// Each line starts this far into the timeline after the one before, and takes this share of it.
const STAGGER: f32 = 0.2;
const WRITING: f32 = 0.45;

/// A line of the page: its y on the grid and where it ends (the first is short).
struct Line {
    y: f32,
    to: f32,
    piece: &'static str,
}

const LINES: [Line; 3] = [
    Line { y: 9.0, to: 10.0, piece: part::NOTE_LINE_SHORT },
    Line { y: 13.0, to: 16.0, piece: part::NOTE_LINE },
    Line { y: 17.0, to: 16.0, piece: part::NOTE_LINE },
];
/// The lines start at x 8, their round ends reaching a unit past on each side. The pieces are drawn at y 12.
const START: f32 = 7.0;
const BASE_Y: f32 = 12.0;

/// How much of each line is written at progress `t`.
fn written(t: f32) -> [f32; 3] {
    std::array::from_fn(|line| {
        let from = STAGGER * line as f32;
        eased(EASE_OUT, span(t, from, from + WRITING))
    })
}

pub(super) fn glyph(canvas: Canvas, run: u64) -> AnyElement {
    canvas
        .frame()
        .with_animation(("tab-notes", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::FileText));
            }
            let lines = LINES.iter().zip(written(t)).map(|(line, written)| {
                let end = START + (line.to + 1.0 - START) * written;
                let cut = Cut { left: START, top: 0.0, right: end, bottom: 24.0 };
                canvas.cut(line.piece, (0.0, line.y - BASE_Y), cut)
            });
            frame_box.child(canvas.part(part::PAGE)).children(lines)
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::written;

    #[test]
    fn the_lines_start_unwritten_and_are_all_written_before_the_end() {
        assert_eq!(written(0.0), [0.0; 3]);
        assert_eq!(written(0.9), [1.0; 3]);
        assert_eq!(written(1.0), [1.0; 3]);
    }

    #[test]
    fn each_line_starts_after_the_one_above() {
        let early = written(0.25);
        assert!(early[0] > early[1] && early[1] > early[2]);
        assert_eq!(written(0.19)[1], 0.0);
        assert_eq!(written(0.39)[2], 0.0);
    }
}
