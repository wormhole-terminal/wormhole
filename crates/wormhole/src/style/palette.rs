//! The parts the command palette and the hotkey's quick panel share:
//! the floating card, its search row and footer, the labels over a group of results and the
//! results themselves, with the characters a search found marked.

use std::ops::Range;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::rows::lead;
use super::scale::{Edges, ITEM_LEAD_W, QP_ROW_H, QP_SEARCH_H, ROW_H, ROW_TWO_LINE_H, TextRole};
use crate::settings::UiColors;

/// The card a palette is drawn on: the elevated surface in a strong outline with the card corners;
/// the caller sets its width ([`super::QUICK_PANEL_W`], or the shortcut sheet's). `float` adds the
/// shadow of something over the window (the quick panel is a window of its own: macOS shadows it).
#[track_caller]
pub fn palette_surface(ui: UiColors, float: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(ui.radius_lg)
        .outlined(ui, ui.border_strong)
        .bg(ui.elevated)
        .text_color(ui.text)
        .when(float, |d| {
            d.shadow(vec![BoxShadow {
                color: ui.shadow,
                offset: point(px(0.), px(FLOAT_SHADOW_Y)),
                blur_radius: px(FLOAT_SHADOW_BLUR),
                spread_radius: px(0.),
                inset: false,
            }])
        })
}

/// The shadow under a palette over the window.
const FLOAT_SHADOW_Y: f32 = 24.0;
const FLOAT_SHADOW_BLUR: f32 = 64.0;

/// The search row along a palette's top ([`QP_SEARCH_H`]): the magnifier at x 16, a hairline
/// under it. Add the field (`Input::new(…).appearance(false)` in a `flex_1` box) and the key cap
/// at its right with `.child(…)`.
#[track_caller]
pub fn palette_search(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .h(px(QP_SEARCH_H))
        .px_4()
        .rule_bottom(ui)
        .child(div().flex_none().text_color(ui.muted).child(super::icon(gpui_kit::assets::IconName::Search)))
}

/// The footer along a palette's bottom ([`QP_ROW_H`], a hairline over it): `style::key_hint`s,
/// and after a `div().flex_1()` a count or a hint at the right, in muted caption text.
#[track_caller]
pub fn palette_footer(ui: UiColors) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_4()
        .h(px(QP_ROW_H))
        .px_4()
        .rule_top(ui)
        .text_caption()
        .text_color(ui.muted)
}

/// The label over a group of results ("Needs input 2"): 12 / 600 muted on a [`ROW_H`] line,
/// sitting on the rows below it, the count faint.
#[track_caller]
pub fn palette_label(ui: UiColors, title: impl Into<SharedString>, count: Option<usize>) -> Div {
    div()
        .flex()
        .flex_none()
        .items_end()
        .gap_2()
        .h(px(ROW_H))
        .px_4()
        .pb_1()
        .text_caption()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ui.muted)
        .child(title.into())
        .children(count.map(|n| {
            div().font_weight(FontWeight::MEDIUM).tabular_nums().text_color(ui.faint).child(n.to_string())
        }))
}

/// `text` with the characters a search found (`marks`, byte ranges) in the primary, 600: on the
/// selected result, the tint's text color.
pub fn marked_text(ui: UiColors, text: SharedString, marks: &[Range<usize>], selected: bool) -> StyledText {
    let style = HighlightStyle {
        color: Some(if selected { ui.list_selected_text } else { ui.primary }),
        font_weight: Some(FontWeight::SEMIBOLD),
        ..Default::default()
    };
    // A range that doesn't fit the text (stale marks) would panic in the text system.
    let fits = |r: &Range<usize>| {
        r.end <= text.len() && text.is_char_boundary(r.start) && text.is_char_boundary(r.end)
    };
    let highlights: Vec<_> = marks.iter().filter(|r| fits(r)).map(|r| (r.clone(), style)).collect();
    StyledText::new(text).with_highlights(highlights)
}

/// What a [`palette_result`] shows.
pub struct PaletteResult {
    /// In the lead column: a status glyph, an action's icon.
    pub lead: AnyElement,
    pub title: SharedString,
    /// The characters of the title a search found.
    pub marks: Vec<Range<usize>>,
    /// The title in 500 rather than 400: a terminal's name.
    pub strong: bool,
    /// After the title, in caption text: where, what it is doing, the setting's page.
    pub detail: Option<SharedString>,
    /// The detail's color when it says something (needs input, finished); muted otherwise.
    pub detail_color: Option<Hsla>,
    /// The detail in this (code) font: a command.
    pub detail_font: Option<SharedString>,
    /// A line under the title (the agent's question), which makes the result [`ROW_TWO_LINE_H`]
    /// tall.
    pub second: Option<SharedString>,
    /// At the right: a key cap, a time, a setting's value.
    pub trailing: Vec<AnyElement>,
}

impl PaletteResult {
    /// A result with just its lead and title; set the rest field by field.
    pub fn new(lead: AnyElement, title: impl Into<SharedString>) -> Self {
        Self {
            lead,
            title: title.into(),
            marks: Vec::new(),
            strong: false,
            detail: None,
            detail_color: None,
            detail_font: None,
            second: None,
            trailing: Vec::new(),
        }
    }
}

/// A result in the command palette or the quick panel: [`QP_ROW_H`] tall
/// (two lines: [`ROW_TWO_LINE_H`]) and edge to edge on the elevated surface, the lead in the lead
/// column and the title at x 40, the detail after it, what sits at the right last. The selected
/// one takes `list.selected` with its title (600) in `list.selected.text` and its detail in
/// `list.selected.muted`. Add `.on_click(…)`.
#[track_caller]
pub fn palette_result(
    id: impl Into<ElementId>,
    ui: UiColors,
    selected: bool,
    parts: PaletteResult,
) -> Stateful<Div> {
    let PaletteResult {
        lead: lead_el,
        title,
        marks,
        strong,
        detail,
        detail_color,
        detail_font,
        second,
        trailing,
    } = parts;
    let muted = if selected { ui.list_selected_muted } else { ui.muted };
    let weight = match (selected, strong) {
        (true, _) => FontWeight::SEMIBOLD,
        (false, true) => FontWeight::MEDIUM,
        (false, false) => FontWeight::NORMAL,
    };
    let two_lines = second.is_some();
    div()
        .id(id)
        .flex()
        .flex_col()
        .justify_center()
        .w_full()
        .h(px(if two_lines { ROW_TWO_LINE_H } else { QP_ROW_H }))
        .px_4()
        .text_body()
        .cursor_pointer()
        .text_color(if selected { ui.list_selected_text } else { ui.text })
        .when(selected, |d| d.bg(ui.list_selected))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .w_full()
                .min_w_0()
                .child(lead(lead_el))
                .child(
                    div()
                        .flex_none()
                        .max_w(relative(TITLE_MAX))
                        .truncate()
                        .font_weight(weight)
                        .child(marked_text(ui, title, &marks, selected)),
                )
                .children(detail.map(|detail| {
                    div()
                        .min_w_0()
                        .truncate()
                        .text_caption()
                        .text_color(detail_color.unwrap_or(muted))
                        .when_some(detail_font, |d, font| d.font_family(font))
                        .child(detail)
                }))
                .child(div().flex_1())
                .children(trailing.into_iter().map(|t| div().flex_none().flex().items_center().child(t))),
        )
        .children(second.map(|second| {
            div()
                .pl(px(SECOND_LINE_INDENT))
                .min_w_0()
                .truncate()
                .text_caption()
                .text_color(muted)
                .child(second)
        }))
}

/// The share of a result's line its title may take before it is cut; the detail gives way first.
const TITLE_MAX: f32 = 0.7;
/// Where a result's second line starts: under its title, past the lead column and the gap (`_2`).
const SECOND_LINE_INDENT: f32 = ITEM_LEAD_W + 8.0;
