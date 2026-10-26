//! The amber edge of a tile whose agent needs input, breathing between two strengths.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::ring::{RING_OUT, line};
use crate::settings::UiColors;
use crate::style::motion::{ANIMATION_FPS, TILE_BREATHE, TILE_BREATHE_LEAST};

/// The edge of a tile whose agent needs input: amber, breathing down to [`TILE_BREATHE_LEAST`] every
/// [`TILE_BREATHE`]; steady under Reduce Motion.
#[track_caller]
pub(super) fn breathing_edge(
    id: impl Into<ElementId>,
    ui: UiColors,
    size: f32,
    corner: f32,
    still: bool,
) -> AnyElement {
    let stroke = px(line(size));
    let mut edge = div()
        .absolute()
        .top(px(-RING_OUT))
        .left(px(-RING_OUT))
        .size(px(size + 2.0 * RING_OUT))
        .rounded(px(corner + RING_OUT))
        .border_color(ui.agent_attention);
    let b = &mut edge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) =
        (Some(stroke.into()), Some(stroke.into()), Some(stroke.into()), Some(stroke.into()));
    if still {
        return edge.into_any_element();
    }
    edge.with_animation(
        id,
        Animation::new(TILE_BREATHE)
            .repeat()
            .with_max_fps(ANIMATION_FPS)
            .with_easing(pulsating_between(TILE_BREATHE_LEAST, 1.0)),
        |edge, opacity| edge.opacity(opacity),
    )
    .into_any_element()
}
