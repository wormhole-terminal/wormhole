//! What Settings → Changes & diffs shows of the diff view: a small diff of a Rust file, and a binary
//! file's diff, both drawn by the view's own rows, colours, notes and hatch from the settings as they
//! are now (layout, wrapping, tab width and the units sizes are written in). The sample is a real
//! `FileDiff` that goes through `build_rows`, so a tab is expanded and a line coloured exactly as the
//! view does it, and a new setting that changes the rows changes the preview with it.

use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::{Settings, SizeUnits};
use wormhole_core::git::{self, DiffLine, FileDiff, Hunk, LineKind};

use super::highlight::Highlights;
use super::notes::{Note, side_note};
use super::render::{DiffColors, RowStyle, hatch, render_row, row_height};
use super::rows::{BuildKey, build_rows};
use crate::settings::{SettingsStore, UiColors};
use crate::style::{self, Edges, TextRole};

/// The sample file, before and after. Tab-indented, as Go or a Makefile is, so the tab width shows,
/// and the changed line longer than a column, so wrapping does. The change comes early: the strip is a
/// quarter of the window at most.
const OLD: &str = "fn report(order: &Order) {\n\tlet total = order.total();\n\tif total > 0 {\n\t\tlog(\"total {}\", total);\n\t}\n}\n"; // text: sample (the preview's file)
const NEW: &str = "fn report(order: &Order) {\n\tlet total = order.total();\n\tif total > 0 {\n\t\tlog(\"total of order {} is {} once the discounts and the tax of every line are in\", order.id, total);\n\t}\n}\n"; // text: sample (the preview's file)
/// Where the sample file is, for the grammar the colours come from.
const PATH: &str = "src/orders.rs"; // text: sample (the preview's file)

/// A binary file's two sides, large enough that decimal and binary units spell it differently
/// (12.4 MB and 11.8 MiB).
const SIZES: (u64, u64) = (12_400_000, 12_900_000);
/// What the binary file is called, folder before name as the diff's header writes it.
const BINARY_FOLDER: &str = "assets/"; // text: sample (the preview's file)
const BINARY_NAME: &str = "intro.mov"; // text: sample (the preview's file)
/// The rows of the view's height a binary file's body is: the note, its sizes and room round them.
const BINARY_ROWS: f32 = 4.0;
/// What a diff says over a binary side (the view's own words: `notes`).
const BINARY_NOTE: &str = "Binary file"; // text: sample (the diff view's own note)

fn line(kind: LineKind, old: Option<u32>, new: Option<u32>, text: &str) -> DiffLine {
    DiffLine { kind, old, new, text: text.into(), no_newline: false }
}

/// The sample's diff: lines 2 to 5 of the file, the fourth replaced, as the diff of a file would hand them.
fn sample_diff() -> FileDiff {
    use LineKind::{Added, Context, Removed};
    let (old, new): (Vec<&str>, Vec<&str>) = (OLD.lines().collect(), NEW.lines().collect());
    let (o, n) = (|at: usize| old[at - 1], |at: usize| new[at - 1]);
    FileDiff {
        hunks: vec![Hunk {
            header: "@@ -2,4 +2,4 @@ fn report".into(), // text: sample (the preview's hunk)
            lines: vec![
                line(Context, Some(2), Some(2), n(2)),
                line(Context, Some(3), Some(3), n(3)),
                line(Removed, Some(4), None, o(4)),
                line(Added, None, Some(4), n(4)),
                line(Context, Some(5), Some(5), n(5)),
            ],
        }],
        old_text: Some(OLD.into()),
        new_text: Some(NEW.into()),
        ..FileDiff::default()
    }
}

/// The card both previews stand in: the view's surface, in the corners and edge of a card.
fn card(ui: UiColors, colors: DiffColors) -> Div {
    div().flex().flex_col().rounded(ui.radius_lg).overflow_hidden().outlined(ui, ui.border).bg(colors.surface)
}

/// The middle line of a split diff, the full height of the body.
fn divider(ui: UiColors, colors: DiffColors) -> Div {
    div().absolute().top_0().bottom_0().left(relative(0.5)).w(ui.border_width).bg(colors.divider)
}

/// A small diff of a file with a tab-indented line and a long one, laid out by `settings`: Split or
/// Unified, wrapped or cut, the tab as wide as the setting says, coloured by the theme's syntax colours.
pub(crate) fn diff(ui: UiColors, settings: &Settings, cx: &App) -> Div {
    let store = cx.global::<SettingsStore>();
    let colors = DiffColors::new(&store.tokens);
    let (font_family, font_size) = crate::fonts::code_font(cx);
    let font_size = f32::from(font_size);
    let diff = sample_diff();
    let highlights = Highlights::for_diff(&diff, PATH, &store.highlight);
    // The preview is as wide as the column, so the sample always has room for a split view.
    let key = BuildKey::of(settings, true, 0);
    let (rows, stats) = build_rows(&diff, key, highlights.as_ref());
    let style = RowStyle {
        font_family,
        font_size,
        row_height: row_height(font_size),
        split: key.split,
        wrap: settings.diff_wrap,
        number_digits: stats.number_digits,
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
    card(ui, colors).child(body.when(key.split, |d| d.child(divider(ui, colors))))
}

/// A binary file's diff as the view draws it: the header's path, and over the hatched columns
/// (Split: one for each side's size; Unified: one, before and after) the note and the sizes in
/// `units`.
pub(crate) fn binary(ui: UiColors, split: bool, units: SizeUnits, cx: &App) -> Div {
    let store = cx.global::<SettingsStore>();
    let colors = DiffColors::new(&store.tokens);
    let (font_family, font_size) = crate::fonts::code_font(cx);
    let rows = row_height(f32::from(font_size));
    let hatched = hatch(colors, rows);
    let column = |detail: Option<String>| {
        div().flex().flex_1().items_center().justify_center().bg(hatched).child(side_note(
            ui,
            colors,
            Note { text: BINARY_NOTE, detail, oid: None },
        ))
    };
    // The view's own words under its notes: a side's size, or both of them.
    let both = git::sizes_label(Some(SIZES.0), Some(SIZES.1), units);
    let body = div().relative().flex().h(px(rows * BINARY_ROWS));
    let body = if split {
        body.child(column(Some(git::byte_size(SIZES.0, units))))
            .child(column(Some(git::byte_size(SIZES.1, units))))
            .child(divider(ui, colors))
    } else {
        body.child(column(both))
    };
    let header = div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(style::TITLE_ROW_H))
        .px_3()
        .rule_bottom(ui)
        .font_family(font_family)
        .text_caption()
        .child(div().min_w_0().truncate().text_color(ui.muted).child(BINARY_FOLDER))
        .child(div().flex_none().text_color(ui.text).child(BINARY_NAME));
    card(ui, colors).child(header).child(body)
}

#[cfg(test)]
mod tests {
    use wormhole_core::git::LineKind;

    use super::{NEW, OLD, sample_diff};

    #[test]
    fn every_line_of_the_sample_is_the_line_of_its_file() {
        // The colours are read from the files by line number: a line that differs from its file has none.
        let (old, new): (Vec<&str>, Vec<&str>) = (OLD.lines().collect(), NEW.lines().collect());
        for line in sample_diff().hunks.iter().flat_map(|hunk| &hunk.lines) {
            let (file, number) = match line.kind {
                LineKind::Removed => (&old, line.old),
                _ => (&new, line.new),
            };
            let at = number.expect("a line has its number") as usize;
            assert_eq!(file[at - 1], line.text);
        }
    }

    #[test]
    fn the_sample_has_a_tab_to_widen_and_a_line_to_wrap() {
        assert!(NEW.lines().any(|line| line.starts_with('\t')));
        assert!(NEW.lines().any(|line| line.len() > 100));
    }
}
