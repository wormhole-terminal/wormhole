//! Terminals & sidebar's live preview: the Terminals list drawn with the sidebar's own cards,
//! heads and rows from the current options, beside the option under the pointer. Coding agents
//! shows the same list with agents at work in it. Appearance's stage is `stage`.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::Density;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::samples::{AGENT_SAMPLES, Card, SAMPLES, SampleHead};
use crate::settings::SettingsStore;
use crate::settings::{Settings, UiColors, settings};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};
use crate::style::{self, Metrics, TextRole, TileFace};
use crate::theme_bridge;

/// A preview list is as wide as the sidebar at its least.
const SIDEBAR_W: f32 = 320.0;

/// Terminals & sidebar → the Terminals list as the options make it, and beside it the list
/// with the option under the pointer.
pub(in crate::settings_window) fn draw_sidebar_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let now = Look::of(&s);
    let label =
        |text: String| div().text_caption().font_weight(FontWeight::MEDIUM).text_color(ui.muted).child(text);
    // In the column the two lists stack: a column's width fits one.
    let column = window.preview_column;
    let mut panes = div().flex().when(column, |d| d.flex_col()).gap_4().w_full().child(
        div()
            .flex_1()
            .min_w_0()
            .max_w(px(SIDEBAR_W))
            .flex()
            .flex_col()
            .gap_2()
            .child(label(settings_sessions_preview_now(now.density.label())))
            .child(list("now", now, SAMPLES, true, ui, cx)),
    );
    if let Some(SidebarLook::Density(density)) = window.sidebar_preview
        && density != now.density
    {
        let previewed = Look { density, ..now };
        panes = panes.child(
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(SIDEBAR_W))
                .flex()
                .flex_col()
                .gap_2()
                .child(label(settings_sessions_preview_hovering(previewed.density.label())))
                .child(list("previewed", previewed, SAMPLES, true, ui, cx)),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(style::preview_frame(ui).child(panes))
        .child(div().text_caption().text_color(ui.muted).child(if column {
            text(SETTINGS_SESSIONS_PREVIEW_CAPTION_STACKED)
        } else {
            text(SETTINGS_SESSIONS_PREVIEW_CAPTION_SIDE_BY_SIDE)
        }))
        .into_any_element()
}

/// Coding agents → the Terminals list as the options make it, with an agent working, one asking and
/// one finished: what the hooks (or the output) tell the sidebar.
pub(in crate::settings_window) fn draw_agents_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let look = Look::of(settings(cx));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(style::preview_frame(ui).child(div().max_w(px(SIDEBAR_W)).child(list(
            "agents",
            look,
            AGENT_SAMPLES,
            true,
            ui,
            cx,
        ))))
        .child(div().text_caption().text_color(ui.muted).child(text(SETTINGS_AGENTS_PREVIEW_CAPTION)))
        .into_any_element()
}

/// The options a list is drawn with.
#[derive(Clone, Copy)]
pub(super) struct Look {
    density: Density,
    cards: bool,
    fade: bool,
    folders: bool,
    notes: bool,
}

impl Look {
    pub(super) fn of(s: &Settings) -> Self {
        Self {
            density: s.density,
            cards: s.show_groups_as_cards,
            fade: s.fade_group_colors,
            folders: s.show_group_folders,
            notes: s.sidebar_notes,
        }
    }
}

/// The sample list in `look`, with the sidebar's own cards, heads and rows; `framed` draws it as a
/// card of its own, else it fills the sidebar it is put in (Appearance's window).
pub(super) fn list(
    id: &'static str,
    look: Look,
    samples: &[Card],
    framed: bool,
    ui: UiColors,
    cx: &App,
) -> AnyElement {
    let metrics = Metrics::of(look.density, look.cards).fading(look.fade).with_group_folders(look.folders);
    // The sidebar's own header: its title and the + (drawn, not working).
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::TITLE_ROW_H))
        .px_4()
        .child(style::panel_title(ui, text(SETTINGS_SESSIONS_TERMINALS)))
        .child(div().flex_1())
        .child(style::icon(IconName::Plus).text_color(ui.muted));
    // A terminal without an agent shows the first agent's logo, as any agent would.
    let plain = wormhole_core::agents::AGENTS
        .first()
        .and_then(|agent| agent.icon())
        .map_or(TileFace::Terminal, TileFace::Logo);
    let mut column = if framed { style::preview_surface(ui) } else { div() }
        .flex()
        .flex_col()
        .pb(px(metrics.card_gap))
        .child(header);
    for (cx_ix, card) in samples.iter().enumerate() {
        let mut lines: Vec<AnyElement> = Vec::new();
        let tokens = &cx.global::<SettingsStore>().tokens;
        let blue = wormhole_core::view::sidebar::GroupColor::Blue;
        // The head's dot is the colour itself; the card is washed with it moved for the text.
        let mark = card.group.map(|_| theme_bridge::hsla(blue.color(tokens)));
        let wash = card.group.map(|_| theme_bridge::hsla(blue.wash(tokens)));
        if let Some(title) = card.group {
            lines.push(
                SampleHead {
                    id: SharedString::from(format!("preview-{id}-head-{cx_ix}")),
                    name: title,
                    folder: card.folder,
                    color: mark,
                    count: card.rows.len(),
                    folded: false,
                    summary: None,
                }
                .draw(ui, metrics),
            );
        }
        for (rx, sample) in card.rows.iter().enumerate() {
            lines.push(sample.row(
                &format!("preview-{id}-{cx_ix}-{rx}"),
                ui,
                metrics,
                &plain,
                look.notes,
                cx,
            ));
        }
        let last = lines.len().saturating_sub(1);
        for (at, line) in lines.into_iter().enumerate() {
            column = column.child(style::card_segment(ui, metrics, at == 0, at == last, wash, line));
        }
    }
    column.into_any_element()
}
