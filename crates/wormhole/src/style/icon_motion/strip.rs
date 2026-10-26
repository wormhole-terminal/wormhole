//! The sidebar's and the right panel's toggle glyph: Lucide's panel icon with its column's strip
//! filled while the column is open. As the column slides out the fill drains to the window's edge,
//! and as it slides in the fill comes back, on the slide's own timing, so the glyph always says
//! whether its column is there. Under Reduce Motion there is no slide, so the fill cuts.

use gpui_kit::*;

/// The strip inside Lucide's `panel-left` (24 units square): from the frame at 3 to the divider at
/// 9, between the frame's top and bottom at 3 and 21. The icon's strokes are drawn over the fill, so
/// it shows between them.
const STRIP_INSET: f32 = 3.0 / 24.0;
const STRIP_WIDTH: f32 = 6.0 / 24.0;
/// The frame's corner (`rx` 2), which the fill's outer corners follow.
const STRIP_CORNER: f32 = 2.0 / 24.0;

/// The side of the glyph the strip is on: its column's side of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripSide {
    Left,
    Right,
}

/// How full a toggle glyph's strip is: its column open or hidden, and the slide on its way there.
pub struct StripFill {
    side: StripSide,
    open: bool,
    slide: Option<(ElementId, Animation)>,
}

impl StripFill {
    /// A strip at rest: full while its column is `open`.
    pub fn new(side: StripSide, open: bool) -> Self {
        StripFill { side, open, slide: None }
    }

    /// The strip following its column as it slides to where `new` said, over `animation`, played
    /// once per `id`.
    pub fn sliding(mut self, id: impl Into<ElementId>, animation: Animation) -> Self {
        self.slide = Some((id.into(), animation));
        self
    }

    /// The fill under a `glyph`-point icon, in `color`, and in `hover`'s color while the pointer is
    /// over its group.
    pub(super) fn render(self, glyph: f32, color: Hsla, hover: Option<(SharedString, Hsla)>) -> AnyElement {
        let full = glyph * STRIP_WIDTH;
        let inset = px(glyph * STRIP_INSET);
        let corner = px(glyph * STRIP_CORNER);
        let strip = div().absolute().top(inset).bottom(inset).bg(color);
        let strip = match self.side {
            StripSide::Left => strip.left(inset).rounded_l(corner),
            StripSide::Right => strip.right(inset).rounded_r(corner),
        };
        let strip = match hover {
            Some((group, hover)) => strip.group_hover(group, move |s| s.bg(hover)),
            None => strip,
        };
        let open = self.open;
        match self.slide {
            Some((id, animation)) => strip
                .with_animation(id, animation, move |d, t| d.w(px(full * if open { t } else { 1.0 - t })))
                .into_any_element(),
            None => strip.w(px(if open { full } else { 0.0 })).into_any_element(),
        }
    }
}
