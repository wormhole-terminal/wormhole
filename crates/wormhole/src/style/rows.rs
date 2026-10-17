//! The lists' rows: square, edge to edge, inset 16, with
//! one lead column at x 16–32 and the text at x 40. A selection is the primary's tint,
//! hover a 5 % fill, and neither moves anything.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::bands::CARD_INSET;
use super::controls::keycap;
use super::labels::{count_badge, line_counts};
use super::metrics::Metrics;
use super::motion::{EASE_OUT, MOTION_FAST};
use super::scale::{
    FILE_NAME_MAX, INDENT, ITEM_LEAD_W, MARK_DOT, META_LINE, NAME_LINE, ROW_H, ROW_TWO_LINE_H, TITLE_GLYPH,
    TextRole,
};
use crate::settings::UiColors;
use wormhole_core::git::ChangeKind;

/// A square row that runs from edge to edge: [`ROW_H`] tall, inset 16 on both sides (the right
/// panel's rows, which sit in cards, are inset [`CARD_INSET`] and rounded instead),
/// body text with a gap of `_2` between its parts. The hover fill is `state.hover`; a
/// selected row (an open file, an expanded commit) takes the primary's tint
/// (`state.selected`) with its text in `selected_text`, and keeps it under the pointer.
/// Start it with a [`ITEM_LEAD_W`] lead (`style::lead`) so its text lands at x 40.
#[track_caller]
pub fn flush_row(id: impl Into<ElementId>, ui: UiColors, selected: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .min_h(px(ROW_H))
        .px_4()
        .text_body()
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected).text_color(ui.selected_text))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
}

/// The lead column of a row or band (x 16–32): a status glyph, a chevron, a checkbox or a
/// file icon, centered.
#[track_caller]
pub fn lead(child: impl IntoElement) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .w(px(ITEM_LEAD_W))
        .h(px(ITEM_LEAD_W))
        .child(child)
}

/// What the second line of a [`session_row`] says, which decides its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowTone {
    /// What's happening, in muted text.
    Plain,
    /// The agent needs you: amber in every state, also on the selected row.
    Attention,
    /// The agent finished: green.
    Done,
    /// A command or an agent failed: `status.danger`, as text.
    Failed,
}

/// One terminal in the sidebar: the status glyph in the lead column, the
/// name at x 40 (14 / 500, on the selected row 600 in the tint's text color), and a second
/// line under it (working, needs you, finished, the command in front, else where the
/// terminal is), which makes the row [`ROW_TWO_LINE_H`] tall. Only the compact density has
/// one-line rows, [`ROW_H`].
/// Nothing sits on the right at rest unless the row has something to say there: `hint` is
/// the ⌘N keycap while ⌘ is held (it takes the place of `trail`), and a compact row that
/// needs you says so there instead of on a second line.
pub struct SessionRow {
    /// A `style::status_glyph`, a `style::agent_logo`, or an empty lead slot.
    pub glyph: AnyElement,
    pub name: SharedString,
    /// The second line; `None` is a one-line row.
    pub detail: Option<SharedString>,
    pub tone: RowTone,
    /// The current terminal: `sidebar.row.selected`, opaque, like the settings nav.
    pub selected: bool,
    pub hint: Option<SharedString>,
    /// The name in 600 without the selection: output you haven't seen yet.
    pub emphasis: bool,
    /// The second line in this (code) font: a command, a path.
    pub detail_font: Option<SharedString>,
    /// After the name: a pin, a worktree mark.
    pub mark: Option<AnyElement>,
    /// Line 1's right side at rest (x 334): the status glyph when it sits on the right, the bell.
    pub trail: Option<AnyElement>,
    /// Line 2's right side, faint: how long a command has run, "new output".
    pub detail_trail: Option<SharedString>,
    /// A `style::status_tile` beside the lines, in place of `glyph`: the name and the second line
    /// start after it.
    pub tile: Option<AnyElement>,
    /// In a card (or lined up with one): inset by the row's own padding instead of 16, the selection
    /// and hover rounded.
    pub card: bool,
    /// What the row measures at the person's density.
    pub metrics: Metrics,
}

impl SessionRow {
    /// A row with just its glyph and name; set the rest field by field.
    pub fn new(glyph: AnyElement, name: impl Into<SharedString>) -> Self {
        Self {
            glyph,
            name: name.into(),
            detail: None,
            tone: RowTone::Plain,
            selected: false,
            hint: None,
            emphasis: false,
            detail_font: None,
            mark: None,
            trail: None,
            detail_trail: None,
            tile: None,
            card: false,
            metrics: Metrics::default(),
        }
    }
}

/// See [`SessionRow`]. Add `.on_click(…)` and the rest of its behavior.
#[track_caller]
pub fn session_row(id: impl Into<ElementId>, ui: UiColors, row: SessionRow) -> Stateful<Div> {
    let SessionRow {
        glyph,
        name,
        detail,
        tone,
        selected,
        hint,
        emphasis,
        detail_font,
        mark,
        trail,
        detail_trail,
        tile,
        card,
        metrics,
    } = row;
    let (name_color, weight) = match (selected, emphasis) {
        (true, _) => (ui.selected_text, FontWeight::SEMIBOLD),
        (false, true) => (ui.text, FontWeight::SEMIBOLD),
        (false, false) => (ui.text, FontWeight::MEDIUM),
    };
    let tone_color = match tone {
        RowTone::Plain if selected => ui.selected_muted,
        RowTone::Plain => ui.muted,
        RowTone::Attention => ui.attention_text,
        RowTone::Done => ui.done_text,
        RowTone::Failed => ui.danger_text,
    };
    // What's happening reads a step heavier than a finished result or a question.
    let detail_weight =
        if tone == RowTone::Plain && detail_font.is_none() { FontWeight::MEDIUM } else { FontWeight::NORMAL };
    let one_line_word = (detail.is_none() && tone == RowTone::Attention).then(|| {
        div()
            .flex_none()
            .text_caption()
            .font_weight(FontWeight::MEDIUM)
            .text_color(tone_color)
            .child("needs you")
    });
    let has_tile = tile.is_some();
    let two_lines = detail.is_some();
    let lines = div()
        .flex()
        .flex_col()
        .justify_center()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(NAME_LINE))
                .when(!has_tile, |d| d.child(lead(glyph)))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_body()
                        .font_weight(weight)
                        .text_color(name_color)
                        .child(name),
                )
                .children(mark.map(|mark| div().flex_none().flex().items_center().child(mark)))
                .child(div().flex_1())
                .map(|d| match hint {
                    // The ⌘ hints fade in; at once under Reduce Motion (an animation's
                    // last frame).
                    Some(hint) => d.child(keycap(ui, hint).with_animation(
                        "hint-in",
                        Animation::new(MOTION_FAST).with_easing(EASE_OUT.easing()),
                        |cap, t| cap.opacity(t),
                    )),
                    None => {
                        d.children(trail.map(|trail| div().flex_none().flex().items_center().child(trail)))
                    }
                })
                .children(one_line_word),
        )
        .children(detail.map(|detail| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(META_LINE))
                .when(!has_tile, |d| d.pl(px(ITEM_LEAD_W + SECOND_LINE_GAP)))
                .min_w_0()
                .text_caption()
                .text_color(tone_color)
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_weight(detail_weight)
                        .when_some(detail_font, |d, font| d.font_family(font))
                        .child(detail),
                )
                .child(div().flex_1())
                .children(detail_trail.map(|trail| {
                    div().flex_none().tabular_nums().text_color(ui.faint).whitespace_nowrap().child(trail)
                }))
        }));
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(if has_tile { metrics.tile_gap } else { TILE_GAP }))
        .w_full()
        .min_h(px(if has_tile {
            metrics.row_h
        } else if two_lines {
            ROW_TWO_LINE_H
        } else {
            ROW_H
        }))
        .map(|d| if card { d.px(px(metrics.row_px)).rounded(px(metrics.row_radius)) } else { d.px_4() })
        .when(has_tile && metrics.two_lines, |d| d.py_1())
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .children(tile)
        .child(lines)
}

/// The gap between a status tile and the lines beside it (at the regular density).
pub const TILE_GAP: f32 = 10.0;

/// One stretch of a card in the sidebar: a group's head and its terminals, or a terminal's own, are
/// drawn as one rounded card (`metrics.card_margin` in from the sidebar's sides, `card_radius` round,
/// its rows `card_pad` in), each line of the list a stretch of it holding `content`. `top` is the
/// first line (the card's rounded top), `bottom` the last (its rounded bottom, and `card_gap` under
/// it). The gaps are part of the stretch, so a drop there lands on its line. Without cards the lines
/// stand in by the same margin with no fill or edge, and the gap under a group is what sets it apart.
#[track_caller]
pub fn card_segment(
    ui: UiColors,
    metrics: Metrics,
    top: bool,
    bottom: bool,
    content: impl IntoElement,
) -> Div {
    let outer = div().flex().flex_col().px(px(metrics.card_margin));
    let gap = px(metrics.card_gap);
    if !metrics.cards {
        return outer.when(bottom, |d| d.pb(gap)).child(content);
    }
    let mut segment = div().flex().flex_col().px(px(metrics.card_pad));
    let edge = Some(ui.border_width.into());
    {
        let b = &mut segment.style().border_widths;
        b.left = edge;
        b.right = edge;
    }
    segment = segment.bg(ui.band).border_color(ui.border);
    {
        let b = &mut segment.style().border_widths;
        if top {
            b.top = edge;
        }
        if bottom {
            b.bottom = edge;
        }
    }
    let radius = px(metrics.card_radius);
    let pad = px(metrics.card_pad);
    let segment = segment
        .when(top, |d| d.rounded_t(radius).pt(pad))
        .when(bottom, |d| d.rounded_b(radius).pb(pad))
        .child(content);
    outer.when(bottom, |d| d.pb(gap)).child(segment)
}

/// The gap between the lead column and the name (`_2`), which a second line skips too.
const SECOND_LINE_GAP: f32 = 8.0;

/// What a [`nav_row`] says beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMark {
    None,
    /// Something on the page wants attention (an agent's hooks are incomplete): an amber dot.
    Attention,
    /// A search found this many settings on the page.
    Hits(usize),
    /// A search found nothing on the page: the row dims.
    NoHits,
    /// A feature-flagged section still being built: an outlined "Preview" tag, so it's obvious
    /// the section isn't a shipped one.
    Preview,
}

/// An item of the settings nav: [`ROW_H`] tall and as wide as the nav, its icon in
/// the lead column and its name at x 40; the current page is the opaque tint of the
/// sidebar's selected row with its icon and name (600) in the tint's text color.
#[track_caller]
pub fn nav_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: IconName,
    label: impl Into<SharedString>,
    selected: bool,
    mark: NavMark,
) -> Stateful<Div> {
    let dim = mark == NavMark::NoHits;
    let (icon_color, text_color) = match (selected, dim) {
        (true, _) => (ui.selected_text, ui.selected_text),
        (false, true) => (ui.faint, ui.faint),
        (false, false) => (ui.muted, ui.text),
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(ROW_H))
        .px_4()
        .text_body()
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(lead(Icon::new(icon).with_size(px(TITLE_GLYPH)).text_color(icon_color)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .text_color(text_color)
                .child(label.into()),
        )
        .map(|d| match mark {
            NavMark::Attention => {
                d.child(div().flex_none().size(px(MARK_DOT)).rounded_full().bg(ui.agent_attention))
            }
            NavMark::Hits(n) => d.child(count_badge(ui, super::scale::count_label(n))),
            NavMark::Preview => d.child(super::status_badge(ui, ui.info, "Preview")),
            NavMark::None | NavMark::NoHits => d,
        })
}

/// What a [`file_row`] says about a file's lines, on the right before its kind letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileLines {
    /// Added and removed lines; a new file has nothing removed, so it says only what it adds.
    Counts(usize, usize),
    /// A binary file: "bin".
    Binary,
    /// Stored with Git LFS, whose counts would be the pointer's: "LFS".
    Lfs,
}

/// One changed file in the Changes panel or under a commit: its icon in the lead
/// column, the name at x 40 (at most [`FILE_NAME_MAX`] of the row), its folder in muted caption
/// text, then on the right the line counts and the kind letter in its status color. A renamed
/// file shows where it came from before the new name, and one moved to another folder a mark.
/// Each tree level steps in by [`INDENT`].
/// The caller shortens a long name in the middle and a long folder at the front
/// (`wormhole_core::elide`); what still doesn't fit is cut at the end.
pub struct FileRow {
    /// From `crate::icons::file`, or `style::icon(IconName::File)`.
    pub icon: AnyElement,
    pub name: SharedString,
    /// Empty in a tree, where the folder rows above say it.
    pub folder: SharedString,
    /// The old name of a renamed file, when it differs from the new one.
    pub renamed_from: Option<SharedString>,
    /// The file came from another folder (a `git mv` across folders): a mark before the name
    /// says so, where "a.rs → a.rs" would say nothing.
    pub moved: bool,
    pub kind: ChangeKind,
    pub lines: FileLines,
    pub depth: usize,
    pub selected: bool,
}

/// See [`FileRow`]. Add `.on_click(…)`.
#[track_caller]
pub fn file_row(id: impl Into<ElementId>, ui: UiColors, row: FileRow) -> Stateful<Div> {
    let FileRow { icon, name, folder, renamed_from, moved, kind, lines, depth, selected } = row;
    let muted = if selected { ui.selected_muted } else { ui.muted };
    let letter_color = match kind {
        ChangeKind::Modified => ui.warning_text,
        ChangeKind::Added | ChangeKind::Untracked => ui.success_text,
        ChangeKind::Deleted | ChangeKind::Conflicted => ui.danger_text,
        // The primary as text, judged on the selected tint as well as on the panel.
        ChangeKind::Renamed => ui.selected_text,
    };
    flush_row(id, ui, selected)
        .pl(px(CARD_INSET + depth as f32 * INDENT))
        .pr(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .child(lead(icon))
        .children(renamed_from.map(|from| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .text_caption()
                .text_color(muted)
                .child(div().min_w_0().truncate().child(from))
                .child(Icon::new(IconName::ArrowRight).with_size(px(RENAME_ARROW)))
        }))
        .when(moved, |d| {
            d.child(
                div()
                    .flex_none()
                    .text_color(muted)
                    .child(Icon::new(IconName::FolderInput).with_size(px(MOVED_MARK))),
            )
        })
        .child(
            div()
                .flex_none()
                .max_w(relative(FILE_NAME_MAX))
                .truncate()
                .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(name),
        )
        .when(!folder.is_empty(), |d| {
            d.child(div().min_w_0().truncate().text_caption().text_color(muted).child(folder))
        })
        .child(div().flex_1())
        .child(match lines {
            // A new file has nothing removed, so it says only what it adds.
            FileLines::Counts(added, removed) => {
                line_counts(ui, Some(added), (removed > 0).then_some(removed)).into_any_element()
            }
            FileLines::Binary => line_word(muted, "bin"),
            FileLines::Lfs => line_word(muted, "LFS"),
        })
        .child(
            div()
                .flex_none()
                .w(px(KIND_LETTER_W))
                .flex()
                .justify_end()
                .text_caption()
                .font_weight(FontWeight::BOLD)
                .text_color(letter_color)
                .child(kind.letter()),
        )
}

/// "bin" or "LFS" where a file row's line counts go.
#[track_caller]
fn line_word(color: Hsla, word: &'static str) -> AnyElement {
    div().flex_none().text_caption().text_color(color).child(word).into_any_element()
}

/// The arrow between a renamed file's old and new name, and the mark of a moved file.
const RENAME_ARROW: f32 = 12.0;
pub const MOVED_MARK: f32 = 12.0;
/// The kind letter's column, so the line counts line up above one another.
pub const KIND_LETTER_W: f32 = 12.0;

/// A folder of the Changes tree: the chevron in the lead column, then the folder's
/// icon and its name (a chain of single folders is one row, "crates/wormhole/src") in muted
/// text. Each level steps in by [`INDENT`]. Add `.on_click(…)` to fold it.
#[track_caller]
pub fn folder_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    depth: usize,
    icon: AnyElement,
    label: impl Into<SharedString>,
    collapsed: bool,
) -> Stateful<Div> {
    flush_row(id, ui, false)
        .pl(px(CARD_INSET + depth as f32 * INDENT))
        .pr(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .text_color(ui.muted)
        .child(lead(super::controls::disclosure(ui, collapsed)))
        // The icon takes a lead column of its own, so the name lands one level in (x 64).
        .child(lead(icon))
        .child(div().min_w_0().truncate().child(label.into()))
}

/// What a [`two_line_row`] shows.
pub struct TwoLineRow {
    /// In the lead column: a feature's icon, an action's ▷, a prompt's page.
    pub lead: AnyElement,
    pub title: SharedString,
    /// The second line, in muted caption text unless `detail_color` says otherwise.
    pub detail: Option<SharedString>,
    /// "Needs trust before it takes effect" reads in the warning color.
    pub detail_color: Option<Hsla>,
    /// At the right end of the first line: a switch, a key.
    pub trailing: Option<AnyElement>,
    /// Shown at the right end of the first line while the row is hovered (`style::item_action`).
    pub actions: Vec<AnyElement>,
}

/// A row of the Project tab: [`ROW_TWO_LINE_H`] tall like the sidebar's terminals, the
/// lead in the lead column, the title at x 40 and the detail under it. Add `.on_click(…)` for a
/// row that does something when clicked (an action); the others are not clickable.
#[track_caller]
pub fn two_line_row(id: impl Into<ElementId>, ui: UiColors, row: TwoLineRow) -> Stateful<Div> {
    let TwoLineRow { lead: lead_el, title, detail, detail_color, trailing, actions } = row;
    let id: ElementId = id.into();
    let group = SharedString::from(format!("two-line-{id}"));
    div()
        .id(id.clone())
        .group(group.clone())
        .flex()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(ROW_TWO_LINE_H))
        .px(px(CARD_INSET))
        .py_1()
        .rounded(px(CARD_ROW_RADIUS))
        .hover(|d| d.bg(ui.hover))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(NAME_LINE))
                .child(lead(lead_el))
                .child(div().flex_1().min_w_0().truncate().text_body().text_color(ui.text).child(title))
                .when(!actions.is_empty(), |d| {
                    d.child(super::buttons::row_actions((id.clone(), "actions"), group, false, actions))
                })
                .children(trailing.map(|t| div().flex_none().flex().items_center().child(t))),
        )
        .children(detail.map(|detail| {
            div()
                .flex()
                .items_center()
                .h(rems(META_LINE))
                .pl(px(ITEM_LEAD_W + SECOND_LINE_GAP))
                .min_w_0()
                .text_caption()
                .text_color(detail_color.unwrap_or(ui.muted))
                .child(div().min_w_0().truncate().child(detail))
        }))
}

/// What a [`commit_row`] shows.
pub struct CommitRow {
    pub subject: SharedString,
    /// The short hash, drawn in `code_font`.
    pub sha: SharedString,
    pub author: SharedString,
    /// How long ago, as `wormhole_core::panel::age` says it.
    pub age: SharedString,
    pub code_font: SharedString,
    /// Opened to show its files: the selected tint, its subject in the tint's text color.
    pub expanded: bool,
}

/// One commit in the History tab: [`ROW_TWO_LINE_H`] tall like the sidebar's
/// terminals, the chevron in the lead column and the subject at x 40 on the first line; the
/// short hash in the primary and the author on the second, the age at the right edge. An
/// expanded commit is the selected row; its files follow it one level in. Add `.on_click(…)`.
#[track_caller]
pub fn commit_row(id: impl Into<ElementId>, ui: UiColors, row: CommitRow) -> Stateful<Div> {
    let CommitRow { subject, sha, author, age, code_font, expanded } = row;
    let (meta, hash) = if expanded { (ui.selected_muted, ui.selected_text) } else { (ui.muted, ui.primary) };
    div()
        .id(id)
        .flex()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(ROW_TWO_LINE_H))
        .px(px(CARD_INSET))
        .py_1()
        .rounded(px(CARD_ROW_RADIUS))
        .cursor_pointer()
        .when(expanded, |d| d.bg(ui.selected))
        .hover(move |d| if expanded { d } else { d.bg(ui.hover) })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(NAME_LINE))
                .child(lead(
                    Icon::new(if expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                        .xsmall()
                        .text_color(meta),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_body()
                        .when(expanded, |d| d.font_weight(FontWeight::SEMIBOLD).text_color(ui.selected_text))
                        .child(subject),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .h(rems(META_LINE))
                .pl(px(ITEM_LEAD_W + SECOND_LINE_GAP))
                .text_caption()
                .text_color(meta)
                .child(div().flex_none().text_color(hash).font_family(code_font).child(sha))
                .child(div().min_w_0().truncate().child(author))
                .child(div().flex_1())
                .child(div().flex_none().tabular_nums().child(age)),
        )
}
