//! Notifications' preview: banners as macOS shows them, for a terminal that needs input, one that finished,
//! a command that failed and another that asks, drawn in the theme's colours over the window's own surface.
//! Two rows of as many as the page's width holds: the strip is one fixed height
//! (`preview_column::PREVIEW_H`) and holds two rows of banners whole, never a third cut off at its edge.

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

/// The room a stage's banners have at most: banners are as wide as macOS draws them, so as many as fit side
/// by side go in a row, two rows deep (the stage's height holds two, not three).
const ROWS: usize = 2;
/// The gap between banners (`gap_2`).
const GAP: f32 = 8.0;

/// What the banners say: a title and a body each, for a terminal that needs input, one that finished, a
/// command that failed and another that asks.
const BANNERS: [(&str, &str); 4] = [
    ("api needs input", "Allow edit of src/limits.rs?"), // text: sample (a made-up notification of the preview)
    ("Rate limits finished", "57 tests passed in 4m"), // text: sample (a made-up notification of the preview)
    ("cargo test failed", "Exited with code 101"),     // text: sample (a made-up notification of the preview)
    ("Flaky login test needs input", "Allow Bash: npm test?"), // text: sample (a made-up notification of the preview)
];

/// The banners, as many as the stage has room for: two rows of as many columns as fit the page's width
/// (a narrow one shows two, one over the other).
pub(in crate::settings_window) fn draw_notifications_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let icon = app_icon_for(cx.global::<SettingsStore>().tokens.appearance);
    // The stage's own sides (`px_4` twice) come off the page's width.
    let room = window.page_width - 32.0;
    let columns = (((room + GAP) / (BANNER_W + GAP)).floor() as usize).clamp(1, BANNERS.len() / ROWS);
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .children(BANNERS.chunks(columns).take(ROWS).map(|row| {
            div().flex().gap_2().children(row.iter().map(|(title, body)| banner(ui, icon, title, body)))
        }))
        .into_any_element()
}

/// One banner: the app's icon, the title over the body, and when it came.
fn banner(ui: UiColors, icon: &'static str, title: &'static str, body: &'static str) -> Div {
    div()
        .w(px(BANNER_W))
        .flex_none()
        .flex()
        .items_center()
        .gap_3()
        .p_3()
        .rounded(ui.radius_lg)
        .when(ui.increase_contrast, |banner| banner.outlined_always(ui, ui.border))
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
