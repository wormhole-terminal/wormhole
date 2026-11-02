//! How status works: with hooks and without.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, TextRole};

/// How far a lead icon hangs left of its text: its width and the gap after it, so the text
/// lines up with the labels of the rows around it.
const HANG: f32 = style::ITEM_LEAD_W + 8.0;

/// A lead icon hung in the margin before its text (see [`HANG`]), at the top of its
/// `.relative()` parent. Placed outside the layout: a negative margin would measure the text
/// around it again in every frame.
pub(super) fn hang(lead: Div) -> Div {
    div().absolute().top_0().left(px(-HANG)).child(lead)
}

/// With hooks and without, side by side, and the promise about the files.
pub(super) fn draw(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let column = |icon: IconName, color: Hsla, title: &'static str, text: &'static str| {
        div()
            .flex_1()
            .min_w_0()
            .relative()
            .flex()
            .gap_2()
            .child(hang(style::lead(style::icon(icon).text_color(color))))
            .child(
                // Its own width, so the words wrap in the column rather than run into the next one.
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().text_body().font_weight(FontWeight::MEDIUM).child(title))
                    .child(div().text_caption().text_color(ui.muted).child(text)),
            )
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .py_3()
        .child(
            div()
                .flex()
                // Room for the second column's hanging icon, and as much again before it.
                .gap(px(2.0 * HANG))
                .child(column(
                    IconName::Check,
                    ui.success,
                    text(SETTINGS_AGENTS_WITH_STATUS_HOOKS),
                    text(SETTINGS_AGENTS_THE_AGENT_TELLS_WORMHOLE_THE_MOMENT),
                ))
                .child(column(
                    IconName::SquareTerminal,
                    ui.muted,
                    text(SETTINGS_AGENTS_WITHOUT),
                    text(SETTINGS_AGENTS_STATUS_IS_READ_FROM_THE_TERMINAL),
                )),
        )
        .child(
            div()
                .text_caption()
                .text_color(ui.muted)
                .child(text(SETTINGS_AGENTS_HOOKS_DO_NOTHING_OUTSIDE_WORMHOLE_ONLY)),
        )
        .into_any_element()
}
