//! Menu bar & hotkey's preview: a stretch of the menu bar with Wormhole's item, a dot and the count
//! of terminals that need input, and under it the quick panel the item and the hotkey open.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, StatusGlyph, TextRole};

/// The menu bar's height.
const MENU_BAR_H: f32 = 24.0;
/// The item's dot.
const ITEM_DOT: f32 = 8.0;

/// The menu bar, its item and the quick panel.
pub(in crate::settings_window) fn draw_menu_bar_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let app: &App = cx;
    let glyph = |id: &'static str, glyph: StatusGlyph| style::status_glyph(id, ui, glyph, app);
    let time = |text: &'static str| {
        div().text_caption().tabular_nums().text_color(ui.faint).child(text).into_any_element()
    };
    let mut needs = style::PaletteResult::new(
        glyph("menu-preview-needs", StatusGlyph::NeedsYou),
        "api", // text: sample (a made-up terminal of the preview)
    );
    needs.strong = true;
    needs.detail = Some("Allow edit?".into()); // text: sample (a made-up terminal of the preview)
    needs.detail_color = Some(ui.attention_text);
    needs.trailing.push(time("2m")); // text: sample (a made-up time of the preview)
    let mut working = style::PaletteResult::new(
        glyph("menu-preview-working", StatusGlyph::Working),
        "Rate limits", // text: sample (a made-up terminal of the preview)
    );
    working.strong = true;
    working.detail = Some("Working".into()); // text: sample (a made-up terminal of the preview)
    working.trailing.push(time("4m")); // text: sample (a made-up time of the preview)

    // The menu bar: the clock's side, the item with its dot and count.
    let bar = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_4()
        .h(px(MENU_BAR_H))
        .px_3()
        .rounded(ui.radius)
        .bg(ui.panel)
        .outlined(ui, ui.border)
        .text_caption()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .rounded(ui.radius_sm)
                .bg(ui.hover)
                .child(div().size(px(ITEM_DOT)).rounded_full().bg(ui.agent_attention))
                .child(div().font_weight(FontWeight::SEMIBOLD).text_color(ui.text).child("1")), // text: sample (a made-up count)
        )
        .child(style::icon(IconName::Search).text_color(ui.muted))
        .child(div().text_color(ui.text).child("Sun 18:42")); // text: sample (a made-up clock of the preview)
    let panel = style::palette_surface(ui, true)
        .w_full()
        .max_w(px(style::QUICK_PANEL_W))
        .child(
            style::palette_search(ui)
                .child(div().flex_1().text_color(ui.faint).child(text(SETTINGS_MENU_BAR_PREVIEW_SEARCH))),
        )
        .child(style::palette_label(ui, text(SETTINGS_MENU_BAR_PREVIEW_NEEDS_INPUT), Some(1)))
        .child(style::palette_result("menu-preview-needs-row", ui, true, needs))
        .child(style::palette_label(ui, text(SETTINGS_MENU_BAR_PREVIEW_WORKING), None))
        .child(style::palette_result("menu-preview-working-row", ui, false, working));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            style::preview_frame(ui)
                .flex()
                .flex_col()
                .gap_3()
                .child(bar)
                .child(div().flex().justify_end().child(panel)),
        )
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_MENU_BAR_PREVIEW_CAPTION)))
        .into_any_element()
}
