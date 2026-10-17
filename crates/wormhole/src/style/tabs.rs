//! The tab bar a window shows instead of the sidebar (Settings → Show terminals in → Tabs): a
//! terminal's tab, its status tile and its name on one line, like its row in the compact list.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::controls::keycap;
use super::scale::{CARD_ROW_RADIUS, NAME_LINE, TAB_MIN_W, TAB_W, TextRole};
use crate::settings::UiColors;

/// The parts of a [`terminal_tab`].
pub struct TerminalTab {
    /// A `style::status_tile`.
    pub tile: AnyElement,
    pub name: SharedString,
    pub selected: bool,
    /// The ⌘N keycap while ⌘ is held.
    pub hint: Option<SharedString>,
}

/// One terminal's tab: its status tile, then its name; the selected one in the sidebar's
/// selection, the others with a hover fill. Add `.on_click(…)`.
#[track_caller]
pub fn terminal_tab(id: impl Into<ElementId>, ui: UiColors, tab: TerminalTab) -> Stateful<Div> {
    let TerminalTab { tile, name, selected, hint } = tab;
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .h(px(TAB_H))
        .pl(px(TAB_PAD_LEAD))
        .pr(px(TAB_PAD_END))
        .rounded(px(CARD_ROW_RADIUS))
        .overflow_hidden()
        .flex_shrink_1()
        .flex_basis(px(TAB_W))
        .max_w(px(TAB_MAX_W))
        .min_w(px(TAB_MIN_W))
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.selected_row))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(tile)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_1()
                .min_w_0()
                .h(rems(NAME_LINE))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_body()
                        .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                        .text_color(if selected { ui.selected_text } else { ui.text })
                        .child(name),
                )
                .children(hint.map(|hint| keycap(ui, hint))),
        )
}

/// A tab's height (a mini tile and one line), the most a shared one grows to, and its padding:
/// less before the tile than after the text.
pub const TAB_H: f32 = 32.0;
const TAB_MAX_W: f32 = 220.0;
const TAB_PAD_LEAD: f32 = 6.0;
const TAB_PAD_END: f32 = 10.0;
