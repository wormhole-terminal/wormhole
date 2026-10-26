//! Project: the folder fills with the icon's color at once and the fill drains away.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::icons::part;
use crate::style::icon_motion::canvas::Canvas;
use crate::style::icon_motion::timeline::span;

const DURATION: Duration = Duration::from_millis(420);
/// The fill is full by here, and drains after.
const FULL_AT: f32 = 0.3;

/// How full the folder is at progress `t`.
fn fill(t: f32) -> f32 {
    span(t, 0.0, FULL_AT) * (1.0 - span(t, FULL_AT, 1.0))
}

pub(super) fn glyph(canvas: Canvas, run: u64) -> AnyElement {
    canvas
        .frame()
        .with_animation(("tab-folder", run), Animation::new(DURATION), move |frame_box, t| {
            if t >= 1.0 {
                return frame_box.child(canvas.icon(IconName::Folder));
            }
            frame_box
                .child(canvas.part(part::FOLDER_FILL).opacity(fill(t)))
                .child(canvas.icon(IconName::Folder))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn the_folder_starts_and_ends_empty_and_is_full_at_its_peak() {
        assert_eq!(fill(0.0), 0.0);
        assert_eq!(fill(1.0), 0.0);
        assert_eq!(fill(0.3), 1.0);
        assert!(fill(0.15) > 0.0 && fill(0.65) > 0.0 && fill(0.65) < 1.0);
    }
}
