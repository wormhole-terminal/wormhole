//! The status tiles in every state, a card of terminals, and the parts of the tab bars (a
//! terminal's tab, a set's chip, label, pile and bottom segment).

use super::*;
use crate::workspace::folder_color;
use style::{
    GroupChip, PileFan, RowTone, SessionRow, StatusGlyph, StatusTile, TabPile, TerminalTab, TileFace,
};
use wormhole_core::agents::{self, AgentId};
use wormhole_core::folder_color::FolderColor;

pub(super) fn render(_studio: &StudioWindow, env: &Env, cx: &mut Context<StudioWindow>) -> AnyElement {
    let ui = env.ui;
    let app: &App = cx;
    let claude = || match agents::agent(AgentId::Claude).icon() {
        Some(path) => TileFace::Logo(path),
        None => TileFace::Monogram("C".into()),
    };
    let tile = |id: &'static str, face: TileFace, glyph: StatusGlyph, selected: bool, mini: bool| {
        style::status_tile(id, ui, StatusTile { face, glyph, selected, mini }, app)
    };
    let tiles = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_3()
        .px_4()
        .py_2()
        .child(tile("g-tile-working", claude(), StatusGlyph::Working, false, false))
        .child(tile("g-tile-running", TileFace::Terminal, StatusGlyph::Running, false, false))
        .child(tile("g-tile-needs", claude(), StatusGlyph::NeedsYou, false, false))
        .child(tile("g-tile-done", claude(), StatusGlyph::Done, false, false))
        .child(tile("g-tile-failed", TileFace::Terminal, StatusGlyph::Failed, false, false))
        .child(tile("g-tile-bell", TileFace::Terminal, StatusGlyph::Bell, false, false))
        .child(tile(
            "g-tile-unread",
            TileFace::Terminal,
            StatusGlyph::Terminal { unread: true },
            false,
            false,
        ))
        .child(tile("g-tile-idle", TileFace::Monogram("Cx".into()), StatusGlyph::Idle, true, false))
        .child(tile("g-tile-mini", claude(), StatusGlyph::Working, false, true))
        .child(tile("g-tile-mini-needs", TileFace::Terminal, StatusGlyph::NeedsYou, false, true));

    let row =
        |id: &'static str, glyph: StatusGlyph, name: &'static str, detail: &'static str, tone, selected| {
            style::session_row(
                id,
                ui,
                SessionRow {
                    detail: Some(detail.into()),
                    tone,
                    selected,
                    tile: Some(tile(id, claude(), glyph, selected, false)),
                    card: true,
                    ..SessionRow::new(div().into_any_element(), name)
                },
            )
            .into_any_element()
        };
    let card = div()
        .w(px(COLUMN_W))
        .flex()
        .flex_col()
        .child(style::card_segment(
            ui,
            true,
            false,
            true,
            false,
            row("g-card-1", StatusGlyph::Working, "sidebar polish", "Editing tabs.rs", RowTone::Plain, true),
        ))
        .child(style::card_segment(
            ui,
            false,
            true,
            true,
            false,
            row("g-card-2", StatusGlyph::NeedsYou, "diff perf", "Waiting for you", RowTone::Attention, false),
        ));

    let tab =
        |id: &'static str, glyph: StatusGlyph, name: &'static str, detail: &'static str, selected, fixed| {
            style::terminal_tab(
                id,
                ui,
                TerminalTab {
                    tile: tile(id, claude(), glyph, selected, false),
                    name: name.into(),
                    detail: Some(detail.into()),
                    tone: RowTone::Plain,
                    detail_font: None,
                    selected,
                    hint: None,
                    fixed,
                },
            )
            .into_any_element()
        };
    let teal = folder_color::band_tint_in(FolderColor::Teal, env.tokens).color;
    let tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .px_4()
        .py_2()
        .child(tab("g-tab-1", StatusGlyph::Working, "sidebar polish", "Editing tabs.rs", true, false))
        .child(tab("g-tab-2", StatusGlyph::Done, "release", "Finished", false, true));
    let chips = div()
        .flex()
        .items_center()
        .gap_1()
        .px_4()
        .py_2()
        .child(style::group_chip(
            "g-chip-work",
            ui,
            GroupChip { color: Some(teal), name: "Work".into(), count: 3, selected: true, alert: None },
        ))
        .child(style::group_chip(
            "g-chip-site",
            ui,
            GroupChip {
                color: None,
                name: "Ungrouped".into(),
                count: 2,
                selected: false,
                alert: Some(ui.agent_attention),
            },
        ))
        .child(style::set_label(ui, Some(teal), "Work"));
    let pile = style::tab_pile(
        "g-pile",
        ui,
        TabPile {
            color: Some(teal),
            name: "api".into(),
            summary: "3 · 1 waiting".into(),
            summary_color: ui.attention_text,
            lead: tile("g-pile-lead", claude(), StatusGlyph::NeedsYou, false, false),
            fan: PileFan::Rest,
        },
    );
    let segment = style::tab_segment("g-segment", ui, Some(teal), "Work")
        .child(tile("g-segment-tile", claude(), StatusGlyph::Working, false, true))
        .child(div().pr_3().text_caption().text_color(ui.muted).child("3"));
    let bars = div().flex().items_center().gap_4().px_4().py_3().child(pile).child(segment);

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(caption(
            ui,
            "Status tiles · working, running, needs you, done, failed, bell, unread, idle, small",
        ))
        .child(tiles)
        .child(caption(ui, "A card of terminals"))
        .child(card)
        .child(caption(ui, "Tabs · shared and fixed width"))
        .child(tabs)
        .child(caption(ui, "Set chips and label"))
        .child(chips)
        .child(caption(ui, "A pile (stacks) and a bottom bar's segment"))
        .child(bars)
        .into_any_element()
}

/// The sidebar's width.
const COLUMN_W: f32 = 350.0;

fn caption(ui: UiColors, text: &'static str) -> Div {
    div().px_4().py_1().text_caption().text_color(ui.muted).child(text)
}
