//! Drawing the diff view: its rows (a line-number gutter, a +/− marker column
//! and the code, on tinted rows; in split mode two equal columns, with hatched
//! fillers where one side has no line) and the card around them. The rows are
//! shared with the Theme Studio's sample. In a diff against the working tree a click on a line
//! comments on it
//! (`review`), and a line with a comment wears a primary bar along its left edge.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::DiffView;
use super::notes::{notes, side_note};
use super::rows::{Cell, RenderedLine, Row};
use crate::settings::UiColors;
use crate::settings::{settings_in, store_in, ui_in};
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::token;
use crate::workspace::Workspace;
use crate::workspace::git_panel::open_file::OpenFile;
use wormhole_core::git::review::LineSide;
use wormhole_core::git::{LineKind, Side};
use wormhole_core::theme::tokens::ThemeTokens;

/// How far the card sits in from the terminal's column, left, right and below.
pub(super) const CARD_INSET: f32 = 16.0;
/// The +/− column between the line numbers and the code.
const MARKER_WIDTH: f32 = 16.0;
/// Roughly the advance of a monospace glyph, in ems.
const CHAR_EM: f32 = 0.6;
/// Space around a line number in the gutter (left, right).
const GUTTER_PAD: f32 = 12.0 + 8.0;
/// Hatch stripes per row: the period divides the row height, so the stripes run on unbroken from
/// one filler row to the next (three on a 23 pt row).
const HATCH_PER_ROW: f32 = 3.0;
/// A hatch stripe's thickness before the 45° turn (≈ 1 px on screen).
const HATCH_STRIPE: f32 = 1.5;
/// Code wider than its column (wrap off): a soft fade over the column's end where lines
/// are cut, the right while there is more to the right and the left once the view has scrolled,
/// and a thin bar under the code saying where the view is.
const FADE_W: f32 = 24.0;
const BAR_H: f32 = 6.0;
/// The bar ends this far before the column's right end, and sits this far above the bottom.
const BAR_END: f32 = 4.0;
const BAR_BOTTOM: f32 = 3.0;
/// The bar's thumb never gets shorter than this, so a very long line still shows one.
const THUMB_MIN: f32 = 24.0;
/// The room after the code in its column (`pr_3`).
const CODE_END_PAD: f32 = 12.0;
/// The bar along a commented line's left edge.
const COMMENT_BAR_W: f32 = 2.0;
/// What rows under the pointer share, so a line shows it can be commented on.
const LINE_GROUP: &str = "diff-line";

/// A line a click comments on: which side it is on, its number there and its text.
#[derive(Clone, Debug)]
pub(crate) struct LineAt {
    pub side: LineSide,
    pub line: u32,
    pub code: SharedString,
}

impl LineAt {
    /// A line as a comment pins it: an added or unchanged line by its number in the new file, a
    /// removed one by its number in the old.
    fn of(line: &RenderedLine) -> Option<Self> {
        let (side, number) = match line.kind {
            LineKind::Removed => (LineSide::Old, line.old?),
            LineKind::Added | LineKind::Context => (LineSide::New, line.new?),
        };
        Some(Self { side, line: number, code: line.text.clone() })
    }
}

/// What a click on a line does: the diff view opens the comment for it.
pub(crate) type OnLine = Rc<dyn Fn(LineAt, &mut Window, &mut App)>;

/// A diff row's height for a code font size: roomy, like an editor with
/// generous line spacing, so tinted rows read as bands rather than stripes.
pub(crate) fn row_height(font_size: f32) -> f32 {
    22.0_f32.max((font_size * 1.75).round())
}

/// The diff's colors, all tokens (the view and the Studio sample share them).
#[derive(Clone, Copy)]
pub(crate) struct DiffColors {
    /// The card behind the code: the window color, opaque, so the tints and
    /// syntax colors are the ones their contrast rules were checked against.
    pub surface: Hsla,
    pub text: Hsla,
    pub added: Hsla,
    pub added_bg: Hsla,
    pub removed: Hsla,
    pub removed_bg: Hsla,
    pub hunk_bg: Hsla,
    pub hunk_text: Hsla,
    pub line_number: Hsla,
    pub filler: Hsla,
    pub divider: Hsla,
    /// The sideways scroll bar: its track and its thumb.
    pub track: Hsla,
    pub thumb: Hsla,
    /// A commented line's bar and the comment icon under the pointer.
    pub comment: Hsla,
}

impl DiffColors {
    pub fn new(tokens: &ThemeTokens) -> Self {
        let t = |name: &str| token(tokens, name);
        Self {
            surface: t("surface.window"),
            text: t("text.primary"),
            added: t("diff.added"),
            added_bg: t("diff.added.background"),
            removed: t("diff.removed"),
            removed_bg: t("diff.removed.background"),
            hunk_bg: t("diff.hunk.background"),
            hunk_text: t("diff.hunk.text"),
            line_number: t("diff.line_number"),
            filler: t("diff.filler"),
            divider: t("border"),
            track: t("state.hover"),
            thumb: t("scrollbar.thumb"),
            comment: t("primary"),
        }
    }

    fn line_bg(&self, kind: LineKind) -> Hsla {
        match kind {
            LineKind::Added => self.added_bg,
            LineKind::Removed => self.removed_bg,
            LineKind::Context => transparent_black(),
        }
    }
}

/// How rows are laid out; fixed for one frame.
#[derive(Clone)]
pub(crate) struct RowStyle {
    pub font_family: SharedString,
    pub font_size: f32,
    pub row_height: f32,
    pub split: bool,
    pub wrap: bool,
    /// Digits in the largest line number (at least 3, so the gutter doesn't jump).
    pub number_digits: usize,
    /// How far the code is scrolled sideways (the gutters stay put).
    pub scroll_x: f32,
    /// A side whose whole column is hatched underneath the rows (a new file's
    /// old side): its fillers stay clear so the stripes aren't drawn twice.
    pub old_hatched: bool,
    pub new_hatched: bool,
    /// A click on a line comments on it; the Studio's sample has no comments.
    pub on_line: Option<OnLine>,
    /// The lines that have a comment.
    pub commented: Rc<Vec<(LineSide, u32)>>,
    /// The icon over a line that opens the file at it in the editor; the Studio's sample has none.
    pub open: Option<LineOpen>,
}

/// What the icon over a line needs: the file, and the colors its button is drawn with.
#[derive(Clone)]
pub(crate) struct LineOpen {
    pub file: OpenFile,
    pub ui: UiColors,
}

impl RowStyle {
    pub fn char_width(&self) -> f32 {
        self.font_size * CHAR_EM
    }

    /// Whole points, so the gutter's edge (and every tint after it) is crisp: 44 at 13 pt.
    pub fn gutter_width(&self) -> f32 {
        (self.number_digits.max(3) as f32 * self.char_width() + GUTTER_PAD).ceil()
    }

    /// Width taken by one column's gutter and marker, before the code starts.
    pub fn code_inset(&self) -> f32 {
        self.gutter_width() + MARKER_WIDTH
    }
}

/// The diagonal hatch of a filler: 45° stripes whose period divides the row
/// height, so the stripes run on unbroken from one filler row to the next.
pub(crate) fn hatch(colors: DiffColors, row_height: f32) -> Background {
    let period = row_height / HATCH_PER_ROW;
    pattern_slash(colors.filler, HATCH_STRIPE, period - HATCH_STRIPE)
}

pub(crate) fn render_row(ix: usize, row: &Row, colors: DiffColors, style: &RowStyle) -> Stateful<Div> {
    let base = div()
        .id(("diff-row", ix))
        .flex()
        .w_full()
        .font_family(style.font_family.clone())
        .text_size(px(style.font_size))
        // Wrapped, every line of a row is a row tall: the continuation reads as part of the
        // same band, and the gutter and marker stay on its first line.
        .when(style.wrap, |d| d.items_start().min_h(px(style.row_height)).line_height(px(style.row_height)))
        .when(!style.wrap, |d| d.items_center().h(px(style.row_height)).whitespace_nowrap());
    match row {
        Row::Hunk(header) => base
            .bg(colors.hunk_bg)
            .text_color(colors.hunk_text)
            .child(div().flex_1().min_w_0().px_3().overflow_hidden().truncate().child(header.clone()))
            // In split mode the header keeps to the old column, clear of the divider.
            .when(style.split, |d| d.child(div().flex_1())),
        Row::Unified(line) => openable(commentable(base, line, colors, style), line, style, ix, "uni")
            .bg(colors.line_bg(line.kind))
            .child(gutter(colors, style, line.old))
            .child(gutter(colors, style, line.new))
            .child(marker(colors, line.kind))
            .child(code(colors, style, line)),
        Row::Split(old, new) => base
            .child(half(ix, colors, style, old, true).self_stretch())
            .child(half(ix, colors, style, new, false).self_stretch()),
    }
}

/// `row` showing `line`: in the diff view a click comments on it, the pointer over it shows the
/// comment icon at its left edge, and with a comment it has the primary bar there.
fn commentable(
    row: Stateful<Div>,
    line: &RenderedLine,
    colors: DiffColors,
    style: &RowStyle,
) -> Stateful<Div> {
    let (Some(on_line), Some(at)) = (style.on_line.clone(), LineAt::of(line)) else { return row };
    let commented = style.commented.contains(&(at.side, at.line));
    let edge = || div().absolute().top_0().left_0();
    row.relative()
        .group(LINE_GROUP)
        .cursor_pointer()
        .when(commented, |d| d.child(edge().bottom_0().w(px(COMMENT_BAR_W)).bg(colors.comment)))
        .child(
            edge()
                .h(px(style.row_height))
                .pl_1()
                .flex()
                .items_center()
                .text_color(colors.comment)
                .invisible()
                .group_hover(LINE_GROUP, |s| s.visible())
                .child(style::icon(IconName::MessageSquarePlus).xsmall()),
        )
        .on_click(move |_, window, cx| on_line(at.clone(), window, cx))
}

/// `row` with the icon at its right end that opens the file at this line in the editor, shown
/// while the pointer is over the row (the comment icon sits at the left end). `part` tells the
/// halves of a split row apart.
fn openable(
    row: Stateful<Div>,
    line: &RenderedLine,
    style: &RowStyle,
    ix: usize,
    part: &'static str,
) -> Stateful<Div> {
    let (Some(open), Some(number)) = (&style.open, line.open_line) else { return row };
    // The ids differ by part and row, as the rows' own do.
    let ids = (
        ElementId::from((SharedString::from(format!("diff-open-{part}")), ix)),
        ElementId::from((SharedString::from(format!("diff-open-box-{part}")), ix)),
    );
    open.file.on_line(row, open.ui, LINE_GROUP, ids, number, style.row_height)
}

fn half(ix: usize, colors: DiffColors, style: &RowStyle, cell: &Cell, old_side: bool) -> Stateful<Div> {
    // `flex_1` has a zero basis: both halves are exactly half the row, whatever they hold.
    let side = div()
        .id((if old_side { "diff-old" } else { "diff-new" }, ix))
        .flex()
        .flex_1()
        .min_w_0()
        .overflow_hidden();
    let side = if style.wrap { side.items_start() } else { side.items_center() };
    match cell {
        Cell::Line(line) => {
            openable(
                commentable(side, line, colors, style),
                line,
                style,
                ix,
                if old_side { "old" } else { "new" },
            )
        }
        .bg(colors.line_bg(line.kind))
        .child(gutter(colors, style, if old_side { line.old } else { line.new }))
        .child(marker(colors, line.kind))
        .child(code(colors, style, line)),
        Cell::Filler => {
            let underneath = if old_side { style.old_hatched } else { style.new_hatched };
            side.when(!underneath, |d| d.bg(hatch(colors, style.row_height)))
        }
    }
}

fn gutter(colors: DiffColors, style: &RowStyle, number: Option<u32>) -> Div {
    div()
        .w(px(style.gutter_width()))
        .flex_none()
        .pr_2()
        .text_right()
        .text_color(colors.line_number)
        .child(number.map(|n| n.to_string()).unwrap_or_default())
}

fn marker(colors: DiffColors, kind: LineKind) -> Div {
    let (text, color) = match kind {
        LineKind::Added => ("+", colors.added),
        LineKind::Removed => ("−", colors.removed),
        LineKind::Context => ("", colors.line_number),
    };
    div().w(px(MARKER_WIDTH)).flex_none().text_color(color).child(text)
}

fn code(colors: DiffColors, style: &RowStyle, line: &RenderedLine) -> Div {
    // The runs are copied once, into the text, not into an intermediate list first.
    let text = StyledText::new(line.text.clone()).with_highlights(line.highlights.iter().cloned());
    let no_newline = line
        .no_newline
        .then(|| div().flex_none().text_caption().text_color(colors.line_number).child("no newline at end"));
    let column = div().flex_1().min_w_0().pr_3().text_color(colors.text);
    if style.wrap {
        // Block layout, so the text wraps at the column's width.
        column.child(div().w_full().child(text)).children(no_newline)
    } else {
        column.overflow_hidden().child(
            div().flex().items_center().gap_2().ml(px(-style.scroll_x)).child(text).children(no_newline),
        )
    }
}

/// The cue that a code column runs on past its right end, over the body: `left..right` is the
/// column's code in the body, `visible` how much of it shows, `scroll` and `max` how far it is
/// and can be scrolled.
fn overflow_cue(
    body: Div,
    colors: DiffColors,
    (left, right): (f32, f32),
    visible: f32,
    scroll: f32,
    max: f32,
) -> Div {
    let track = (right - left - BAR_END).max(0.0);
    let thumb = (track * visible / (visible + max)).max(THUMB_MIN).min(track);
    let thumb_at = (track - thumb) * (scroll / max).clamp(0.0, 1.0);
    let clear = Hsla { a: 0.0, ..colors.surface };
    let fade = |at: f32, from: Hsla, to: Hsla| {
        div().absolute().top_0().bottom_0().left(px(at)).w(px(FADE_W)).bg(linear_gradient(
            90.0,
            linear_color_stop(from, 0.0),
            linear_color_stop(to, 1.0),
        ))
    };
    body.when(scroll < max, |d| d.child(fade(right - FADE_W, clear, colors.surface)))
        .when(scroll > 0.0, |d| d.child(fade(left, colors.surface, clear)))
        .child(
            div()
                .absolute()
                .left(px(left))
                .w(px(track))
                .bottom(px(BAR_BOTTOM))
                .h(px(BAR_H))
                .rounded_full()
                .bg(colors.track)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(thumb_at))
                        .w(px(thumb))
                        .h_full()
                        .rounded_full()
                        .bg(colors.thumb),
                ),
        )
}

/// The Theme Studio's sample: a small split diff drawn by the same code as the view.
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

impl Workspace {
    /// The diff view, its card `top` below the column's top (under the floating title row), or
    /// inset like its sides where nothing is over it (`None`: the title row sits above the column).
    pub(crate) fn render_diff(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let view = self.diff_view.as_ref()?;
        let ui = ui_in(&self.profile, cx);
        let colors = DiffColors::new(&store_in(&self.profile, cx).tokens);
        let split_fits = self.diff_split_fits(cx);
        // The room for a split view changed (the window or a panel was resized): build the rows
        // for it now rather than at the next tick.
        let want_split = split_fits && settings_in(&self.profile, cx).diff_split;
        if view.built.is_some_and(|key| key.split != want_split) && !view.job.is_pending() {
            let this = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = this.update(cx, |this, cx| this.sync_diff(cx));
            });
        }
        let header = self.render_diff_header(view, ui, colors, split_fits, cx);
        let body = self.render_diff_body(view, ui, colors, split_fits, cx);
        let card = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .rounded(ui.radius_lg)
            .outlined(ui, ui.border)
            .bg(colors.surface)
            .child(header)
            .child(body)
            .children(self.render_review_bar(ui, cx));
        Some(
            div()
                .id("diff-view")
                .size_full()
                .flex()
                .flex_col()
                .pt(px(CARD_INSET))
                .px(px(CARD_INSET))
                .pb(px(CARD_INSET))
                .child(card)
                .into_any_element(),
        )
    }

    fn render_diff_body(
        &self,
        view: &DiffView,
        ui: UiColors,
        colors: DiffColors,
        split_fits: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let body = div().relative().flex_1().min_h_0();
        if view.loading {
            return body.child(style::empty_note(ui, "Loading diff…"));
        }
        if let Some(err) = &view.error {
            return body.child(style::empty_note(ui, err.clone()));
        }
        if let Some(log) = &view.submodule {
            return body.child(self.render_submodule_body(view, log, ui, cx));
        }
        if view.too_large() {
            return body.child(self.render_too_large(view, ui, cx));
        }
        let (font_family, font_size) = crate::fonts::code_font_in(&self.profile, cx);
        let font_size = f32::from(font_size);
        // Rows are no longer rebuilt (with a fresh list) when a setting changes, and the
        // list only measures again when its width does: do it when the font size moved.
        if view.measured_font.replace(font_size) != font_size {
            view.wrap_list.remeasure();
        }
        let split = view.split(split_fits, cx);
        let wrap = settings_in(&self.profile, cx).diff_wrap;
        let empty = view.rows.is_empty();
        let (old_side, new_side) = view.sides;
        // A column with nothing to show is hatched as a whole, with a note saying why.
        let old_hatched = split && (old_side != Side::Text || empty);
        let new_hatched = split && (new_side != Side::Text || empty);
        let mut style = RowStyle {
            font_family,
            font_size,
            row_height: row_height(font_size),
            split,
            wrap,
            number_digits: view.stats.number_digits,
            scroll_x: 0.0,
            old_hatched,
            new_hatched,
            // Only the working tree's lines are the ones the agent edits.
            on_line: view.is_working_tree().then(|| self.comment_click(view, cx)),
            commented: Rc::new(if view.is_working_tree() { self.commented_lines(view) } else { Vec::new() }),
            // The same lines, and only a file that is in the working tree now to open.
            open: view
                .is_working_tree()
                .then(|| view.open_file(cx))
                .flatten()
                .map(|file| LineOpen { file, ui }),
        };

        // Sideways scrolling: as far as the longest line is wider than its column.
        let width = view.body_width.get();
        let column = if split {
            width / 2.0 - style.code_inset()
        } else {
            width - style.code_inset() - style.gutter_width()
        };
        let text_width = view.stats.widest_chars as f32 * style.char_width();
        let content = text_width + 24.0;
        let max_scroll = if wrap { 0.0 } else { (content - column).max(0.0) };
        // The cue only when code is cut off, not for the room scrolling leaves after the end.
        let cut = !wrap && !empty && text_width > column - CODE_END_PAD;
        view.max_scroll_x.set(max_scroll);
        style.scroll_x = view.scroll_x.min(max_scroll);
        // Where each column's code is in the body, for the cue that it runs on (not over a
        // column hatched as a whole: it has no code).
        let code_spans: Vec<(f32, f32)> = if split {
            let half = width / 2.0;
            let old = (!old_hatched).then_some((style.code_inset(), half));
            let new = (!new_hatched).then_some((half + style.code_inset(), width));
            old.into_iter().chain(new).collect()
        } else {
            vec![(style.code_inset() + style.gutter_width(), width)]
        };
        let scroll_x = style.scroll_x;

        let measure = view.body_width.clone();
        let body = body
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.scroll_diff_sideways(event, window, cx)
            }))
            .child(
                canvas(move |bounds, _, _| measure.set(f32::from(bounds.size.width)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
            );

        let half = || div().absolute().top_0().bottom_0().w(relative(0.5));
        let body = body
            .when(old_hatched, |d| d.child(half().left_0().bg(hatch(colors, style.row_height))))
            .when(new_hatched, |d| d.child(half().right_0().bg(hatch(colors, style.row_height))))
            .when(!split && empty, |d| {
                d.child(div().absolute().top_0().left_0().size_full().bg(hatch(colors, style.row_height)))
            });

        let rows = view.rows.clone();
        let profile = self.profile.clone();
        let list: AnyElement = if empty {
            div().into_any_element()
        } else if wrap {
            list(view.wrap_list.clone(), move |ix, _window, cx| {
                let colors = DiffColors::new(&store_in(&profile, cx).tokens);
                render_row(ix, &rows[ix], colors, &style).into_any_element()
            })
            .size_full()
            .into_any_element()
        } else {
            let mut list = uniform_list("diff-rows", rows.len(), move |range, _window, cx| {
                let colors = DiffColors::new(&store_in(&profile, cx).tokens);
                range.map(|ix| render_row(ix, &rows[ix], colors, &style).into_any_element()).collect()
            })
            .track_scroll(&view.scroll)
            .size_full();
            // A sideways swipe scrolls the code (above), never the list up or down.
            list.style().restrict_scroll_to_axis = Some(true);
            list.into_any_element()
        };

        let notes = notes(view, split, empty, settings_in(&self.profile, cx).size_units);
        let mut body = body.child(list);
        if cut {
            for span in code_spans {
                body = overflow_cue(body, colors, span, column, scroll_x, max_scroll);
            }
        }
        body
            // The middle line runs the full height, below the last row too, so an
            // empty side still reads as a side.
            .when(split, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(0.5))
                        .w(ui.border_width)
                        .bg(colors.divider),
                )
            })
            .when(notes.iter().any(Option::is_some), |d| {
                let mut overlay = div().absolute().top_0().left_0().size_full().flex();
                for note in notes {
                    overlay = overlay.child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .children(note.map(|note| side_note(ui, colors, note))),
                    );
                }
                d.child(overlay)
            })
    }
}
