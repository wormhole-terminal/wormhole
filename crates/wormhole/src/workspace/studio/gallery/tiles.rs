//! The status tiles in every state, a card of terminals and the terminal's tab.

use super::*;
use style::{
    RowTone, SessionRow, StatusGlyph, StatusTile, TILE, TILE_DENSE_FOLD, TILE_MINI, TILE_SMALL, TILE_TITLE,
    TerminalTab, TileFace,
};
use wormhole_core::agents::AgentId;

pub(super) fn render(_studio: &StudioWindow, env: &Env, cx: &mut Context<StudioWindow>) -> AnyElement {
    let ui = env.ui;
    let app: &App = cx;
    let claude = || TileFace::Agent(AgentId::Claude);
    let tile = |id: &'static str, face: TileFace, glyph: StatusGlyph, selected: bool, mini: bool| {
        style::status_tile(id, ui, StatusTile { face, glyph, selected, mini }, app)
    };
    let tiles = tile_states(ui, app);

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
            style::Metrics::default(),
            true,
            false,
            None,
            row("g-card-1", StatusGlyph::Working, "sidebar polish", "Editing tabs.rs", RowTone::Plain, true),
        ))
        .child(style::card_segment(
            ui,
            style::Metrics::default(),
            false,
            true,
            None,
            row("g-card-2", StatusGlyph::NeedsYou, "diff perf", "Waiting for you", RowTone::Attention, false),
        ));

    let tab = |id: &'static str, glyph: StatusGlyph, name: &'static str, selected| {
        style::terminal_tab(
            id,
            ui,
            TerminalTab {
                tile: style::status_tile_at(
                    id,
                    ui,
                    StatusTile { face: claude(), glyph, selected, mini: true },
                    TILE_SMALL,
                    app,
                ),
                name: name.into(),
                selected,
                hint: None,
                close: selected
                    .then(|| style::close_x(id, ui, style::SelectedFill::Plain).into_any_element()),
            },
        )
        .into_any_element()
    };
    let switch = |id: &'static str, alert: bool| {
        style::group_switcher(
            id,
            ui,
            style::GroupSwitch {
                name: "Release 1.2".into(),
                alert,
                spoken: "Group Release 1.2, 3 terminals".into(),
            },
        )
    };
    let tabs = div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(switch("g-switch", true))
        .child(style::tab_divider(ui, 16.0))
        .child(tab("g-tab-1", StatusGlyph::Working, "sidebar polish", true))
        .child(tab("g-tab-2", StatusGlyph::Done, "release", false))
        .child(style::tab_divider(ui, 14.0))
        .child(tab("g-tab-3", StatusGlyph::Idle, "cargo test", false));

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(caption(ui, "Status tiles · every state at 32, 20, 18, 16 and 14 pt"))
        .child(env.mark("status.tile", tiles))
        .child(caption(ui, "A card of terminals"))
        .child(card)
        .child(caption(ui, "Tab bar · the group switcher with its amber dot, the open tab and the others"))
        .child(tabs)
        .into_any_element()
}

/// The sidebar's width.
const COLUMN_W: f32 = 350.0;

fn caption(ui: UiColors, text: &'static str) -> Div {
    div().px_4().py_1().text_caption().text_color(ui.muted).child(text)
}

/// Every state of the tile at every size, the same terminals in each row: a terminal at rest, an
/// agent at rest, working, a command running, needs you, done, failed, the bell, new output, an
/// agent with no logo (its monogram), and last the same on a selected row. The light and the
/// breathing edge move; under Reduce Motion they stand still (set it in Settings to see them so).
fn tile_states(ui: UiColors, app: &App) -> Div {
    let agent = |id: AgentId| TileFace::Agent(id);
    let states: [(&str, TileFace, StatusGlyph); 11] = [
        ("terminal", TileFace::Terminal, StatusGlyph::Shell),
        ("agent", agent(AgentId::Claude), StatusGlyph::Idle),
        ("working", agent(AgentId::Claude), StatusGlyph::Working),
        ("running", TileFace::Terminal, StatusGlyph::Running),
        ("needs", agent(AgentId::Claude), StatusGlyph::NeedsYou),
        ("done", agent(AgentId::Claude), StatusGlyph::Done),
        ("failed", agent(AgentId::Gemini), StatusGlyph::Failed),
        ("bell", TileFace::Terminal, StatusGlyph::Bell),
        ("output", TileFace::Terminal, StatusGlyph::Terminal { unread: true }),
        ("monogram", agent(AgentId::Codex), StatusGlyph::Done),
        ("gemini", agent(AgentId::Gemini), StatusGlyph::Working),
    ];
    let sizes =
        [("32", TILE), ("20", TILE_MINI), ("18", TILE_TITLE), ("16", TILE_SMALL), ("14", TILE_DENSE_FOLD)];
    let line = |label: String, selected: bool, size: f32| {
        let row = states.iter().fold(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .px_2()
                .py_2()
                .rounded(ui.radius)
                .when(selected, |d| d.bg(ui.selected_row)),
            |row, (name, face, glyph)| {
                let id = SharedString::from(format!("g-tile-{label}-{name}"));
                let tile = StatusTile { face: face.clone(), glyph: *glyph, selected, mini: false };
                row.child(if selected {
                    // The badge is cut out in the selected row's own colour.
                    style::status_tile_on(ElementId::Name(id), ui, tile, size, ui.selected_row, app)
                } else {
                    style::status_tile_at(ElementId::Name(id), ui, tile, size, app)
                })
            },
        );
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().px_2().text_caption().text_color(ui.muted).child(label))
            .child(row)
    };
    let mut column = div().flex().flex_col().gap_1().px_2().py_1();
    for (label, size) in sizes {
        column = column.child(line(format!("{label} pt"), false, size));
    }
    column.child(line("32 pt on a selected row".to_string(), true, TILE))
}
