//! Appearance's live preview: a small whole window on a plain backdrop, as wide as the narrowest column
//! has room for. It is built from the parts the window itself draws, at their real size: the sidebar's
//! Terminals list (`preview::list`) with its one terminal, the title row's branch chip and tile, the status
//! bar's items, and a terminal in the terminal's own font, size and colours. A change to any of those
//! parts shows here by itself; only the arrangement is this file's, and it is short of what a window
//! shows (a sidebar's one terminal, four lines of a transcript) so that all of the window's edges are in.
//!
//! The traffic lights are the system's, drawn here as a window in the background has them; the blur
//! behind a window isn't drawn (it is the window's own effect view, behind everything GPUI paints),
//! which the caption says.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::theme::fonts::FontSlot;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::preview::{self, Look, caption};
use super::samples::STAGE_SAMPLES;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::preview_column::FRAMED_H;
use crate::settings_window::{SettingsWindow, ui as window_ui};
use crate::style::{self, Edges, StatusGlyph, StatusTile, TextRole, TileFace};
use crate::theme_bridge::hsla;

/// The backdrop fills the scene's frame. The window in it is the Terminals list's own parts at their size:
/// the traffic lights' row, the list's header and one terminal's card (66 pt at the roomiest density), and
/// the status bar's row under them, with `WINDOW_INSET` round it. Its width fits the narrowest column
/// (`WINDOW_MIN`: 640 less the nav and the page's padding is 356, less the backdrop's line).
const STAGE_H: f32 = FRAMED_H;
const WINDOW_INSET: f32 = 8.0;
const WINDOW_W: f32 = 340.0;
/// The sidebar's width in the miniature.
const SIDEBAR_W: f32 = 140.0;
/// The traffic lights as macOS draws them (`style::LIGHTS_X`, `style::ASSUMED_LIGHTS_SPAN`).
const LIGHT: f32 = 12.0;
const LIGHT_GAP: f32 = 8.0;
/// The terminal's inset: 16 at the sides, 12 at the top (Aligned A-01).
const TERM_INSET_X: f32 = 16.0;
const TERM_INSET_Y: f32 = 8.0;

/// Appearance → the window in miniature: theme, primary, opacity, fonts, density and the status
/// bar as they are now.
pub(in crate::settings_window) fn draw_stage(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    // The main window's colours, whatever this window's look.
    let ui = crate::settings::ui(cx);
    let store = cx.global::<SettingsStore>();
    let status_bar = store.settings_without_zen().status_bar;
    let surfaces = style::window_surfaces(cx, true);
    let look = Look::of(settings(cx));
    // The backdrop and its outline are this window's, like the other previews' frames; only the
    // miniature wears the main window's colours.
    let chrome = window_ui(cx);
    // The miniature's outline takes a line of the border's width, so what sits flush inside it rounds
    // to the window's radius less that width (the clip is square, `Edges::corners`).
    let inner = (ui.radius_lg - ui.border_width).max(px(0.0));
    let flat = px(0.0);
    // With a status bar it closes the window's bottom, otherwise the sidebar and the terminal do.
    let (side_bottom, term_bottom) = if status_bar { (flat, flat) } else { (inner, inner) };
    let lights = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(LIGHT_GAP))
        .h(px(style::TITLE_ROW_H))
        .pl(px(style::LIGHTS_X))
        .children((0..3).map(|_| div().size(px(LIGHT)).rounded_full().bg(ui.border_strong)));
    let sidebar = div()
        .w(px(SIDEBAR_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(surfaces.panel)
        .corners(inner, flat, flat, side_bottom)
        .rule_right(ui)
        .child(lights)
        .child(preview::list("stage", look, STAGE_SAMPLES, false, ui, cx));
    let content = div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(ui, cx))
        .child(terminal(cx).corners(flat, flat, term_bottom, flat));
    let window = div()
        .flex_none()
        .w(px(WINDOW_W))
        .h(px(STAGE_H - 2.0 * WINDOW_INSET))
        .flex()
        .flex_col()
        .rounded(ui.radius_lg)
        .overflow_hidden()
        .outlined(ui, ui.border_strong)
        .bg(surfaces.background)
        .child(div().flex_1().min_h_0().flex().child(sidebar).child(content))
        .when(status_bar, |d| d.child(status(ui, cx).corners(flat, flat, inner, inner)));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .h(px(STAGE_H))
                .w_full()
                .rounded(chrome.radius_lg)
                .overflow_hidden()
                .outlined(chrome, chrome.border)
                .bg(chrome.panel)
                .child(window),
        )
        .child(caption(ui, text(SETTINGS_APPEARANCE_STAGE_CAPTION)))
        .into_any_element()
}

/// The terminal's title row: its folder, the branch chip, and the agent at work in it.
fn title_row(ui: UiColors, cx: &App) -> Div {
    let tile = style::status_tile_at(
        "stage-title-tile",
        ui,
        StatusTile {
            face: TileFace::Agent(AgentId::Claude),
            glyph: StatusGlyph::Working,
            selected: false,
            mini: true,
        },
        style::TILE_SMALL,
        cx,
    );
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::TITLE_ROW_H))
        .px_3()
        .rule_bottom(ui)
        .text_body()
        .child(style::branch_chip(ui, Some("main"), None)) // text: sample (a made-up branch of the preview)
        .child(div().flex_none().child(tile))
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .text_color(ui.text)
                .child("Sidebar cards"), // text: sample (a made-up terminal of the preview)
        )
}

/// The terminal: an agent's transcript in the terminal's font, size, line height and colours.
fn terminal(cx: &App) -> Div {
    let store = cx.global::<SettingsStore>();
    let colors = &store.tokens.terminal;
    let (font, _) = crate::fonts::font(FontSlot::Terminal, cx);
    let (fg, green, yellow, blue) =
        (hsla(colors.foreground), hsla(colors.ansi[2]), hsla(colors.ansi[3]), hsla(colors.ansi[4]));
    let muted = Hsla { a: 0.6, ..fg };
    let line = |parts: &[(&'static str, Hsla)]| {
        div()
            .flex()
            .whitespace_nowrap()
            .children(parts.iter().map(|(text, color)| div().text_color(*color).child(*text)))
    };
    // text: sample (the preview's made-up transcript, below)
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .px(px(TERM_INSET_X))
        .py(px(TERM_INSET_Y))
        .bg(hsla(colors.background))
        .font_family(font)
        .text_size(px(store.settings.font_size))
        .line_height(relative(store.settings.line_height))
        .text_color(fg)
        .child(line(&[("wormhole", blue), (" on ", muted), ("main", green)])) // text: sample
        .child(line(&[("$ ", muted), ("claude", fg)])) // text: sample
        .child(line(&[("● ", green), ("Read ", fg), ("rows.rs", muted)])) // text: sample
        .child(line(&[("✻ ", yellow), ("Working…", yellow)])) // text: sample
}

/// The status bar with what it shows while an agent waits and another works.
fn status(ui: UiColors, cx: &App) -> Div {
    let tile = style::status_tile_at(
        "stage-status-tile",
        ui,
        StatusTile {
            face: TileFace::Agent(AgentId::Codex),
            glyph: StatusGlyph::NeedsYou,
            selected: false,
            mini: true,
        },
        style::TILE_SMALL,
        cx,
    );
    style::status_bar(ui, false)
        .child(
            style::status_attention("stage-status-attention", ui, false)
                .child(tile)
                .child(div().text_color(ui.attention_text).child("needs input · 2m")), // text: sample (the preview's made-up wait)
        )
        .child(style::status_item("stage-status-working", ui, false, false).child("1 working")) // text: sample (a made-up count)
}
