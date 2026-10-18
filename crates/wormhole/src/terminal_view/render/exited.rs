//! The bar along the bottom of a pane whose shell has ended.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keymap;

use crate::settings::store_in;
use crate::style::{Edges, TextRole};
use crate::terminal_view::{TerminalEvent, TerminalView};

impl TerminalView {
    /// The bar along the bottom of a pane whose shell has ended: what ended and how (or that its
    /// session host went away with it), a new shell in the same tab, or closing the tab.
    pub(super) fn render_exited(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = store_in(&self.profile, cx).ui;
        let what = self.exited_words();
        div()
            .id("exited-bar")
            .absolute()
            .left_0()
            .right_0()
            .bottom_0()
            .h(px(crate::style::TITLE_ROW_H))
            .flex()
            .items_center()
            .gap_2()
            .pl_4()
            .pr_2()
            .bg(ui.panel)
            .rule_top(ui)
            .cursor(CursorStyle::Arrow)
            // The pane below it would start a selection.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().text_color(ui.muted).child(crate::style::icon(IconName::Terminal)))
            .child(div().flex_1().min_w_0().truncate().text_caption().text_color(ui.muted).child(what))
            .child(
                Button::new("exited-restart")
                    .small()
                    .outline()
                    .icon(IconName::RefreshCw)
                    .label("Restart")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Restart))),
            )
            .child(
                div()
                    .id("exited-close")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(px(crate::style::CONTROL_H))
                    .px_2()
                    .rounded(ui.radius)
                    .text_caption()
                    .text_color(ui.muted)
                    .cursor_pointer()
                    .hover(move |d| d.bg(ui.hover).text_color(ui.text))
                    .child("Close terminal")
                    .child(crate::style::keycap(ui, keymap::glyphs(keymap::ActionId::CloseTerminal)))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Close))),
            )
            .into_any_element()
    }
}
