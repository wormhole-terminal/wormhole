//! Appearance's live preview: a window over a wallpaper at its real size, cut off where the column
//! ends. It is built from the parts the window itself draws: the sidebar's Terminals list (the same
//! list as Terminals & sidebar's preview, `preview::list`), the title row's crumbs and branch chip,
//! the status bar's items, and a terminal in the terminal's own font, size and colours. A change to
//! any of those parts shows here by itself; only the arrangement is this file's.
//!
//! The traffic lights are the system's, drawn here as a window in the background has them; the blur
//! behind a window isn't drawn (it is the window's own effect view, behind everything GPUI paints),
//! which the caption says.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::view::layout::SIDEBAR_MIN;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::preview::{self, Look};
use super::samples::SAMPLES;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::SettingsWindow;
use crate::style::{self, Edges, StatusGlyph, StatusTile, TextRole, TileFace};
use crate::theme_bridge::hsla;

/// The wallpaper's height, and how far in from its top left corner the window sits.
const STAGE_H: f32 = 400.0;
const WINDOW_INSET: f32 = 24.0;
/// The window: as wide as the app opens one, so the column shows its left part.
const WINDOW_W: f32 = 1100.0;
/// The traffic lights as macOS draws them (`style::LIGHTS_X`, `style::ASSUMED_LIGHTS_SPAN`).
const LIGHT: f32 = 12.0;
const LIGHT_GAP: f32 = 8.0;
/// The terminal's inset: 16 at the sides, 12 at the top (Aligned A-01).
const TERM_INSET_X: f32 = 16.0;
const TERM_INSET_Y: f32 = 12.0;
/// The wallpaper's two washes, from the theme's own colours.
const WASH_ALPHA: f32 = 0.55;
const WASH_ANGLE: f32 = 135.0;

/// Appearance → the window over the desktop: theme, primary, opacity, fonts, density and the status
/// bar as they are now.
pub(in crate::settings_window) fn draw_stage(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    // The main window's colours, whatever this window's look.
    let ui = crate::settings::ui(cx);
    let store = cx.global::<SettingsStore>();
    let base = hsla(store.tokens.terminal.background.with_alpha(1.0));
    let status_bar = store.settings.status_bar;
    let surfaces = style::window_surfaces(cx, true);
    let look = Look::of(settings(cx));
    // The theme's own branch magenta, not the one in effect: a branch color picked below recolors
    // the chips, never the desktop behind the window.
    let magenta = store.base_tokens().get("git.branch").map(hsla).unwrap_or(ui.branch);
    let wallpaper = linear_gradient(
        WASH_ANGLE,
        linear_color_stop(Hsla { a: WASH_ALPHA, ..magenta }, 0.0),
        linear_color_stop(Hsla { a: WASH_ALPHA, ..ui.info }, 1.0),
    );
    let lights = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(LIGHT_GAP))
        .h(px(style::TITLE_ROW_H))
        .pl(px(style::LIGHTS_X))
        .children((0..3).map(|_| div().size(px(LIGHT)).rounded_full().bg(ui.border_strong)));
    let sidebar = div()
        .w(px(SIDEBAR_MIN))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(surfaces.panel)
        .rule_right(ui)
        .child(lights)
        .child(preview::list("stage", look, SAMPLES, false, ui, cx));
    let content =
        div().flex_1().min_w_0().h_full().flex().flex_col().child(title_row(ui, cx)).child(terminal(cx));
    let window = div()
        .absolute()
        .top(px(WINDOW_INSET))
        .left(px(WINDOW_INSET))
        .w(px(WINDOW_W))
        .h(px(STAGE_H - 2.0 * WINDOW_INSET))
        .flex()
        .flex_col()
        .rounded(ui.radius_lg)
        .overflow_hidden()
        .outlined(ui, ui.border_strong)
        .bg(surfaces.background)
        .child(div().flex_1().min_h_0().flex().child(sidebar).child(content))
        .when(status_bar, |d| d.child(status(ui, cx)));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .relative()
                .h(px(STAGE_H))
                .w_full()
                .rounded(ui.radius_lg)
                .overflow_hidden()
                .outlined(ui, ui.border)
                .bg(base)
                .child(div().absolute().size_full().bg(wallpaper))
                .child(window),
        )
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_APPEARANCE_STAGE_CAPTION)))
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
        .px_4()
        .rule_bottom(ui)
        .text_body()
        .child(style::icon(IconName::Folder).text_color(ui.muted))
        .child(div().text_color(ui.muted).child("~/Projects/wormhole")) // text: sample (a made-up folder of the preview)
        .child(style::icon(IconName::ChevronRight).text_color(ui.faint))
        .child(style::branch_chip(ui, Some("sidebar-cards"), Some(3))) // text: sample (a made-up branch of the preview)
        .child(style::icon(IconName::ChevronRight).text_color(ui.faint))
        .child(tile)
        .child(div().font_weight(FontWeight::MEDIUM).text_color(ui.text).child("Sidebar cards")) // text: sample (a made-up terminal of the preview)
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
        .child(line(&[("~/Projects/wormhole", blue), (" on ", muted), ("sidebar-cards", green)])) // text: sample
        .child(line(&[("$ ", muted), ("claude", fg)])) // text: sample
        .child(line(&[("● ", green), ("Read ", fg), ("crates/wormhole/src/sidebar/rows.rs", muted)])) // text: sample
        .child(line(&[("● ", green), ("Update ", fg), ("crates/wormhole/src/style/tile.rs", muted)])) // text: sample
        .child(line(&[("  ⎿ ", muted), ("+18 −11", fg)])) // text: sample
        .child(line(&[("● ", green), ("Bash ", fg), ("cargo test -p wormhole", muted)])) // text: sample
        .child(line(&[("  ⎿ ", muted), ("test result: ", fg), ("ok", green), (". 57 passed; 0 failed", fg)])) // text: sample
        .child(line(&[("✻ ", yellow), ("Working… ", yellow), ("(esc to interrupt)", muted)])) // text: sample
        .child(line(&[("> ", fg), ("█", fg)])) // text: sample
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
                .child(div().child("Flaky login test")) // text: sample (a made-up terminal of the preview)
                .child(div().text_color(ui.attention_text).child("needs input · 2m")), // text: sample (the preview's made-up wait)
        )
        .child(style::status_item("stage-status-working", ui, false, false).child("1 working")) // text: sample (a made-up count)
}
