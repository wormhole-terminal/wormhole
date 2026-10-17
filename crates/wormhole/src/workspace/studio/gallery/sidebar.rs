//! The sidebar's parts: folder bands, terminal rows in every state
//! (agents and plain terminals), the status glyphs and the status bar's items.

use super::*;
use crate::theme_bridge;
use crate::workspace::folder_color;
use style::{BandKind, BandParts, FolderLook, FolderMeta, RowTone, Section, SessionRow, StatusGlyph};
use wormhole_core::agents::{self, AgentId};
use wormhole_core::folder_color::FolderColor;
use wormhole_core::layout::status_bar::CONTEXT_METER_W;
use wormhole_core::sidebar::Bracket;

/// The sidebar's width (and its minimum in the app).
const COLUMN_W: f32 = 350.0;

pub(super) fn render(_studio: &StudioWindow, env: &Env, cx: &mut Context<StudioWindow>) -> AnyElement {
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
    // A folder band's second line: its path, the ±N of its changes and its branch (`None`: no
    // repository; `Some(None)`: a detached HEAD). `path_w` cuts the path as a narrow sidebar does.
    let meta = |path: &'static str,
                changes: Option<usize>,
                branch: Option<Option<&'static str>>,
                path_w: Option<f32>| FolderMeta {
        path: path.into(),
        path_w,
        changes,
        branch: branch.map(|b| env.mark("branch.chip", style::branch_tag(ui, b)).into_any_element()),
        pull_request: None,
    };
    let folder_parts = |title: &'static str, section, count: Option<&str>, interactive: bool| BandParts {
        kind: BandKind::Folder,
        section,
        title: title.into(),
        detail: None,
        count: count.map(SharedString::from),
        waiting: None,
        add: if interactive { Some(("New terminal here", Box::new(|_, _, _| {}))) } else { None },
        on_toggle: if interactive { Some(Box::new(|_, _, _| {})) } else { None },
        drop_target: false,
        sticky: false,
        joined: false,
    };
    let folder = |id: &'static str, title: &'static str, section, count: Option<&str>, meta: FolderMeta| {
        style::folder_band(
            id,
            ui,
            folder_parts(title, section, count, true),
            FolderLook::default(),
            Some(meta),
        )
    };

    // The anatomy: an open folder with three terminals, then two folded folders that share
    // a hairline.
    let anatomy = column(ui)
        .child(env.mark(
            "sidebar.band",
            folder(
                "g-band-terminal",
                "terminal",
                Section::Open,
                Some("3"),
                meta("~/Projects/terminal", Some(3), Some(Some("ui-unify")), None),
            ),
        ))
        .child(env.mark(
            "sidebar.row",
            row(
                "g-row-anatomy-1",
                glyph("g-glyph-a1", StatusGlyph::Working),
                Some("Editing sidebar.rs…"),
                RowTone::Plain,
                true,
                None,
            ),
        ))
        .child(env.mark(
            "sidebar.row",
            row(
                "g-row-anatomy-2",
                glyph("g-glyph-a2", StatusGlyph::NeedsYou),
                Some("Needs you · Approve cargo test?"),
                RowTone::Attention,
                false,
                None,
            ),
        ))
        .child(
            style::session_row(
                "g-row-anatomy-3",
                ui,
                SessionRow::new(glyph("g-glyph-a3", StatusGlyph::Idle), "zsh"),
            )
            // How the pointer shows on a row (`state.hover`).
            .bg(ui.hover),
        )
        .child(folder(
            "g-band-dotfiles",
            "dotfiles",
            Section::Collapsed,
            Some("3"),
            meta("~/dotfiles", None, None, None),
        ))
        .child(style::folder_band(
            "g-band-website",
            ui,
            // A terminal folded away in it needs you: the amber dot before the count.
            BandParts {
                joined: true,
                waiting: Some(glyph("g-band-waiting", StatusGlyph::NeedsYou)),
                ..folder_parts("website", Section::Collapsed, Some("2"), true)
            },
            FolderLook::default(),
            Some(meta("~/Projects/website", None, Some(Some("main")), None)),
        ));

    // Every row state, two lines and one.
    let states = |two_lines: bool| {
        let line = |text: &'static str| two_lines.then_some(text);
        let id = |a: &'static str, b: &'static str| if two_lines { a } else { b };
        column(ui)
            .child(caption(ui, if two_lines { "Two lines · 44" } else { "One line · 28" }))
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

    // The band's other states: a terminal dragged over it, and stuck to the top of the list. Their
    // line is too narrow for the whole path.
    let band_state = |id: &'static str, drop_target: bool, sticky: bool| {
        style::folder_band(
            id,
            ui,
            BandParts {
                drop_target,
                sticky,
                add: None,
                on_toggle: None,
                ..folder_parts("api-server", Section::Collapsed, Some("2"), false)
            },
            FolderLook::default(),
            Some(meta(
                "~/Projects/clients/acme/services/api-server",
                Some(1),
                Some(Some("feature/very-long-branch-name")),
                Some(96.0),
            )),
        )
    };
    // ⌘ held: the count drops to 40 %. "No folder": a muted title, one line, with no path or git.
    let held = |id: &'static str, title: &'static str, look: FolderLook, meta: Option<FolderMeta>| {
        style::folder_band(id, ui, folder_parts(title, Section::Collapsed, Some("3"), false), look, meta)
    };
    // A pinned folder's band stays with no terminal in it: its pin after the name, no count.
    let pinned = style::folder_band(
        "g-band-pinned",
        ui,
        BandParts {
            detail: Some(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .text_color(ui.faint)
                    .child(Icon::new(IconName::Pin).xsmall())
                    .into_any_element(),
            ),
            count: None,
            waiting: None,
            ..folder_parts("notes", Section::Open, None, true)
        },
        FolderLook::default(),
        Some(meta("~/Projects/notes", None, None, None)),
    );
    // The compact density keeps a band to one line: the chip and ±N follow the name.
    let compact = style::band(
        "g-band-compact",
        ui,
        BandParts {
            detail: Some(style::branch_chip(ui, Some("ui-unify"), Some(3)).into_any_element()),
            ..folder_parts("terminal", Section::Open, None, true)
        },
    );
    // A folder with a colour: the band is tinted from the left as the title row is, and the choices
    // (Project tab) and the dots (the band's menu) are the same eight, drawn from this theme.
    let tinted = |id: &'static str, color: FolderColor, path: &'static str| {
        style::folder_band(
            id,
            ui,
            folder_parts(color.key(), Section::Open, Some("2"), true),
            FolderLook { tint: Some(folder_color::band_tint_in(color, env.tokens)), ..FolderLook::default() },
            Some(meta(path, Some(2), Some(Some("main")), None)),
        )
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
    let bands = column(ui)
        .child(caption(
            ui,
            "A folder's colour: tinted bands · the choices · the choices a project sets · the dots in the menu",
        ))
        .child(env.mark("folder.color", tinted("g-band-teal", FolderColor::Teal, "~/Projects/api")))
        .child(tinted("g-band-pink", FolderColor::Pink, "~/Projects/site"))
        .child(div().px_4().py_2().child(env.mark("folder.color", choices)))
        .child(div().px_4().pb_2().child(locked_choices))
        .child(div().px_4().pb_2().child(menu_dots))
        .child(caption(
            ui,
            "Drop target · stuck to the top · a cut path · pinned · hover one for its + · compact · no folder",
        ))
        .child(band_state("g-band-drop", true, false))
        .child(band_state("g-band-sticky", false, true))
        .child(held(
            "g-band-held",
            "dotfiles",
            FolderLook { cmd_held: true, ..FolderLook::default() },
            Some(meta("~/dotfiles", None, None, None)),
        ))
        .child(pinned)
        .child(compact)
        .child(held("g-band-unknown", "No folder", FolderLook { unknown: true, ..FolderLook::default() }, None))
        .child(style::folder_band(
            "g-band-detached",
            ui,
            folder_parts("bisect", Section::Open, None, true),
            FolderLook::default(),
            Some(meta("~/Projects/bisect", None, Some(None), None)),
        ));

    // Terminals without an agent say what runs; an idle agent shows its logo.
    let code_font = crate::fonts::code_font(app).0;
    let plain =
        |id: &'static str, name: &'static str, g: StatusGlyph, command: Option<&'static str>| SessionRow {
            detail: command.map(SharedString::from),
            detail_font: command.map(|_| code_font.clone()),
            ..SessionRow::new(glyph(id, g), name)
        };
    let codex = agents::agent(AgentId::Codex);
    let plain_rows = column(ui)
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
        ));

    // A split's rows joined by a bracket, the selected terminal's siblings marked "on
    // screen", and the chain mark where the members aren't together.
    let split_row = |id: &'static str,
                     g: StatusGlyph,
                     name: &'static str,
                     selected: bool,
                     bracket: Option<Bracket>,
                     trail: Option<AnyElement>| {
        style::session_row(
            id,
            ui,
            SessionRow {
                detail: Some("Idle".into()),
                selected,
                trail,
                ..SessionRow::new(glyph(Box::leak(format!("{id}-glyph").into_boxed_str()), g), name)
            },
        )
        .relative()
        .children(bracket.map(|part| style::split_bracket(ui, part)))
    };
    let split_rows = column(ui)
        .child(caption(ui, "A split · on screen beside the selected one · the chain mark"))
        .child(split_row("g-split-a", StatusGlyph::Working, "claude", true, Some(Bracket::Start), None))
        .child(split_row(
            "g-split-b",
            StatusGlyph::Idle,
            "api-server",
            false,
            Some(Bracket::Mid),
            Some(style::on_screen_mark(ui).into_any_element()),
        ))
        .child(split_row(
            "g-split-c",
            StatusGlyph::Idle,
            "zsh",
            false,
            Some(Bracket::End),
            Some(style::on_screen_mark(ui).into_any_element()),
        ))
        .child(split_row(
            "g-split-d",
            StatusGlyph::Idle,
            "docs",
            false,
            None,
            Some(style::chain_mark(("g-split-chain", 1u64), ui).into_any_element()),
        ));

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
    let drops = column(ui)
        .child(caption(ui, "A terminal dragged over: a row · between rows · inside a bracket"))
        .child(drop_row("g-drop-row", vec![style::row_drop_target(ui).into_any_element()]))
        .child(drop_row("g-drop-line", vec![style::drop_line(ui, false, true).into_any_element()]))
        .child(drop_row("g-drop-inset", vec![style::drop_line(ui, true, true).into_any_element()]))
        .child(drop_row("g-drop-after", Vec::new()));

    let glyphs = div().flex().flex_wrap().gap_4().p_2().children(
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
    );

    let status_bar = style::status_bar(ui)
        .child(
            style::status_item("g-status-branch", ui, false, false)
                .child(div().text_color(ui.branch).child(style::icon(IconName::GitBranch)))
                .child(div().text_color(ui.branch).child("ui-unify"))
                .child(div().ml_1().child("±3")),
        )
        .child(style::status_separator(ui))
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
        .child(style::status_context(ui, CONTEXT_METER_W, true, 0.62, false, "62%"))
        .child(style::status_separator(ui))
        .child(style::status_text(ui).child("zsh · 90×40"));

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(div().flex().flex_wrap().gap_4().child(anatomy).child(bands))
        .child(div().flex().flex_wrap().gap_4().child(states(true)).child(states(false)))
        .child(div().flex().flex_wrap().gap_4().child(plain_rows).child(split_rows).child(drops))
        .child(env.mark("status.glyph", glyphs))
        .child(env.mark("status.bar", status_bar))
        .into_any_element()
}

/// A slice of the sidebar at its width at most (a narrow gallery column
/// squeezes it), its surface, an outline.
fn column(ui: UiColors) -> Div {
    div().flex().flex_col().w_full().max_w(px(COLUMN_W)).bg(ui.panel).outlined(ui, ui.border)
}

fn caption(ui: UiColors, text: &'static str) -> Div {
    div().px_4().py_1().text_caption().text_color(ui.muted).child(text)
}
