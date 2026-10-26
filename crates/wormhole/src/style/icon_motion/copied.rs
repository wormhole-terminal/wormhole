//! Copy turning into a check and back: the sheet in front fills, the copy icon gives way to a
//! check that grows in, which stays a while, and the copy icon returns. It says the copy happened
//! where the button's own drawing is, and ends on the copy icon, ready to copy again.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::canvas::Canvas;
use super::timeline::span;
use crate::icons::part;

/// The whole run, on one linear timeline whose stretches are the fractions below.
const DURATION: Duration = Duration::from_millis(1400);
/// The front sheet fills, then (with the swap) lets go of the fill.
const FILL_UNTIL: f32 = 0.12;
const SWAP_UNTIL: f32 = 0.22;
/// The check holds until here, then the copy icon comes back.
const RETURN_FROM: f32 = 0.86;
/// The check grows from this share of its size.
const CHECK_FROM: f32 = 0.7;

/// What is drawn at progress `t`: the sheet's fill and the two icons' opacity, and the check's
/// scale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    sheet: f32,
    copy: f32,
    check: f32,
    check_scale: f32,
}

fn frame(t: f32) -> Frame {
    let swap = span(t, FILL_UNTIL, SWAP_UNTIL);
    let back = span(t, RETURN_FROM, 1.0);
    Frame {
        sheet: span(t, 0.0, FILL_UNTIL) * (1.0 - swap),
        copy: 1.0 - swap + back,
        check: swap - back,
        check_scale: CHECK_FROM + (1.0 - CHECK_FROM) * swap,
    }
}

/// The Copy icon, or on its `run`th click the run.
pub(super) fn glyph(size: f32, run: u64) -> AnyElement {
    let canvas = Canvas::new(size);
    if run == 0 {
        return canvas.icon(IconName::Copy).into_any_element();
    }
    canvas
        .frame()
        .with_animation(("icon-copied", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::Copy));
            }
            let now = frame(t);
            let grown = Transformation::scale(Size { width: now.check_scale, height: now.check_scale });
            frame_box
                .child(canvas.part(part::COPY_SHEET).opacity(now.sheet))
                .child(canvas.layer(IconName::Copy).opacity(now.copy))
                .child(canvas.layer(IconName::Check).transform(grown).opacity(now.check))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Frame, frame};

    #[test]
    fn it_starts_and_ends_as_the_copy_icon_alone() {
        let rest = Frame { sheet: 0.0, copy: 1.0, check: 0.0, check_scale: 1.0 };
        assert_eq!(Frame { check_scale: 1.0, ..frame(0.0) }, rest);
        assert_eq!(frame(1.0), rest);
    }

    #[test]
    fn the_sheet_fills_then_the_check_takes_over_and_holds() {
        assert!((frame(0.12).sheet - 1.0).abs() < 1e-6);
        let swapping = frame(0.17);
        assert!(swapping.sheet > 0.0 && swapping.sheet < 1.0);
        assert!(swapping.copy > 0.0 && swapping.check > 0.0);
        let held = frame(0.5);
        assert_eq!((held.sheet, held.copy, held.check, held.check_scale), (0.0, 0.0, 1.0, 1.0));
        let back = frame(0.93);
        assert!(back.copy > 0.0 && back.check > 0.0);
    }
}
