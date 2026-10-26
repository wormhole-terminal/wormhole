//! An icon swapped for another (the pane zoom button's corners turning from out to in): the old
//! one shrinks away and the new one grows into its place. It is driven by the change of the icon,
//! not by a click, so a zoom made by the keys shows it too.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::canvas::Canvas;
use super::timeline::{eased, span};
use crate::style::motion::EASE_OUT;

const DURATION: Duration = Duration::from_millis(240);
/// How small the icons get at the turn.
const SMALLEST: f32 = 0.6;

/// Which icon is drawn at progress `t`, and how big.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    new: bool,
    scale: f32,
}

fn frame(t: f32) -> Frame {
    if t < 0.5 {
        Frame { new: false, scale: 1.0 - (1.0 - SMALLEST) * eased(EASE_OUT, span(t, 0.0, 0.5)) }
    } else {
        Frame { new: true, scale: SMALLEST + (1.0 - SMALLEST) * eased(EASE_OUT, span(t, 0.5, 1.0)) }
    }
}

/// `icon`, or on its `run`th change the swap from `before` to it.
pub(super) fn glyph(icon: IconName, before: IconName, size: f32, run: u64) -> AnyElement {
    let canvas = Canvas::new(size);
    if run == 0 {
        return canvas.icon(icon).into_any_element();
    }
    canvas
        .icon(icon)
        .with_animation(("icon-swap", run), Animation::new(DURATION), move |_, t| {
            let now = frame(t);
            let shown = canvas.icon(if now.new { icon } else { before });
            if t >= 1.0 {
                shown
            } else {
                shown.transform(Transformation::scale(Size { width: now.scale, height: now.scale }))
            }
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::frame;

    #[test]
    fn the_old_icon_shrinks_then_the_new_one_grows_to_full_size() {
        let (first, middle, last) = (frame(0.0), frame(0.4999), frame(1.0));
        assert!(!first.new && first.scale == 1.0);
        assert!(!middle.new && (middle.scale - 0.6).abs() < 0.01);
        assert!(frame(0.5).new && (frame(0.5).scale - 0.6).abs() < 1e-6);
        assert!(last.new && (last.scale - 1.0).abs() < 1e-3);
    }
}
