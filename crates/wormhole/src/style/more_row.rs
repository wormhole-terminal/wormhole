//! The row that opens more: a label, what is behind it ("17"), and a chevron that turns down while the
//! rest is shown under the row. It is a [`super::setting`]'s height, with its hairline above, so it sits
//! among the rows of a list.

use gpui_kit::accesskit::Role;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::disclosure;
use super::scale::{Edges, SETTING_ROW_MIN_H, TextRole};
use crate::settings::UiColors;

/// A row that opens more. `value` says what is behind it (a count); the caller adds `.on_click(…)` that
/// flips what `open` reads, and draws what is shown under the row. A tab stop, so ↩ and Space open it as
/// a click does; VoiceOver reads a button that is expanded or not.
#[track_caller]
pub fn more_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    value: Option<SharedString>,
    open: bool,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .when_some(value.clone(), |d, value| d.aria_description(value))
        .aria_expanded(open)
        .focusable()
        .tab_stop(true)
        .focus_visible(move |d| d.bg(ui.hover))
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .min_h(px(SETTING_ROW_MIN_H))
        .py_1()
        .rule_top(ui)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .child(div().flex_1().min_w_0().truncate().text_body().font_weight(FontWeight::MEDIUM).child(label))
        .children(
            value.map(|value| div().flex_none().text_body().tabular_nums().text_color(ui.muted).child(value)),
        )
        .child(disclosure(ui, !open))
}
