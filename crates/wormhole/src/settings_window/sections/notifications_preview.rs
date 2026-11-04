//! Notifications' preview: two banners as macOS shows them, one for a terminal that needs input and
//! one for an agent that finished, drawn in the theme's colours over the window's own surface.

use gpui_kit::prelude::*;
use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::icons::app_icon_for;
use crate::settings::{SettingsStore, UiColors};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{Edges, TextRole};

/// A banner's width: as wide as macOS draws them.
const BANNER_W: f32 = 344.0;
/// The app's icon in a banner.
const BANNER_ICON: f32 = 32.0;

/// The banners, and what they show under them.
pub(in crate::settings_window) fn draw_notifications_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let icon = app_icon_for(cx.global::<SettingsStore>().tokens.appearance);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_3()
                .p_4()
                .rounded(ui.radius_lg)
                .outlined(ui, ui.border)
                .bg(ui.background)
                .child(banner(
                    ui,
                    icon,
                    "api needs input", // text: sample (a made-up notification of the preview)
                    "Allow edit of src/limits.rs?", // text: sample (a made-up notification of the preview)
                ))
                .child(banner(
                    ui,
                    icon,
                    "Release notes finished", // text: sample (a made-up notification of the preview)
                    "Claude Code · 12m",      // text: sample (a made-up notification of the preview)
                )),
        )
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_NOTIFICATIONS_PREVIEW_CAPTION)))
        .into_any_element()
}

/// One banner: the app's icon, the title over the body, and when it came.
fn banner(ui: UiColors, icon: &'static str, title: &'static str, body: &'static str) -> Div {
    div()
        .w_full()
        .max_w(px(BANNER_W))
        .flex()
        .items_center()
        .gap_3()
        .p_3()
        .rounded(ui.radius_lg)
        .outlined(ui, ui.border)
        .bg(ui.panel)
        .shadow(crate::select::menu_shadow(ui))
        .child(img(icon).size(px(BANNER_ICON)).flex_none())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_body()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(text(SETTINGS_NOTIFICATIONS_PREVIEW_NOW)),
                        ),
                )
                .child(div().truncate().text_body().text_color(ui.muted).child(body)),
        )
}
