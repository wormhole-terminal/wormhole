//! A colour as a tint across the left of a bar: the title row's project colour and a coloured
//! folder's band wear it the same way, so the two read as one mark.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::project::TINT_STOPS;

/// `color` as a gradient from the left edge, `peak` strong there and fading out with
/// [`TINT_STOPS`]: each stretch of it a quad of its own (GPUI's gradient has two stops). Put them
/// first in a `.relative()` bar, before its content, so the content is painted over them.
#[track_caller]
pub(crate) fn tint_layers(color: Hsla, peak: f32) -> Vec<AnyElement> {
    TINT_STOPS
        .windows(2)
        .map(|stops| {
            let ((from, strong), (to, weak)) = (stops[0], stops[1]);
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(from))
                .w(relative(to - from))
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(color.opacity(peak * strong), 0.0),
                    linear_color_stop(color.opacity(peak * weak), 1.0),
                ))
                .into_any_element()
        })
        .collect()
}
