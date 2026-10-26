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
    let size = if tile < LINE_THIN_BELOW { BADGE_MINI } else { BADGE };
    let mark = size - 2.0 * BADGE_RIM - 2.0;
    let (fill, inside): (Hsla, Option<AnyElement>) = match state {
        StatusGlyph::NeedsYou => (
            ui.agent_attention,
            Some(
                div()
                    .text_size(px(mark + 2.0))
                    .line_height(px(mark + 2.0))
                    .font_weight(FontWeight::BLACK)
                    .text_color(surface)
                    .child("!")
                    .into_any_element(),
            ),
        ),
        StatusGlyph::Done => (ui.agent_done, Some(badge_mark(glyph::CHECK, mark, surface))),
        StatusGlyph::Failed => (ui.danger, Some(badge_mark(glyph::FAILED, mark, surface))),
        StatusGlyph::Bell => (ui.agent_bell, Some(badge_mark(glyph::BELL, mark, surface))),
        StatusGlyph::Terminal { unread: true } => (ui.primary, None),
        _ => return None,
    };
    // A dot for new output: smaller than the other badges, nothing inside.
    let size = if inside.is_none() { size - BADGE_RIM * 2.0 } else { size };
    let mut badge = div()
        .absolute()
        .right(px(-BADGE_OUT))
        .bottom(px(-BADGE_OUT))
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(fill)
        .border_color(surface)
        .text_caption();
    let rim = Some(px(BADGE_RIM).into());
    let b = &mut badge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (rim, rim, rim, rim);
    Some(badge.children(inside).into_any_element())
}

fn badge_mark(path: &'static str, size: f32, color: Hsla) -> AnyElement {
    svg().path(path).size(px(size.max(1.0))).text_color(color).into_any_element()
}
