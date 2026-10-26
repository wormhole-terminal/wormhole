//! The badge in a tile's corner for the states its edge doesn't tell: needs input, finished,
//! failed, the bell and new output.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::ring::LINE_THIN_BELOW;
use crate::icons::glyph;
use crate::settings::UiColors;
use crate::style::scale::TextRole;
use crate::style::status::StatusGlyph;

/// The corner badge, in a tile and in one below [`LINE_THIN_BELOW`], and the rim in the surface's
/// colour that cuts it out.
const BADGE: f32 = 14.0;
const BADGE_MINI: f32 = 10.0;
const BADGE_RIM: f32 = 2.0;
/// How far the badge hangs past the tile's corner.
const BADGE_OUT: f32 = 3.0;

/// The badge in the tile's bottom-right corner for the states the edge doesn't tell: needs input
/// (!), finished (✓), failed (×), the bell, and new output (a dot).
#[track_caller]
pub(super) fn badge(ui: UiColors, state: StatusGlyph, tile: f32, surface: Hsla) -> Option<AnyElement> {
    grown(ui, state, tile, surface, 1.0)
}

/// [`badge`] at `scale` of its size (a status change grows it from nothing, past its size and back):
/// everything of it scales, its rim and its mark too, about the middle it has at rest.
#[track_caller]
pub(super) fn grown(
    ui: UiColors,
    state: StatusGlyph,
    tile: f32,
    surface: Hsla,
    scale: f32,
) -> Option<AnyElement> {
    if scale <= 0.0 {
        return None;
    }
    let size = if tile < LINE_THIN_BELOW { BADGE_MINI } else { BADGE };
    let mark = size - 2.0 * BADGE_RIM - 2.0;
    // The "!" is text, and GPUI keeps a glyph in its atlas for every size it is drawn at and never lets it
    // go: a size that changed by a fraction every frame of a pop would add a glyph a frame. So it is set in
    // whole points, a handful of sizes the atlas keeps for good. A whole badge is as it always was.
    let mark_text = ((mark + 2.0) * scale).round().max(1.0);
    let (fill, inside): (Hsla, Option<AnyElement>) = match state {
        StatusGlyph::NeedsYou => (
            ui.agent_attention,
            Some(
                div()
                    .text_size(px(mark_text))
                    .line_height(px(mark_text))
                    .font_weight(FontWeight::BLACK)
                    .text_color(surface)
                    .child("!")
                    .into_any_element(),
            ),
        ),
        StatusGlyph::Done => (ui.agent_done, Some(badge_mark(glyph::CHECK, mark * scale, surface))),
        StatusGlyph::Failed => (ui.danger, Some(badge_mark(glyph::FAILED, mark * scale, surface))),
        StatusGlyph::Bell => (ui.agent_bell, Some(badge_mark(glyph::BELL, mark * scale, surface))),
        StatusGlyph::Terminal { unread: true } => (ui.primary, None),
        _ => return None,
    };
    // A dot for new output: smaller than the other badges, nothing inside.
    let size = if inside.is_none() { size - BADGE_RIM * 2.0 } else { size };
    let (whole, size) = (size, size * scale);
    // About the middle the badge has at rest: it hangs [`BADGE_OUT`] past the corner when whole.
    let out = -BADGE_OUT + (whole - size) / 2.0;
    let mut badge = div()
        .absolute()
        .right(px(out))
        .bottom(px(out))
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(fill)
        .border_color(surface)
        .text_caption();
    let rim = Some(px(BADGE_RIM * scale).into());
    let b = &mut badge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (rim, rim, rim, rim);
    Some(badge.children(inside).into_any_element())
}

fn badge_mark(path: &'static str, size: f32, color: Hsla) -> AnyElement {
    svg().path(path).size(px(size.max(1.0))).text_color(color).into_any_element()
}
