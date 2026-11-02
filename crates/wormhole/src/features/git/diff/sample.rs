//! The Theme Studio's sample of the diff: a small split diff drawn by the same rows as the view.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::LineKind;

use super::render::{DiffColors, RowStyle, render_row, row_height};
use super::rows::{Cell, RenderedLine, Row};
use crate::settings::UiColors;
use crate::style::Edges;
use crate::syntax::visible::visible;

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

/// What the sample in Settings → Changes & diffs follows: the person's own diff options.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    pub split: bool,
    pub wrap: bool,
    pub tab_width: usize,
}

/// A line of the sample: its kind, its numbers and its text as the file has it (tabs and all).
struct Sample {
    kind: LineKind,
    old: Option<u32>,
    new: Option<u32>,
    text: &'static str,
}

/// A hunk of a made-up file, one replaced line among context. The replacement is long enough to wrap
/// and the lines start with tabs, so wrapping and tab width each show.
const HUNK: &str = "@@ -41,5 +41,6 @@ impl Bucket"; // text: sample (a made-up hunk of the preview)
const BEFORE: [Sample; 1] = [Sample {
    kind: LineKind::Context,
    old: Some(41),
    new: Some(41),
    text: "fn take(&mut self) {", // text: sample (a made-up line of code of the preview)
}];
const REMOVED: Sample = Sample {
    kind: LineKind::Removed,
    old: Some(42),
    new: None,
    text: "\tlet budget = self.tokens;", // text: sample (a made-up line of code of the preview)
};
const ADDED: [Sample; 2] = [
    Sample {
        kind: LineKind::Added,
        old: None,
        new: Some(42),
        text: "\tlet budget = self.refill.min(self.capacity * BURST_FACTOR);", // text: sample (a made-up line of code of the preview)
    },
    Sample {
        kind: LineKind::Added,
        old: None,
        new: Some(43),
        text: "\tself.stats.checked += 1;", // text: sample (a made-up line of code of the preview)
    },
];
const AFTER: [Sample; 2] = [
    Sample {
        kind: LineKind::Context,
        old: Some(43),
        new: Some(44),
        text: "\tif budget == 0 {", // text: sample (a made-up line of code of the preview)
    },
    Sample {
        kind: LineKind::Context,
        old: Some(44),
        new: Some(45),
        text: "\t\treturn false;", // text: sample (a made-up line of code of the preview)
    },
];

/// The sample's rows as the diff view lays them out under `look`, with no card round them (the page's
/// preview card is that): split with fillers, or unified; wrapped or cut; tabs as wide as the option.
pub(crate) fn preview_rows(
    ui: UiColors,
    colors: DiffColors,
    look: Look,
    font_family: SharedString,
    font_size: f32,
) -> Div {
    let line = |sample: &Sample| {
        Arc::new(RenderedLine {
            kind: sample.kind,
            old: sample.old,
            new: sample.new,
            text: visible(sample.text, look.tab_width).text.into_owned().into(),
            highlights: Vec::new(),
            no_newline: false,
            open_line: sample.new,
        })
    };
    let mut rows = vec![Row::Hunk(HUNK.into())];
    let context = |rows: &mut Vec<Row>, lines: &[Sample]| {
        for sample in lines {
            let shared = line(sample);
            rows.push(if look.split {
                Row::Split(Cell::Line(shared.clone()), Cell::Line(shared))
            } else {
                Row::Unified(shared)
            });
        }
    };
    context(&mut rows, &BEFORE);
    if look.split {
        // The replaced line faces its replacement; the line added after it has nothing opposite.
        rows.push(Row::Split(Cell::Line(line(&REMOVED)), Cell::Line(line(&ADDED[0]))));
        rows.push(Row::Split(Cell::Filler, Cell::Line(line(&ADDED[1]))));
    } else {
        rows.push(Row::Unified(line(&REMOVED)));
        rows.extend(ADDED.iter().map(|sample| Row::Unified(line(sample))));
    }
    context(&mut rows, &AFTER);
    let style = RowStyle {
        font_family,
        font_size,
        row_height: row_height(font_size),
        split: look.split,
        wrap: look.wrap,
        number_digits: 2,
        scroll_x: 0.0,
        old_hatched: false,
        new_hatched: false,
        on_line: None,
        commented: Rc::default(),
        open: None,
    };
    let mut body = div().relative().flex().flex_col().w_full();
    for (ix, row) in rows.iter().enumerate() {
        body = body.child(render_row(ix, row, colors, &style));
    }
    body.when(look.split, |d| {
        d.child(div().absolute().top_0().bottom_0().left(relative(0.5)).w(ui.border_width).bg(colors.divider))
    })
}
