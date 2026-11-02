//! The page's last line: where an agent's notifications are set, with the way there, and "How status works",
//! which opens what the status hooks do under the line.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;

use super::how;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let open = window.status_open;
    let line = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x_4()
        .rule_top(ui)
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_AGENTS_FOOTER_LINE)))
        .child(
            style::link_button("open-notifications", ui, text(SETTINGS_AGENTS_OPEN_NOTIFICATIONS), true)
                .on_click(cx.listener(|this, _, _, cx| this.show_section(Section::Notifications, cx))),
        )
        .child(
            style::link_button("how-status-works", ui, text(SETTINGS_AGENTS_HOW_STATUS_WORKS), false)
                .role(gpui_kit::accesskit::Role::Button)
                .aria_expanded(open)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status_open = !this.status_open;
                    cx.notify();
                })),
        );
    div().flex().flex_col().child(line).children(open.then(|| how::draw(cx))).into_any_element()
}
