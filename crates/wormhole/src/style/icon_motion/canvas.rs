//! The 24-unit box Lucide draws its icons in, at the size one is shown. A piece of an icon
//! (`icons::part`) is drawn on the same grid, so putting one over the icon takes the numbers its
//! path data already has: the point (12, 14) of a gauge's needle is `at(12.0)`, `at(14.0)`.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;

/// The grid Lucide's icons are drawn on.
const UNITS: f32 = 24.0;

/// An icon's box, `size` points on a side.
#[derive(Clone, Copy, Debug)]
pub(super) struct Canvas {
    size: f32,
}

/// A rectangle of the grid, in its units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Cut {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Canvas {
    pub(super) fn new(size: f32) -> Self {
        Canvas { size }
    }

    /// `units` of the grid, in points.
    fn at(self, units: f32) -> Pixels {
        px(self.size * units / UNITS)
    }

    /// The box the parts are laid in: the icon's size, and the place its absolute parts are
    /// measured from. It stands where the icon stood.
    pub(super) fn frame(self) -> Div {
        div().relative().flex_none().size(px(self.size))
    }

    /// The whole icon, as it is drawn at rest.
    pub(super) fn icon(self, icon: IconName) -> Icon {
        Icon::new(icon).with_size(px(self.size))
    }

    /// The whole icon laid over the box's other parts, which it doesn't push aside.
    pub(super) fn layer(self, icon: IconName) -> Icon {
        self.icon(icon).absolute().top_0().left_0()
    }

    /// A piece of an icon in its place in the box, in the icon's color. It is an `Icon` rather than a
    /// bare `svg()`, which GPUI paints only in a color of its own: the `Icon` takes the text color
    /// around it as it lays out, as the whole icon does, hover included.
    pub(super) fn part(self, path: &'static str) -> Icon {
        self.part_at(path, (0.0, 0.0))
    }

    /// A piece moved by `shift` grid units: one shape serves for the parts that are alike but
    /// stand apart (the two circles of git-compare are one dot, 12 units apart).
    pub(super) fn part_at(self, path: &'static str, (x, y): (f32, f32)) -> Icon {
        Icon::empty().path(path).with_size(px(self.size)).absolute().left(self.at(x)).top(self.at(y))
    }

    /// The part of a piece inside `cut`, with the rest hidden: a line drawn from one end, or a
    /// fill growing from an edge, by moving the cut.
    pub(super) fn cut(self, path: &'static str, shift: (f32, f32), cut: Cut) -> Div {
        // The piece keeps its place on the grid while its window moves, so the window holds it
        // back by its own offset.
        let inside = self.part_at(path, (shift.0 - cut.left, shift.1 - cut.top));
        div()
            .absolute()
            .left(self.at(cut.left))
            .top(self.at(cut.top))
            .w(self.at((cut.right - cut.left).max(0.0)))
            .h(self.at((cut.bottom - cut.top).max(0.0)))
            .overflow_hidden()
            .child(inside)
    }
}
