//! Terminal's preview: two split terminals as the page's options draw them. Each pane has the split
//! pane's own header (`style::pane_header`) and, under "Gaps between split terminals", is a rounded
//! surface of its own with the gap the divider fills between them (`style::PANE_GAP`); without, the two
//! share one surface with a hairline between. The minimap runs down the right edge of each pane while it
//! is on. The text is the terminal's own font, size, line height and colours.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::fonts::FontSlot;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::card::{Words, card};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, PANE_GAP, PANE_INSET, TERMINAL_PAD};
use crate::theme_bridge::hsla;

/// The minimap's column along a pane's right edge, and its bars.
const MINIMAP_W: f32 = 40.0;
const MINIMAP_BAR_H: f32 = 2.0;
const MINIMAP_BAR_GAP: f32 = 2.0;
/// How wide a bar is per character of the line it stands for, and how strong it is drawn.
const MINIMAP_PER_CHAR: f32 = 1.2;
const MINIMAP_ALPHA: f32 = 0.5;

/// One coloured run of a line.
type Run = (&'static str, usize);

/// Colours of a pane's runs, by the ANSI slot or the foreground (`FG`).
const FG: usize = usize::MAX;
const MUTED: usize = usize::MAX - 1;
const GREEN: usize = 2;
const YELLOW: usize = 3;
const BLUE: usize = 4;

const TEST_PANE: [&[Run]; 5] = [
    &[("api ", BLUE), ("$ ", MUTED), ("cargo test", FG)], // text: sample (the preview's made-up terminals)
    &[("running 412 tests", FG)],                         // text: sample (the preview's made-up terminals)
    &[("limits::burst ... ", FG), ("ok", GREEN)],         // text: sample (the preview's made-up terminals)
    &[("limits::refill ... ", FG), ("ok", GREEN)],        // text: sample (the preview's made-up terminals)
    &[("test result: ", FG), ("ok", GREEN)],              // text: sample (the preview's made-up terminals)
];
const LOG_PANE: [&[Run]; 5] = [
    &[("api ", BLUE), ("$ ", MUTED), ("git log --oneline", FG)], // text: sample (the preview's made-up terminals)
    &[("a41f2c9 ", YELLOW), ("limits: refill", FG)], // text: sample (the preview's made-up terminals)
    &[("9e03b11 ", YELLOW), ("limits: burst", FG)],  // text: sample (the preview's made-up terminals)
    &[("77d0a2e ", YELLOW), ("api: routes", FG)],    // text: sample (the preview's made-up terminals)
    &[("api ", BLUE), ("$ ", MUTED)],                // text: sample (the preview's made-up terminals)
];

/// Terminal → two split terminals under the page's options.
pub(in crate::settings_window) fn draw_split_preview(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx);
    let (gaps, minimap) = (s.split_gaps, s.minimap);
    let panes = [("cargo test", TEST_PANE, true), ("git log", LOG_PANE, false)]; // text: sample (made-up terminal names of the preview)
    let mut row = div().flex().w_full();
    if gaps {
        row = row.gap(px(PANE_GAP));
    } else {
        // One surface for both, with the corner a pane's own would have.
        row = row.rounded(ui.radius_lg).overflow_hidden().outlined(ui, ui.border);
    }
    for (ix, (title, lines, focused)) in panes.into_iter().enumerate() {
        row = row.child(pane(ui, ix, title, &lines, focused, gaps, minimap, cx));
    }
    card(
        ui,
        Words {
            id: "terminal-preview-card",
            what: SETTINGS_WINDOW_SPLIT_TERMINALS,
            spoken: SETTINGS_WINDOW_PREVIEW_TERMINAL_LABEL,
            caption: SETTINGS_TERMINAL_PREVIEW_CAPTION,
        },
        style::preview_frame(ui).child(row),
    )
}

/// One pane: its header, its text and, while on, the minimap.
#[allow(clippy::too_many_arguments, reason = "one pane's look, each a fact of the page's options")]
fn pane(
    ui: UiColors,
    ix: usize,
    title: &'static str,
    lines: &[&[Run]],
    focused: bool,
    gaps: bool,
    minimap: bool,
    cx: &App,
) -> Div {
    let store = cx.global::<SettingsStore>();
    let colors = &store.tokens.terminal;
    let (font, _) = crate::fonts::font(FontSlot::Terminal, cx);
    let fg = hsla(colors.foreground);
    let muted = Hsla { a: 0.6, ..fg };
    let color = |slot: usize| match slot {
        FG => fg,
        MUTED => muted,
        slot => hsla(colors.ansi[slot]),
    };
    let header = style::pane_header(("split-preview-header", ix), ui, focused, gaps).child(title);
    let text = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .px(px(if gaps { PANE_INSET } else { PANE_INSET - TERMINAL_PAD }))
        .pb_2()
        .font_family(font)
        .text_size(px(store.settings.font_size))
        .line_height(relative(store.settings.line_height))
        .text_color(fg)
        .children(lines.iter().map(|runs| {
            div()
                .flex()
                .whitespace_nowrap()
                .children(runs.iter().map(|(text, slot)| div().text_color(color(*slot)).child(*text)))
        }));
    div()
        .relative()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(hsla(colors.background))
        .when(gaps, |d| d.rounded(ui.radius_lg).overflow_hidden())
        .when(!gaps && ix > 0, |d| d.rule_left(ui))
        .child(header)
        .child(div().flex().flex_1().min_h_0().child(text).when(minimap, |d| d.child(minimap_of(ui, lines))))
        .when(gaps, |d| d.child(style::pane_outline(ui, focused)))
}

/// The minimap: a bar for each line, as long as the line is.
fn minimap_of(ui: UiColors, lines: &[&[Run]]) -> Div {
    div().flex_none().w(px(MINIMAP_W)).flex().flex_col().gap(px(MINIMAP_BAR_GAP)).pr_2().pt_1().children(
        lines.iter().map(|runs| {
            let chars: usize = runs.iter().map(|(text, _)| text.chars().count()).sum();
            div()
                .h(px(MINIMAP_BAR_H))
                .w(px((chars as f32 * MINIMAP_PER_CHAR).min(MINIMAP_W)))
                .rounded_full()
                .bg(ui.faint.opacity(MINIMAP_ALPHA))
        }),
    )
}
