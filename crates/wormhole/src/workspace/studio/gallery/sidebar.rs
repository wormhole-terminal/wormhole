//! The sidebar's parts: group cards and their heads, terminal rows in every state
//! (agents and plain terminals), the status glyphs and the status bar's items.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;
use style::{RowTone, SessionRow, StatusGlyph};
use wormhole_core::agents::{self, AgentId};
use wormhole_core::folder_color::FolderColor;
use wormhole_core::sidebar::Bracket;
use wormhole_core::sidebar::GroupColor;

use super::Env;
use super::view::GalleryView;
use crate::settings::UiColors;
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge;

/// The sidebar's width (and its minimum in the app).
const COLUMN_W: f32 = 350.0;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let app: &App = cx;
    let glyph = |id: &'static str, glyph: StatusGlyph| style::status_glyph(id, ui, glyph, app);
    let row = |id: &'static str,
               g: AnyElement,
               detail: Option<&'static str>,
               tone,
               selected,
               hint: Option<&str>| {
        style::session_row(
            id,
            ui,
            SessionRow {
                detail: detail.map(SharedString::from),
                tone,
                selected,
                hint: hint.map(|h| SharedString::from(h.to_string())),
                ..SessionRow::new(g, if tone == RowTone::Attention { "diff perf" } else { "sidebar polish" })
            },
        )
        .into_any_element()
    };
    // A group's card as the list draws it: the head (the name alone, its pin, the chevron), its rows
    // under a wash of the group's colour; a folded group keeps its terminals' tiles, their count and
    // an alert dot; a pinned group with nothing in it is a line with a +; a terminal in no group is a
    // card of one row.
    let metrics = style::Metrics::default();
    let blue = theme_bridge::hsla(GroupColor::Blue.color(env.tokens));
    let tile = |id: &'static str, glyph: StatusGlyph, size: f32| {
        style::status_tile_at(
            id,
            ui,
            style::StatusTile { face: style::TileFace::Terminal, glyph, selected: false, mini: false },
            size,
            app,
        )
    };
    let head = |id: &'static str,
                name: &'static str,
                color: Option<Hsla>,
                folded: bool,
                pinned: bool,
                tiles: Vec<AnyElement>,
                alert: Option<Hsla>,
                empty: bool,
                folder: Option<&'static str>| {
        style::group_head(
            id,
            ui,
            style::GroupHead {
                metrics,
                name: name.into(),
                tooltip: None,
                folder: folder.map(SharedString::from),
                color,
                folded,
                pinned,
                count: tiles.len(),
                tiles,
                alert,
                empty: empty.then(|| {
                    let add: style::ClickHandler = Box::new(|_, _, _| {});
                    ("New terminal in group".into(), add)
                }),
                add: None,
                on_toggle: Box::new(|_, _, _| {}),
                spoken_edit: name.into(),
                spoken_fold: name.into(),
                on_label: None,
                label_anchor: None,
                name_edit: None,
            },
        )
    };
    let tile_row =
        |id: &'static str, name: &'static str, glyph: StatusGlyph, detail: &'static str, selected| {
            style::session_row(
                id,
                ui,
                SessionRow {
                    detail: Some(detail.into()),
                    selected,
                    tile: Some(tile(id, glyph, style::TILE)),
                    card: true,
                    metrics,
                    ..SessionRow::new(div().into_any_element(), name)
                },
            )
        };
    // A card is washed with its colour moved just far enough for the text to read (`GroupColor::wash`);
    // the head's dot keeps the colour itself.
    let wash = |color: GroupColor| Some(theme_bridge::hsla(color.wash(env.tokens)));
    let card = |top: bool, bottom: bool, wash: Option<Hsla>, content: AnyElement| {
        style::card_segment(ui, metrics, top, bottom, wash, content)
    };
    let anatomy = || {
        column(ui)
            .child(caption(
                ui,
                "A group's card · folded · pinned and empty · with a folder · a terminal in no group",
            ))
            .child(
                env.mark(
                    "sidebar.card",
                    card(
                        true,
                        false,
                        wash(GroupColor::Blue),
                        env.mark(
                            "sidebar.band",
                            head(
                                "g-head-open",
                                "Release 1.2",
                                Some(blue),
                                false,
                                false,
                                vec![],
                                None,
                                false,
                                None,
                            ),
                        )
                        .into_any_element(),
                    ),
                ),
            )
            .child(card(
                false,
                false,
                wash(GroupColor::Blue),
                env.mark(
                    "sidebar.row",
                    tile_row("g-row-anatomy-1", "Sidebar cards", StatusGlyph::Working, "Working · 4m", true),
                )
                .into_any_element(),
            ))
            .child(card(
                false,
                true,
                wash(GroupColor::Blue),
                tile_row(
                    "g-row-anatomy-2",
                    "Fix pile fan-out",
                    StatusGlyph::NeedsYou,
                    "Needs you · 2m",
                    false,
                )
                .into_any_element(),
            ))
            .child(card(
                true,
                true,
                wash(GroupColor::Pink),
                head(
                    "g-head-folded",
                    "Review",
                    Some(theme_bridge::hsla(GroupColor::Pink.color(env.tokens))),
                    true,
                    true,
                    vec![
                        tile("g-folded-1", StatusGlyph::Working, style::TILE_MINI),
                        tile("g-folded-2", StatusGlyph::NeedsYou, style::TILE_MINI),
                        tile("g-folded-3", StatusGlyph::Idle, style::TILE_MINI),
                    ],
                    Some(ui.agent_attention),
                    false,
                    None,
                )
                .into_any_element(),
            ))
            .child(card(
                true,
                true,
                wash(GroupColor::Orange),
                head(
                    "g-head-empty",
                    "Docs",
                    Some(theme_bridge::hsla(GroupColor::Orange.color(env.tokens))),
                    false,
                    true,
                    vec![],
                    None,
                    true,
                    None,
                )
                .into_any_element(),
            ))
            // A group with a folder: its path under the name (after it in Dense).
            .child(card(
                true,
                true,
                wash(GroupColor::Blue),
                head(
                    "g-head-folder",
                    "Release 1.2",
                    Some(blue),
                    false,
                    false,
                    vec![],
                    None,
                    false,
                    Some("~/Projects/wormhole"),
                )
                .into_any_element(),
            ))
            // A terminal in no group: the row is its card.
            .child(style::card_segment_solo(
                ui,
                metrics,
                false,
                tile_row("g-row-loose", "scratch", StatusGlyph::Idle, "~", false),
            ))
    };

    // Every row state, two lines and one.
    let states = |two_lines: bool| {
        let line = |text: &'static str| two_lines.then_some(text);
        let id = |a: &'static str, b: &'static str| if two_lines { a } else { b };
        column(ui)
            .child(caption(ui, if two_lines { "Two lines" } else { "One line" }))
            .child(row(
                id("g-r2-normal", "g-r1-normal"),
                glyph(id("g-g2-normal", "g-g1-normal"), StatusGlyph::Working),
                line("Editing sidebar.rs…"),
                RowTone::Plain,
                false,
                None,
            ))
            .child(div().bg(ui.hover).child(row(
                id("g-r2-hover", "g-r1-hover"),
                glyph(id("g-g2-hover", "g-g1-hover"), StatusGlyph::Working),
                line("Editing sidebar.rs…"),
                RowTone::Plain,
                false,
                None,
            )))
            .child(row(
                id("g-r2-selected", "g-r1-selected"),
                glyph(id("g-g2-selected", "g-g1-selected"), StatusGlyph::Working),
                line("Editing sidebar.rs…"),
                RowTone::Plain,
                true,
                None,
            ))
            .child(row(
                id("g-r2-needs", "g-r1-needs"),
                glyph(id("g-g2-needs", "g-g1-needs"), StatusGlyph::NeedsYou),
                line("Needs you · Approve cargo test?"),
                RowTone::Attention,
                false,
                None,
            ))
            .child(row(
                id("g-r2-done", "g-r1-done"),
                glyph(id("g-g2-done", "g-g1-done"), StatusGlyph::Done),
                line("Finished · 12m ago"),
                RowTone::Done,
                false,
                None,
            ))
            .child(row(
                id("g-r2-failed", "g-r1-failed"),
                glyph(id("g-g2-failed", "g-g1-failed"), StatusGlyph::Failed),
                line("Failed · rsync failed"),
                RowTone::Failed,
                false,
                None,
            ))
            .child(row(
                id("g-r2-hint", "g-r1-hint"),
                glyph(id("g-g2-hint", "g-g1-hint"), StatusGlyph::Working),
                line("Editing sidebar.rs…"),
                RowTone::Plain,
                true,
                Some("⌘1"),
            ))
    };

    // The nine colours a group wears, each as a washed head; and the eight a folder can wear (the
    // Project tab's choices, the dots) drawn from this theme.
    let group_colors: Vec<(GroupColor, Hsla)> =
        GroupColor::ALL.iter().map(|&color| (color, theme_bridge::hsla(color.color(env.tokens)))).collect();
    let washed = || {
        column(ui)
            .child(caption(ui, "Grey · Blue · Red · Yellow · Green · Pink · Purple · Cyan · Orange"))
            .children(group_colors.iter().enumerate().map(|(ix, &(color, hsla))| {
                let id = Box::leak(format!("g-head-{}", color.key()).into_boxed_str());
                card(
                    true,
                    true,
                    wash(color),
                    head(id, color.label(), Some(hsla), false, ix % 3 == 0, vec![], None, false, None)
                        .into_any_element(),
                )
            }))
    };
    let colors = FolderColor::ALL.map(|color| (color, theme_bridge::hsla(color.color(env.tokens))));
    let choices = div()
        .flex()
        .flex_wrap()
        .items_center()
        .child(style::color_choice("g-color-none", ui, "None", None, false, true))
        .children(colors.iter().enumerate().map(|(ix, (color, hsla))| {
            style::color_choice(("g-color", ix), ui, color.label(), Some(*hsla), ix == 4, true)
        }));
    // Where the project sets the colour (its `config.json` locks it, or holds a hex colour): the
    // choices are dimmed and the person can't pick.
    let locked_choices = div()
        .flex()
        .flex_wrap()
        .items_center()
        .child(style::color_choice("g-locked-none", ui, "None", None, false, false))
        .children(colors.iter().enumerate().map(|(ix, (color, hsla))| {
            style::color_choice(("g-locked", ix), ui, color.label(), Some(*hsla), ix == 4, false)
        }));
    let menu_dots = div()
        .flex()
        .items_center()
        .gap_2()
        .children(colors.iter().map(|(_, hsla)| style::color_dot(ui, *hsla, style::COLOR_DOT_MENU)));
    let bands = || {
        column(ui)
            .child(caption(
                ui,
                "A folder's colour (the Project tab): the choices · the choices a project sets · dots",
            ))
            .child(div().px_4().py_2().child(env.mark("folder.color", choices)))
            .child(div().px_4().pb_2().child(locked_choices))
            .child(div().px_4().pb_2().child(menu_dots))
            .child(caption(
                ui,
                "A branch chip: the git branch and its changes, where the sidebar's rows show one",
            ))
            .child(
                div()
                    .px_4()
                    .pb_2()
                    .child(env.mark("branch.chip", style::branch_chip(ui, Some("ui-unify"), Some(3)))),
            )
    };

    // Terminals without an agent say what runs; an idle agent shows its logo.
    let code_font = crate::fonts::code_font(app).0;
    let plain =
        |id: &'static str, name: &'static str, g: StatusGlyph, command: Option<&'static str>| SessionRow {
            detail: command.map(SharedString::from),
            detail_font: command.map(|_| code_font.clone()),
            ..SessionRow::new(glyph(id, g), name)
        };
    let codex = agents::agent(AgentId::Codex);
    let plain_rows = || {
        column(ui)
            .child(caption(ui, "Plain terminals · a pin · an idle agent's logo"))
            .child(style::session_row(
                "g-plain-tests",
                ui,
                SessionRow {
                    detail_trail: Some("1:12".into()),
                    ..plain(
                        "g-plain-g1",
                        "tests",
                        StatusGlyph::Terminal { unread: false },
                        Some("cargo test -p wormhole-core"),
                    )
                },
            ))
            .child(style::session_row(
                "g-plain-prod",
                ui,
                SessionRow {
                    trail: Some(glyph("g-plain-bell", StatusGlyph::Bell)),
                    ..plain(
                        "g-plain-g2",
                        "prod",
                        StatusGlyph::Terminal { unread: false },
                        Some("ssh deploy@prod-1"),
                    )
                },
            ))
            .child(style::session_row(
                "g-plain-dev",
                ui,
                SessionRow {
                    emphasis: true,
                    detail_trail: Some("new output".into()),
                    ..plain(
                        "g-plain-g3",
                        "dev server",
                        StatusGlyph::Terminal { unread: true },
                        Some("npm run dev"),
                    )
                },
            ))
            .child(style::session_row(
                "g-plain-pinned",
                ui,
                SessionRow {
                    mark: Some(Icon::new(IconName::Pin).xsmall().text_color(ui.faint).into_any_element()),
                    ..plain("g-plain-g4", "zsh", StatusGlyph::Shell, None)
                },
            ))
            .child(style::session_row(
                "g-agent-logo",
                ui,
                SessionRow::new(
                    style::agent_logo(codex.icon(), codex.monogram, style::ITEM_LEAD_W, ui.muted),
                    "rate limits",
                ),
            ))
    };

    // A split's rows share one card (no mark of their own); the chain mark shows where the
    // members aren't together.
    let split_row = |id: &'static str,
                     g: StatusGlyph,
                     name: &'static str,
                     selected: bool,
                     trail: Option<AnyElement>,
                     line: Option<Bracket>| {
        style::session_row(
            id,
            ui,
            SessionRow {
                detail: Some("Idle".into()),
                selected,
                trail,
                // These rows lead with a glyph, 16 in (`px_4`), not a tile: the line runs through it.
                behind: line.map(|part| {
                    style::split_line(part, 16.0 + style::ITEM_LEAD_W / 2.0, ui).into_any_element()
                }),
                ..SessionRow::new(glyph(Box::leak(format!("{id}-glyph").into_boxed_str()), g), name)
            },
        )
        .relative()
    };
    let split_rows = || {
        column(ui)
            .child(caption(ui, "A split: its rows in one card, a line along them · the chain mark"))
            .child(split_row("g-split-a", StatusGlyph::Working, "claude", true, None, Some(Bracket::Start)))
            .child(split_row("g-split-b", StatusGlyph::Idle, "api-server", false, None, Some(Bracket::Mid)))
            .child(split_row("g-split-c", StatusGlyph::Idle, "zsh", false, None, Some(Bracket::End)))
            .child(split_row(
                "g-split-d",
                StatusGlyph::Idle,
                "docs",
                false,
                Some(style::chain_mark(("g-split-chain", 1u64), ui).into_any_element()),
                None,
            ))
    };

    // What a terminal dragged over the list shows: the row it would make a split with, and
    // the line between two rows, full width outside any bracket and inset inside one.
    let drop_row = |id: &'static str, marks: Vec<AnyElement>| {
        div()
            .relative()
            .child(row(
                id,
                glyph(Box::leak(format!("{id}-glyph").into_boxed_str()), StatusGlyph::Idle),
                Some("Idle"),
                RowTone::Plain,
                false,
                None,
            ))
            .children(marks)
    };
    let drops = || {
        column(ui)
            .child(caption(ui, "A terminal dragged over: a row · between rows · inside a bracket"))
            .child(drop_row("g-drop-row", vec![style::row_drop_target(ui).into_any_element()]))
            .child(drop_row("g-drop-line", vec![style::drop_line(ui, false, true).into_any_element()]))
            .child(drop_row("g-drop-inset", vec![style::drop_line(ui, true, true).into_any_element()]))
            .child(drop_row("g-drop-after", Vec::new()))
    };

    let glyphs = || {
        div().flex().flex_wrap().gap_4().p_2().children(
            [
                ("g-glyph-working", StatusGlyph::Working, "Working"),
                ("g-glyph-running", StatusGlyph::Running, "Running"),
                ("g-glyph-needs", StatusGlyph::NeedsYou, "Needs you"),
                ("g-glyph-done", StatusGlyph::Done, "Done"),
                ("g-glyph-idle", StatusGlyph::Idle, "Idle"),
                ("g-glyph-bell", StatusGlyph::Bell, "Bell"),
                ("g-glyph-failed", StatusGlyph::Failed, "Failed"),
                ("g-glyph-terminal", StatusGlyph::Terminal { unread: false }, "Terminal"),
                ("g-glyph-unread", StatusGlyph::Terminal { unread: true }, "Unread"),
                ("g-glyph-shell", StatusGlyph::Shell, "Shell"),
            ]
            .map(|(id, g, label)| {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(glyph(id, g))
                    .child(div().text_caption().text_color(ui.muted).child(label))
            }),
        )
    };

    let status_bar = || {
        style::status_bar(ui, false)
            // The terminal that has waited longest, as a button with its tile; the rest as numbers.
            .child(
                style::status_attention("g-status-attention", ui, false)
                    .child(style::status_tile(
                        "g-status-attention-tile",
                        ui,
                        style::StatusTile {
                            face: style::TileFace::Terminal,
                            glyph: StatusGlyph::NeedsYou,
                            selected: false,
                            mini: true,
                        },
                        cx,
                    ))
                    .child("Fix pile fan-out")
                    .child(div().text_color(ui.attention_text).child("needs you · 2m")),
            )
            .child(
                style::status_item("g-status-working", ui, false, true)
                    .child(glyph("g-status-spin", StatusGlyph::Working))
                    .child("2 working"),
            )
            .child(
                style::status_item("g-status-done", ui, false, false)
                    .child(div().text_color(ui.agent_done).child(style::icon(IconName::CircleCheck)))
                    .child("1 done"),
            )
            .child(div().flex_1())
            .child(style::status_separator(ui))
            .child(style::status_text(ui).child("claude · 90×40"))
    };

    // The page builds one piece at a time (a piece is a list item), each as wide as the sidebar.
    match part {
        0 => anatomy().into_any_element(),
        1 => washed().into_any_element(),
        2 => bands().into_any_element(),
        3 => div().flex().flex_wrap().gap_4().child(states(true)).child(states(false)).into_any_element(),
        4 => plain_rows().into_any_element(),
        5 => split_rows().into_any_element(),
        6 => drops().into_any_element(),
        _ => div()
            .flex()
            .flex_col()
            .gap_4()
            .child(env.mark("status.glyph", glyphs()))
            .child(env.mark("status.bar", status_bar()))
            .into_any_element(),
    }
}

/// A slice of the sidebar at its width at most (a narrow gallery column
/// squeezes it), its surface, an outline.
fn column(ui: UiColors) -> Div {
    div().flex().flex_col().w_full().max_w(px(COLUMN_W)).bg(ui.panel).outlined(ui, ui.border)
}

fn caption(ui: UiColors, text: &'static str) -> Div {
    div().px_4().py_1().text_caption().text_color(ui.muted).child(text)
}
