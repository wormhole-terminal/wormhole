//! Two-line rows: a feature, an action or a prompt, and a commit of the History tab.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::bands::CARD_INSET;
use crate::style::scale::{CARD_ROW_RADIUS, ITEM_LEAD_W, META_LINE, NAME_LINE, ROW_TWO_LINE_H, TextRole};

use super::{SECOND_LINE_GAP, lead};

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
                    d.child(crate::style::buttons::row_actions(
                        (id.clone(), "actions"),
                        group,
                        false,
                        actions,
                    ))
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
    /// How long ago, as `wormhole_core::view::panel::age` says it.
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
