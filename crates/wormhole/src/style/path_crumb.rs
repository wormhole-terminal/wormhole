//! A path as the title row writes it: an icon, the parents in regular weight and the last part in
//! medium. The terminal's folder and the file a diff or a viewer covers the terminal with are drawn by it,
//! so a path reads the same wherever it stands; what a click, a rest or a right-click on it do is the
//! window's (the hover card, the Finder menu).

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::TextRole;
use crate::settings::UiColors;

/// The crumb of a path. `text` is the parents with their slashes and the last part (`None`: the row has
/// room for the icon alone). `shrinks`: the text gives way to what is beside the crumb, the folders
/// before the name, instead of keeping its width (a row that cut the path itself keeps it). `soft` colours
/// the icon and the parents; the name is the text colour. Add `.on_click(…)` and the rest.
#[track_caller]
pub fn path_crumb(
    id: impl Into<ElementId>,
    ui: UiColors,
    icon: AnyElement,
    text: Option<(SharedString, SharedString)>,
    shrinks: bool,
    soft: Hsla,
) -> Stateful<Div> {
    let text = text.map(|(parents, last)| {
        if shrinks {
            // The folders give way first; the name stays whole as long as they have anything left to
            // give, then it is cut too.
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis_start()
                        .child(parents),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_shrink(0.01)
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ui.text)
                        .child(last),
                )
        } else {
            div()
                .flex()
                .flex_none()
                .child(div().flex_none().child(parents))
                .child(div().flex_none().font_weight(FontWeight::MEDIUM).text_color(ui.text).child(last))
        }
    });
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .text_label()
        .font_weight(FontWeight::NORMAL)
        .text_color(soft)
        .cursor_pointer()
        .hover(|d| d.text_color(ui.text))
        .map(|d| if shrinks { d.flex_1().min_w_0() } else { d.flex_none() })
        .child(div().flex_none().child(icon))
        .children(text)
}
