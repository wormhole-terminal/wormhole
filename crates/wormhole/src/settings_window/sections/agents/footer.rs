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
use crate::settings_window::SettingsWindow;
use crate::style;

pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let open = window.status_open;
    let line = style::footer(text(SETTINGS_AGENTS_FOOTER_LINE))
        .link(
            text(SETTINGS_AGENTS_OPEN_NOTIFICATIONS),
            cx.listener(|this, _, _, cx| this.show_section(Section::Notifications, cx)),
        )
        .link(
            text(SETTINGS_AGENTS_HOW_STATUS_WORKS),
            cx.listener(|this, _, _, cx| {
                this.status_open = !this.status_open;
                cx.notify();
            }),
        );
    div().flex().flex_col().pt_3().child(line).children(open.then(|| how::draw(cx))).into_any_element()
}
