//! Terminals & sidebar's live preview: the Terminals list drawn with the sidebar's own cards,
//! heads and rows from the current options, beside the option under the pointer. Coding agents
//! shows the same list with agents at work in it. Appearance's stage is `stage`. The strip puts each on
//! its stage and under it the caption (`preview_column`), so these draw the scene alone.

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

use super::samples::{AGENT_SAMPLES, Card, DENSITY_SAMPLES, SampleHead};
use crate::settings::SettingsStore;
use crate::settings::{Settings, UiColors, settings};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};
use crate::style::{self, Edges, Metrics, TextRole, TileFace};
use crate::theme_bridge;

/// The narrowest a density's cell in Terminals & sidebar's preview gets: a tile and the start of a row's
/// two lines, as in Coding agents' cards. Three side by side need this and the gaps in the stage.
const DENSITY_CELL_MIN_W: f32 = 152.0;
/// The gap between preview cells (`gap_2`).
const GAP: f32 = 8.0;

/// The narrowest an agent's card in Coding agents' preview gets: a tile and the start of its two lines.
/// The cards wrap, so a narrow column shows two and then one, and a wide one all three side by side;
/// either is under the strip's height. Two side by side must fit the narrowest column (356 less the
/// stage's 16 pt at each side is 324, and a gap of `gap_2` between them).
const AGENT_CARD_MIN_W: f32 = 152.0;

/// How many densities fit side by side in a stage `width` wide: all three where each has the room a list's
/// rows need, else as many as do (the current one first), and always one.
fn densities_that_fit(width: f32) -> usize {
    let gap = GAP;
    (((width + gap) / (DENSITY_CELL_MIN_W + gap)).floor() as usize).clamp(1, Density::ALL.len())
}

/// Terminals & sidebar → the Terminals list in every density at once, side by side, each in the sidebar's
/// own surface and drawn by its own rows from the options as they are now (cards, fading, folders, notes).
/// The density in use wears the soft ring and its name in the primary, and the one under the pointer in an
/// option of the Density row is named in full text.
pub(in crate::settings_window) fn draw_sidebar_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let now = Look::of(&s);
    let hovered = window.sidebar_preview.map(|SidebarLook::Density(density)| density);
    // The stage's own sides (`px_4` twice) come off the page's width.
    let room = densities_that_fit(window.page_width - 32.0);
    // The current density first where not all fit, then the ones nearest to it in the order of `ALL`.
    let mut shown: Vec<Density> = Density::ALL.to_vec();
    let at = shown.iter().position(|d| *d == now.density).unwrap_or(0);
    shown.sort_by_key(|d| {
        (Density::ALL.iter().position(|x| x == d).unwrap_or(0) as isize - at as isize).abs()
    });
    shown.truncate(room);
    shown.sort_by_key(|d| Density::ALL.iter().position(|x| x == d).unwrap_or(0));
    let radius = ui.radius_lg;
    let cells = shown.into_iter().map(|density| {
        let current = density == now.density;
        let look = Look { density, ..now };
        let name = div()
            .flex_none()
            .truncate()
            .text_caption()
            .font_weight(FontWeight::MEDIUM)
            .text_color(if current {
                ui.primary
            } else if hovered == Some(density) {
                ui.text
            } else {
                ui.muted
            })
            .child(density.label());
        // The ring is the cell's own overlay, outside its edge, so the surface under it clips alone.
        let surface = div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .size_full()
                    .rounded(radius)
                    .overflow_hidden()
                    .bg(ui.panel)
                    .when(ui.increase_contrast, |d| d.outlined_always(ui, ui.border))
                    .child(cards(density.label(), look, DENSITY_SAMPLES, false, false, ui, cx)),
            )
            .when(current, |d| d.child(style::preview_ring(ui, radius)));
        div().flex_1().min_w_0().flex().flex_col().gap_2().child(name).child(surface)
    });
    div().flex_1().min_h_0().flex().gap_2().children(cells).into_any_element()
}

/// Coding agents → terminals each in a card of its own, with agents working, asking and finished: what the
/// hooks (or the output) tell the sidebar. Two rows of as many cards as the column has the width for (three
/// across, two where it is narrow): stacked in a list they would not fit the strip's height.
pub(in crate::settings_window) fn draw_agents_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let look = Look::of(settings(cx));
    let metrics = Metrics::of(look.density, look.cards).fading(look.fade).with_group_folders(look.folders);
    let plain = plain_face();
    // Two rows of as many cards as fit across the stage (its own sides, `px_4` twice, come off the page's width).
    let columns =
        (((window.page_width - 32.0 + GAP) / (AGENT_CARD_MIN_W + GAP)).floor() as usize).clamp(1, 3);
    let cards: Vec<AnyElement> = AGENT_SAMPLES
        .iter()
        .take(2 * columns)
        .enumerate()
        .map(|(ix, sample)| {
            let row = sample.row(&format!("preview-agents-{ix}"), ui, metrics, &plain, look.notes, cx);
            style::preview_surface(ui)
                .flex_1()
                .min_w(px(AGENT_CARD_MIN_W))
                .py_2()
                .child(style::card_segment_solo(ui, metrics, sample.selected, row))
                .into_any_element()
        })
        .collect();
    // Rows of exactly `columns`, so the cards of a row are as wide as the others' and never stretch to fill.
    let mut cards = cards.into_iter();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children((0..2).map(|_| div().flex().gap_2().children(cards.by_ref().take(columns))))
        .into_any_element()
}

/// A terminal without an agent shows the first agent's logo, as any agent would.
fn plain_face() -> TileFace {
    wormhole_core::agents::AGENTS
        .first()
        .and_then(|agent| agent.icon())
        .map_or(TileFace::Terminal, TileFace::Logo)
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

/// The sample list in `look`, with the sidebar's own header, cards, heads and rows; `framed` draws it as a
/// card of its own, else it fills the sidebar it is put in (Appearance's window).
pub(super) fn list(
    id: &'static str,
    look: Look,
    samples: &[Card],
    framed: bool,
    ui: UiColors,
    cx: &App,
) -> AnyElement {
    cards(id, look, samples, framed, true, ui, cx)
}

/// [`list`] with or without the header ("Terminals" and the +): a preview short of room leaves it out.
pub(super) fn cards(
    id: &'static str,
    look: Look,
    samples: &[Card],
    framed: bool,
    header: bool,
    ui: UiColors,
    cx: &App,
) -> AnyElement {
    let metrics = Metrics::of(look.density, look.cards).fading(look.fade).with_group_folders(look.folders);
    // The sidebar's own header: its title and the + (drawn, not working).
    let head = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::TITLE_ROW_H))
        .px_4()
        .child(style::panel_title(ui, text(SETTINGS_SESSIONS_TERMINALS)))
        .child(div().flex_1())
        .child(style::icon(IconName::Plus).text_color(ui.muted));
    let plain = plain_face();
    let mut column = if framed { style::preview_surface(ui) } else { div() }
        .flex()
        .flex_col()
        .pb(px(metrics.card_gap))
        .when(header, |column| column.child(head));
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
