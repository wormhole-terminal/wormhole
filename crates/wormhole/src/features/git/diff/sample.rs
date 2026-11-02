//! The Theme Studio's sample of the diff: a small split diff drawn by the same rows as the view.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::LineKind;

use super::render::{DiffColors, RowStyle, render_row, row_height};
use super::rows::{RenderedLine, Row};
use crate::settings::UiColors;
use crate::style::Edges;

/// A small split diff drawn by the same code as the view.
pub(crate) fn sample(ui: UiColors, colors: DiffColors, font_family: SharedString, font_size: f32) -> Div {
    use super::rows::Cell as C;
    use LineKind::{Added, Context, Removed};
    let line = |kind, old, new, text| C::Line(Arc::new(RenderedLine::plain(kind, old, new, text)));
    let rows = [
        Row::Hunk("@@ -41,4 +41,6 @@ impl OrdersApi".into()),
        Row::Split(
            line(Context, Some(41), Some(41), "let id = req.id;"),
            line(Context, Some(41), Some(41), "let id = req.id;"),
        ),
        Row::Split(
            line(Removed, Some(42), None, "let total = 0;"),
            line(Added, None, Some(42), "let mut total = 0;"),
        ),
        Row::Split(C::Filler, line(Added, None, Some(43), "total += tax(id);")),
        Row::Split(C::Filler, line(Added, None, Some(44), "log(total);")),
        Row::Split(
            line(Context, Some(43), Some(45), "Ok(total)"),
            line(Context, Some(43), Some(45), "Ok(total)"),
        ),
    ];
    let style = RowStyle {
        font_family,
        font_size,
        row_height: row_height(font_size),
        split: true,
        wrap: false,
        number_digits: 2,
        scroll_x: 0.0,
        old_hatched: false,
        new_hatched: false,
        on_line: None,
        commented: Rc::default(),
        open: None,
    };
    let mut body = div().relative().flex().flex_col();
    for (ix, row) in rows.iter().enumerate() {
        body = body.child(render_row(ix, row, colors, &style));
    }
    div()
        .flex()
        .flex_col()
        .w(px(560.0))
        .rounded(ui.radius_lg)
        .overflow_hidden()
        .outlined(ui, ui.border)
        .bg(colors.surface)
        .child(body.child(
            div().absolute().top_0().bottom_0().left(relative(0.5)).w(ui.border_width).bg(colors.divider),
        ))
}
