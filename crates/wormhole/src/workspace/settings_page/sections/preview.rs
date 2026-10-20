//! Terminals & sidebar's live preview: the Terminals list drawn with the sidebar's own cards,
//! heads and rows from the current options, beside the option under the pointer. Appearance's
//! stage is `stage`.

use wormhole_core::sidebar::Density;

use super::*;
use crate::settings::SettingsStore;
use crate::theme_bridge;
use style::{Metrics, RowTone, SessionRow, StatusGlyph, TileFace};

/// A preview list is as wide as the sidebar at its least.
const SIDEBAR_W: f32 = 320.0;

/// Terminals & sidebar → the Terminals list as the options make it, and beside it the list
/// with the option under the pointer.
pub(in crate::workspace::settings_page) fn draw_sidebar_preview(
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
            .child(label(format!("Now · {}", now.density.label())))
            .child(list("now", now, ui, cx)),
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
                .child(label(format!("Previewed while hovering {}", previewed.density.label())))
                .child(list("previewed", previewed, ui, cx)),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().p_4().rounded(ui.radius_lg).outlined(ui, ui.border).bg(ui.background).child(panes))
        .child(div().text_caption().text_color(ui.muted).child(if column {
            "Live preview of the Terminals list. Hovering a density previews it under the current one"
        } else {
            "Live preview of the Terminals list. Hovering a density previews it next to the current one"
        }))
        .into_any_element()
}

/// The options a list is drawn with.
#[derive(Clone, Copy)]
struct Look {
    density: Density,
    cards: bool,
    notes: bool,
}

impl Look {
    fn of(s: &Settings) -> Self {
        Self { density: s.density, cards: s.show_groups_as_cards, notes: s.sidebar_notes }
    }
}

/// A terminal of the sample list.
struct Sample {
    name: &'static str,
    glyph: StatusGlyph,
    detail: &'static str,
    tone: RowTone,
    selected: bool,
    /// Notes and open tasks.
    notes: usize,
}

/// A card of the sample list: a group's head and its rows, or a terminal in no group.
struct Card {
    group: Option<&'static str>,
    rows: &'static [Sample],
}

const SAMPLES: &[Card] = &[
    Card {
        group: None,
        rows: &[Sample {
            name: "scratch",
            glyph: StatusGlyph::Idle,
            detail: "~",
            tone: RowTone::Plain,
            selected: false,
            notes: 0,
        }],
    },
    Card {
        group: Some("Release 1.2"),
        rows: &[
            Sample {
                name: "Sidebar cards",
                glyph: StatusGlyph::Working,
                detail: "Working · 4m",
                tone: RowTone::Plain,
                selected: true,
                notes: 2,
            },
            Sample {
                name: "Fix pile fan-out",
                glyph: StatusGlyph::NeedsYou,
                detail: "Needs you · 2m",
                tone: RowTone::Attention,
                selected: false,
                notes: 0,
            },
            Sample {
                name: "cargo test",
                glyph: StatusGlyph::Running,
                detail: "cargo test --workspace",
                tone: RowTone::Plain,
                selected: false,
                notes: 0,
            },
        ],
    },
    Card {
        group: None,
        rows: &[Sample {
            name: "zsh",
            glyph: StatusGlyph::Idle,
            detail: "~/Projects/terminal/wormhole",
            tone: RowTone::Plain,
            selected: false,
            notes: 0,
        }],
    },
];

/// The sample list in `look`, with the sidebar's own cards, heads and rows.
fn list(id: &'static str, look: Look, ui: UiColors, cx: &App) -> AnyElement {
    let metrics = Metrics::of(look.density, look.cards);
    // The sidebar's own header: its title and the + (drawn, not working).
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::TITLE_ROW_H))
        .px_4()
        .child(style::panel_title(ui, "Terminals"))
        .child(div().flex_1())
        .child(style::icon(IconName::Plus).text_color(ui.muted));
    let claude = wormhole_core::agents::AGENTS.first();
    let face = || match claude.and_then(|agent| agent.icon()) {
        Some(path) => TileFace::Logo(path),
        None => TileFace::Terminal,
    };
    let mut column = div()
        .flex()
        .flex_col()
        .rounded(ui.radius)
        .overflow_hidden()
        .outlined(ui, ui.border)
        .bg(ui.panel)
        .pb(px(metrics.card_gap))
        .child(header);
    for (cx_ix, card) in SAMPLES.iter().enumerate() {
        let mut lines: Vec<AnyElement> = Vec::new();
        let wash = card.group.map(|_| {
            theme_bridge::hsla(
                wormhole_core::sidebar::GroupColor::Blue.color(&cx.global::<SettingsStore>().tokens),
            )
        });
        if let Some(title) = card.group {
            lines.push(
                style::group_head(
                    SharedString::from(format!("preview-{id}-head-{cx_ix}")),
                    ui,
                    style::GroupHead {
                        metrics,
                        name: title.into(),
                        color: wash,
                        folded: false,
                        pinned: false,
                        tiles: Vec::new(),
                        count: card.rows.len(),
                        alert: None,
                        empty: None,
                        on_toggle: Box::new(|_, _, _| {}),
                        spoken_edit: SharedString::default(),
                        spoken_fold: SharedString::default(),
                        on_label: None,
                        name_edit: None,
                    },
                )
                .into_any_element(),
            );
        }
        for (rx, sample) in card.rows.iter().enumerate() {
            let tile = style::status_tile_at(
                SharedString::from(format!("preview-{id}-tile-{cx_ix}-{rx}")),
                ui,
                style::StatusTile {
                    face: face(),
                    glyph: sample.glyph,
                    selected: sample.selected,
                    mini: false,
                },
                metrics.tile,
                cx,
            );
            let mut row = SessionRow::new(div().into_any_element(), sample.name);
            row.selected = sample.selected;
            row.tone = sample.tone;
            row.tile = Some(tile);
            row.card = true;
            row.metrics = metrics;
            if metrics.two_lines {
                row.detail = Some(sample.detail.into());
            }
            if look.notes && sample.notes > 0 {
                row.trail = Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(style::icon(IconName::StickyNote))
                        .child(sample.notes.to_string())
                        .into_any_element(),
                );
            }
            lines.push(
                style::session_row(SharedString::from(format!("preview-{id}-row-{cx_ix}-{rx}")), ui, row)
                    .into_any_element(),
            );
        }
        let last = lines.len().saturating_sub(1);
        for (at, line) in lines.into_iter().enumerate() {
            column = column.child(style::card_segment(ui, metrics, at == 0, at == last, wash, line));
        }
    }
    column.into_any_element()
}
