//! A component's lines, as the Borders setting wants them.
//!
//! A `border` in a component's markup is a decorative line: with Borders off it keeps its width and draws
//! in no colour, so nothing moves when the setting changes (the same rule as `style::Edges::outlined`).
//! A rule marked always (a checkbox's outline, a selection ring) draws whatever the setting says.

use gpui_kit::{Hsla, Pixels, Styled, px, transparent_black};

/// Which sides of a box a line is on.
#[derive(Clone, Copy)]
pub(crate) struct Sides {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

#[allow(dead_code, reason = "the compiler writes any side a component's rule names")]
impl Sides {
    pub(crate) const ALL: Self = Self { top: true, right: true, bottom: true, left: true };
    pub(crate) const TOP: Self = Self { top: true, right: false, bottom: false, left: false };
    pub(crate) const RIGHT: Self = Self { top: false, right: true, bottom: false, left: false };
    pub(crate) const BOTTOM: Self = Self { top: false, right: false, bottom: true, left: false };
    pub(crate) const LEFT: Self = Self { top: false, right: false, bottom: false, left: true };
}

pub(crate) trait Rule: Styled + Sized {
    /// A line of `width` px on `sides`. `borders` is the setting; `always` keeps the line without it.
    fn rule(mut self, borders: bool, sides: Sides, width: f32, color: Hsla, always: bool) -> Self {
        let w: Pixels = px(width);
        let widths = &mut self.style().border_widths;
        if sides.top {
            widths.top = Some(w.into());
        }
        if sides.right {
            widths.right = Some(w.into());
        }
        if sides.bottom {
            widths.bottom = Some(w.into());
        }
        if sides.left {
            widths.left = Some(w.into());
        }
        self.border_color(if borders || always { color } else { transparent_black() })
    }
}

impl<T: Styled> Rule for T {}
