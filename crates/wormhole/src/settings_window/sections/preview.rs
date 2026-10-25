//! Terminals & sidebar's live preview: the Terminals list drawn with the sidebar's own cards,
//! heads and rows from the current options, beside the option under the pointer. Coding agents
//! shows the same list with agents at work in it. Appearance's stage is `stage`.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{Metrics, RowTone, SessionRow, StatusGlyph, TileFace};
use wormhole_core::agents::AgentId;
use wormhole_core::view::sidebar::Density;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::settings::SettingsStore;
use crate::settings::{Settings, UiColors, settings};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};
use crate::style::{self, Edges, TextRole};
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
        .child(div().p_4().rounded(ui.radius_lg).outlined(ui, ui.border).bg(ui.background).child(panes))
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
        .child(
            div()
                .p_4()
                .rounded(ui.radius_lg)
                .outlined(ui, ui.border)
                .bg(ui.background)
                .child(div().max_w(px(SIDEBAR_W)).child(list("agents", look, AGENT_SAMPLES, true, ui, cx))),
        )
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

/// A terminal of the sample list.
pub(super) struct Sample {
    name: &'static str,
    glyph: StatusGlyph,
    detail: &'static str,
    tone: RowTone,
    selected: bool,
    /// Notes and open tasks.
    notes: usize,
    /// The agent at work in it; `None` draws the first agent's logo, as any agent would.
    agent: Option<AgentId>,
}

/// A card of the sample list: a group's head and its rows, or a terminal in no group.
pub(super) struct Card {
    group: Option<&'static str>,
    /// The group's folder, as its head shows it.
    folder: Option<&'static str>,
    rows: &'static [Sample],
}

pub(super) const SAMPLES: &[Card] = &[
    Card {
        group: None,
        folder: None,
        rows: &[Sample {
            name: "scratch",
            glyph: StatusGlyph::Idle,
            detail: "~",
            tone: RowTone::Plain,
            selected: false,
            notes: 0,
            agent: None,
        }],
    },
    Card {
        group: Some("Release 1.2"), // text: sample (a made-up terminal of the preview)
        folder: Some("~/Projects/wormhole"), // text: sample (a made-up folder of the preview)
        rows: &[
            Sample {
                name: "Sidebar cards", // text: sample (a made-up terminal of the preview)
                glyph: StatusGlyph::Working,
                detail: "Working · 4m", // text: sample (a made-up terminal of the preview)
                tone: RowTone::Plain,
                selected: true,
                notes: 2,
                agent: None,
            },
            Sample {
                name: "Fix pile fan-out", // text: sample (a made-up terminal of the preview)
                glyph: StatusGlyph::NeedsYou,
                detail: "Needs input · 2m", // text: sample (a made-up terminal of the preview)
                tone: RowTone::Attention,
                selected: false,
                notes: 0,
                agent: None,
            },
            Sample {
                name: "cargo test", // text: sample (a made-up terminal of the preview)
                glyph: StatusGlyph::Running,
                detail: "cargo test --workspace", // text: sample (a made-up terminal of the preview)
                tone: RowTone::Plain,
                selected: false,
                notes: 0,
                agent: None,
            },
        ],
    },
    Card {
        group: None,
        folder: None,
        rows: &[Sample {
            name: "zsh",
            glyph: StatusGlyph::Idle,
            detail: "~/Projects/terminal/wormhole", // text: sample (a made-up folder of the preview)
            tone: RowTone::Plain,
            selected: false,
            notes: 0,
            agent: None,
        }],
    },
];

const AGENT_SAMPLES: &[Card] = &[Card {
    group: Some("api"),             // text: sample (a made-up project of the preview)
    folder: Some("~/Projects/api"), // text: sample (a made-up folder of the preview)
    rows: &[
        Sample {
            name: "Rate limits", // text: sample (a made-up terminal of the preview)
            glyph: StatusGlyph::Working,
            detail: "Working · 4m", // text: sample (a made-up terminal of the preview)
            tone: RowTone::Plain,
            selected: true,
            notes: 0,
            agent: Some(AgentId::Claude),
        },
        Sample {
            name: "Flaky login test", // text: sample (a made-up terminal of the preview)
            glyph: StatusGlyph::NeedsYou,
            detail: "Needs input · Allow edit?", // text: sample (a made-up terminal of the preview)
            tone: RowTone::Attention,
            selected: false,
            notes: 0,
            agent: Some(AgentId::Codex),
        },
        Sample {
            name: "Release notes", // text: sample (a made-up terminal of the preview)
            glyph: StatusGlyph::Done,
            detail: "Finished · 1m", // text: sample (a made-up terminal of the preview)
            tone: RowTone::Done,
            selected: false,
            notes: 0,
            agent: Some(AgentId::OpenCode),
        },
    ],
}];

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
    let claude = wormhole_core::agents::AGENTS.first();
    let face = |agent: Option<AgentId>| match (agent, claude.and_then(|agent| agent.icon())) {
        (Some(agent), _) => TileFace::Agent(agent),
        (None, Some(path)) => TileFace::Logo(path),
        (None, None) => TileFace::Terminal,
    };
    let mut column = div()
        .flex()
        .flex_col()
        .when(framed, |d| d.rounded(ui.radius).overflow_hidden().outlined(ui, ui.border).bg(ui.panel))
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
                style::group_head(
                    SharedString::from(format!("preview-{id}-head-{cx_ix}")),
                    ui,
                    style::GroupHead {
                        metrics,
                        name: title.into(),
                        tooltip: None,
                        folder: card.folder.map(SharedString::from),
                        color: mark,
                        folded: false,
                        pinned: false,
                        tiles: Vec::new(),
                        count: card.rows.len(),
                        alert: None,
                        empty: None,
                        add: None,
                        on_toggle: Box::new(|_, _, _| {}),
                        spoken_edit: SharedString::default(),
                        spoken_fold: SharedString::default(),
                        on_label: None,
                        label_anchor: None,
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
                    face: face(sample.agent),
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
