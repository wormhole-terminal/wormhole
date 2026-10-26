//! A terminal's row in the sidebar and in the tab bar, and the way a long name fades out.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::controls::keycap;
use crate::style::metrics::Metrics;
use crate::style::motion::{EASE_OUT, MOTION_FAST};
use crate::style::scale::{ITEM_LEAD_W, META_LINE, NAME_LINE, ROW_H, ROW_TWO_LINE_H, TextRole};

use super::detail::{DetailRoll, Words, rolling};
use super::{SECOND_LINE_GAP, TILE_GAP, lead};

/// What the second line of a [`session_row`] says, which decides its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowTone {
    /// What's happening, in muted text.
    Plain,
    /// The agent needs input: amber in every state, also on the selected row.
    Attention,
    /// The agent finished: green.
    Done,
    /// A command or an agent failed: `status.danger`, as text.
    Failed,
}

/// One terminal in the sidebar: the status glyph in the lead column, the
/// name at x 40 (14 / 500, on the selected row 600 in the tint's text color), and a second
/// line under it (working, needs input, finished, the command in front, else where the
/// terminal is), which makes the row [`ROW_TWO_LINE_H`] tall. Only the compact density has
/// one-line rows, [`ROW_H`].
/// Nothing sits on the right at rest unless the row has something to say there: `hint` is
/// the ⌘N keycap while ⌘ is held (it takes the place of `trail`), and a compact row that
/// needs input says so there instead of on a second line.
pub struct SessionRow {
    /// A `style::status_glyph`, a `style::agent_logo`, or an empty lead slot.
    pub glyph: AnyElement,
    pub name: SharedString,
    /// The name's place while it is being renamed: its field.
    pub name_edit: Option<AnyElement>,
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
    /// Drawn first, under the tile and the lines, placed by the caller (absolutely, in the row):
    /// a split's line, which the tiles stand on.
    pub behind: Option<AnyElement>,
    /// In a card (or lined up with one): inset by the row's own padding instead of 16, the selection
    /// and hover rounded.
    pub card: bool,
    /// The only row of its card (`card_segment_solo`): the card itself takes the selection and hover
    /// colours, so the row draws neither.
    pub solo: bool,
    /// What the row measures at the person's density.
    pub metrics: Metrics,
    /// A name too long for its line: cut to the line's room and faded out over its end instead of
    /// an ellipsis, so it uses the whole width (the caller measures it).
    pub name_fade: Option<NameFade>,
    /// The name under the pointer of the row's group, where a × then shows at the line's end: cut
    /// shorter, so it has faded out before the ×. Only paints, so nothing moves.
    pub name_hover: Option<(NameFade, SharedString)>,
    /// The second line is changing from other words (a status change that rolls it).
    pub detail_roll: Option<DetailRoll>,
}

/// The room a long name leaves at its line's end for the × that shows there under the pointer:
/// the ×'s glyph and a gap.
pub const CLOSE_GLYPH_ROOM: f32 = crate::style::scale::GLYPH + 4.0;

/// A name as a row or a tab draws it: whole when it fits; cut at `rest` and faded out there when it
/// doesn't; and under the pointer of `hover`'s group cut at its room instead (shorter, clear of the
/// × that shows then), drawn in the same place. Set the text's size, weight and colour on it.
#[track_caller]
pub(in crate::style) fn name_text(
    name: SharedString,
    rest: Option<&NameFade>,
    hover: Option<&(NameFade, SharedString)>,
) -> Div {
    let cut = |fade: &NameFade| {
        div().max_w(px(fade.room)).overflow_hidden().whitespace_nowrap().child(faded_name(name.clone(), fade))
    };
    let at_rest = match rest {
        Some(fade) => cut(fade),
        None => div().truncate().child(name.clone()),
    };
    let base = div().relative().min_w_0();
    match hover {
        None => base.child(at_rest),
        Some((fade, group)) => base.child(at_rest.group_hover(group.clone(), |s| s.invisible())).child(
            cut(fade).absolute().top_0().left_0().invisible().group_hover(group.clone(), |s| s.visible()),
        ),
    }
}

/// How a name too long for its line ends: the bytes that fade from its colour to nothing, and the
/// width it is cut to, where the fade has reached nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct NameFade {
    pub bytes: std::ops::Range<usize>,
    pub room: f32,
}

/// `name`, its `fade` bytes going from its colour to nothing a character at a time and what follows
/// them invisible.
fn faded_name(name: SharedString, fade: &NameFade) -> StyledText {
    // `fade_out` lowers the text colour's alpha; a translucent `color` would only be blended over it.
    let faded = |amount: f32| HighlightStyle { fade_out: Some(amount), ..Default::default() };
    let chars: Vec<(usize, char)> = name[fade.bytes.clone()].char_indices().collect();
    let steps = chars.len() as f32 + 1.0;
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = chars
        .iter()
        .enumerate()
        .map(|(n, &(at, c))| {
            let start = fade.bytes.start + at;
            (start..start + c.len_utf8(), faded((n as f32 + 1.0) / steps))
        })
        .collect();
    if fade.bytes.end < name.len() {
        highlights.push((fade.bytes.end..name.len(), faded(1.0)));
    }
    StyledText::new(name).with_highlights(highlights)
}

impl SessionRow {
    /// A row with just its glyph and name; set the rest field by field.
    pub fn new(glyph: AnyElement, name: impl Into<SharedString>) -> Self {
        Self {
            glyph,
            name: name.into(),
            name_edit: None,
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
            behind: None,
            card: false,
            solo: false,
            metrics: Metrics::default(),
            name_fade: None,
            name_hover: None,
            detail_roll: None,
        }
    }
}

/// The colour a second line of `tone` is set in.
fn tone_color_of(ui: UiColors, tone: RowTone, selected: bool) -> Hsla {
    match tone {
        RowTone::Plain if selected => ui.selected_muted,
        RowTone::Plain => ui.muted,
        RowTone::Attention => ui.attention_text,
        RowTone::Done => ui.done_text,
        RowTone::Failed => ui.danger_text,
    }
}

/// What's happening reads a step heavier than a finished result or a question.
fn detail_weight_of(tone: RowTone, code: bool) -> FontWeight {
    if tone == RowTone::Plain && !code { FontWeight::MEDIUM } else { FontWeight::NORMAL }
}

/// See [`SessionRow`]. Add `.on_click(…)` and the rest of its behavior.
#[track_caller]
pub fn session_row(id: impl Into<ElementId>, ui: UiColors, row: SessionRow) -> Stateful<Div> {
    let id: ElementId = id.into();
    let SessionRow {
        glyph,
        name,
        name_edit,
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
        behind,
        card,
        solo,
        metrics,
        name_fade,
        name_hover,
        detail_roll,
    } = row;
    let (name_color, weight) = match (selected, emphasis) {
        (true, _) => (ui.selected_text, FontWeight::SEMIBOLD),
        (false, true) => (ui.text, FontWeight::SEMIBOLD),
        (false, false) => (ui.text, FontWeight::MEDIUM),
    };
    let tone_color = tone_color_of(ui, tone, selected);
    let detail_weight = detail_weight_of(tone, detail_font.is_some());
    let one_line_word = (detail.is_none() && tone == RowTone::Attention).then(|| {
        div()
            .flex_none()
            .text_caption()
            .font_weight(FontWeight::MEDIUM)
            .text_color(tone_color)
            .child("needs input")
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
                .map(|d| match name_edit {
                    Some(edit) => d.child(div().flex_1().min_w_0().child(edit)),
                    None => d.child(
                        // Cut at its room, the end faded out rather than an ellipsis.
                        name_text(name.clone(), name_fade.as_ref(), name_hover.as_ref())
                            .text_body()
                            .font_weight(weight)
                            .text_color(name_color),
                    ),
                })
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
                .child(match detail_roll {
                    Some(DetailRoll { from, from_tone, from_font, shift, to }) => {
                        let old = Words {
                            text: from,
                            color: tone_color_of(ui, from_tone, selected),
                            weight: detail_weight_of(from_tone, from_font.is_some()),
                            font: from_font,
                        };
                        let now = Words {
                            text: detail,
                            color: tone_color,
                            font: detail_font,
                            weight: detail_weight,
                        };
                        rolling(ElementId::from((id.clone(), "detail")), shift, to, old, now)
                    }
                    None => div()
                        .min_w_0()
                        .truncate()
                        .font_weight(detail_weight)
                        .when_some(detail_font, |d, font| d.font_family(font))
                        .child(detail)
                        .into_any_element(),
                })
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
        .map(|d| {
            if card {
                d.px(px(metrics.row_px + if solo { metrics.card_pad } else { 0.0 }))
                    .rounded(px(metrics.row_radius))
            } else {
                d.px_4()
            }
        })
        .when(has_tile && metrics.two_lines, |d| d.py_1())
        .cursor_pointer()
        .when(selected && !solo, |d| d.bg(ui.selected_row))
        .when(!solo, |d| d.hover(move |d| if selected { d } else { d.bg(ui.hover) }))
        .children(behind)
        .children(tile)
        .child(lines)
}
