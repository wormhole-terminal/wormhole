//! Terminals & sidebar's and Coding agents' preview: a miniature of the Terminals list beside a terminal
//! pane, compiled from the design's `ListMock` (`ui/generated/list_mock.rs`). This file fills the component
//! from the settings (density, cards, folders, the row details) and from the agent registry (each agent's
//! tile); where everything sits is the design's.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::agents::{AgentId, Tile, agent};
use wormhole_core::theme::tokens::ThemeTokens;
use wormhole_core::view::sidebar::{Density, GroupColor};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::card::{Words, card};
use crate::settings::{Settings, SettingsStore, settings};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};
use crate::theme_bridge::{hsla, token};
use crate::ui::{ListMockProps, Look, PaneLine, Row};

/// How a terminal's status reads in the list, which sets its dot and its second line's colour.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Idle,
    Working,
    NeedsInput,
    Done,
}

/// A terminal of the sample list. `agent` is the one at work in it (its tile and mark), none a plain
/// terminal.
struct Sample {
    name: &'static str,
    status: &'static str,
    tone: Tone,
    agent: Option<AgentId>,
    /// The files it changed, as "+added" and "-removed".
    changes: Option<(&'static str, &'static str)>,
    notes: bool,
}

// text: sample (the made-up terminals of the preview: names, statuses and the files they changed)
const SESSIONS: [Sample; 3] = [
    Sample { name: "scratch", status: "Idle", tone: Tone::Idle, agent: None, changes: None, notes: false },
    Sample {
        name: "Sidebar cards",
        status: "Working · 4m",
        tone: Tone::Working,
        agent: Some(AgentId::Claude),
        changes: Some(("+48", "−12")),
        notes: true,
    },
    Sample {
        name: "cargo test",
        status: "Finished · 1m",
        tone: Tone::Done,
        agent: None,
        changes: Some(("+3", "−1")),
        notes: false,
    },
];

// text: sample (the made-up terminals of the preview: names and statuses)
const AGENTS: [Sample; 3] = [
    Sample {
        name: "Rate limits",
        status: "Working · 4m",
        tone: Tone::Working,
        agent: Some(AgentId::Claude),
        changes: None,
        notes: false,
    },
    Sample {
        name: "Flaky login test",
        status: "Needs input · Allow edit?",
        tone: Tone::NeedsInput,
        agent: Some(AgentId::Codex),
        changes: None,
        notes: false,
    },
    Sample {
        name: "Release notes",
        status: "Finished · 1m",
        tone: Tone::Done,
        agent: Some(AgentId::OpenCode),
        changes: None,
        notes: false,
    },
];

/// The options the list is drawn with.
struct Options {
    density: Density,
    cards: bool,
    fade: bool,
    folders: bool,
    changes: bool,
    dot: bool,
    notes: bool,
}

impl Options {
    fn of(s: &Settings, hovered: Option<Density>) -> Self {
        Self {
            density: hovered.unwrap_or(s.density),
            cards: s.show_groups_as_cards,
            fade: s.fade_group_colors,
            folders: s.show_group_folders,
            changes: s.sidebar_changes,
            dot: s.sidebar_status_dot,
            notes: s.sidebar_notes,
        }
    }

    /// What the Coding agents page shows: the list as the person has it, with what hooks add (dots).
    fn agents(s: &Settings) -> Self {
        Self { density: Density::Regular, changes: false, dot: true, notes: false, ..Self::of(s, None) }
    }
}

/// Terminals & sidebar → the Terminals list as the options make it; the density under the pointer in the
/// menu replaces the current one while it is there.
pub(in crate::settings_window) fn draw_sidebar_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let hovered = window.sidebar_preview.map(|SidebarLook::Density(density)| density);
    let options = Options::of(settings(cx), hovered);
    list_card(
        &SESSIONS,
        options,
        Words {
            id: "sidebar-preview-card",
            what: SETTINGS_WINDOW_TERMINALS_LIST,
            spoken: SETTINGS_WINDOW_PREVIEW_TERMINALS_LABEL,
            caption: SETTINGS_SESSIONS_PREVIEW_CAPTION,
        },
        "sidebar-list-mock",
        cx,
    )
}

/// Coding agents → the Terminals list with an agent working, one asking and one finished: what the hooks
/// (or the output) tell the sidebar.
pub(in crate::settings_window) fn draw_agents_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let options = Options::agents(settings(cx));
    list_card(
        &AGENTS,
        options,
        Words {
            id: "agents-preview-card",
            what: SETTINGS_WINDOW_TERMINALS_LIST,
            spoken: SETTINGS_WINDOW_PREVIEW_AGENTS_LABEL,
            caption: SETTINGS_AGENTS_PREVIEW_CAPTION,
        },
        "agents-list-mock",
        cx,
    )
}

fn list_card(
    samples: &[Sample],
    options: Options,
    words: Words,
    id: &'static str,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let store = cx.global::<SettingsStore>();
    let tokens = &store.tokens;
    let spoken: SharedString = wormhole_text::text(words.spoken).into();
    let mock = props(samples, &options, tokens, id, spoken);
    card(ui, words, div().child(mock.render(&Look::of(cx))).overflow_hidden())
}

/// The component's props for `samples` under `options`, in `tokens`' colours.
fn props(
    samples: &[Sample],
    options: &Options,
    tokens: &ThemeTokens,
    id: &'static str,
    spoken: SharedString,
) -> ListMockProps {
    let (primary, muted) = (token(tokens, "primary"), token(tokens, "text.muted"));
    let (attention, done) = (token(tokens, "agent.attention"), token(tokens, "agent.done"));
    let on_primary = token(tokens, "text.on_primary");
    let tone_color = |tone: Tone| match tone {
        Tone::Idle => muted,
        Tone::Working => primary,
        Tone::NeedsInput => attention,
        Tone::Done => done,
    };
    let compact = options.density == Density::Compact;
    let dense = options.density == Density::Dense;
    let rows = samples
        .iter()
        .enumerate()
        .map(|(at, sample)| {
            let selected = at == 1;
            let changes = options.changes.then_some(sample.changes).flatten();
            let (added, removed) = changes.unwrap_or_default();
            Row {
                name: sample.name.into(),
                status: sample.status.into(),
                has_status: !dense,
                selected,
                compact,
                dense,
                tile: tile_fill(sample.agent, tokens),
                icon: Some(
                    IconName::from(if sample.agent.is_some() {
                        IconName::Sparkles
                    } else {
                        IconName::Terminal
                    })
                    .into(),
                ),
                dot: options.dot && sample.tone != Tone::Idle,
                dot_color: tone_color(sample.tone),
                status_color: if selected { on_primary } else { tone_color(sample.tone) },
                changes: changes.is_some(),
                added: added.into(),
                removed: removed.into(),
                add_color: token(tokens, "diff.added"),
                remove_color: token(tokens, "diff.removed"),
                notes: options.notes && sample.notes,
            }
        })
        .collect();
    let mark = GroupColor::Blue;
    let line = hsla(mark.color(tokens));
    let term = &tokens.terminal;
    let (fg, bg) = (hsla(term.foreground), hsla(term.background));
    let ansi = |i: usize| hsla(term.ansi[i]);
    // text: sample (the preview's made-up terminal)
    let pane = [
        ("~/Projects/api $ claude", ansi(4)),
        ("✻ Reading src/limits.rs", fg),
        ("✻ Editing 2 files…", fg),
        ("  ⎿  +48 −12", ansi(2)),
        ("Allow edit of src/limits.rs? (y/n)", ansi(3)),
    ]
    .into_iter()
    .map(|(text, color)| PaneLine { text: text.into(), color })
    .collect();
    ListMockProps {
        id: id.into(),
        spoken,
        cards: options.cards,
        flat: !options.cards,
        // The group's colour as a wash over the list: lighter when its colours fade.
        group_fill: if options.cards {
            Hsla { a: if options.fade { 0.14 } else { 0.26 }, ..line }
        } else {
            gpui_kit::transparent_black()
        },
        group_line: Hsla { a: 0.6, ..line },
        group: "api".into(), // text: sample (a made-up group of the preview)
        folder: options.folders.then(|| "~/Projects/api".into()), // text: sample (a made-up folder)
        pane_bg: bg,
        rows,
        pane,
    }
}

/// A terminal's tile: its agent's colour (a brand's is the brand's, not the theme's), else the plain
/// terminal's, which the theme decides.
fn tile_fill(agent_id: Option<AgentId>, tokens: &ThemeTokens) -> Hsla {
    match agent_id.map(|id| agent(id).tile) {
        None => token(tokens, "button.background"),
        Some(Tile::Black) => token(tokens, "tile.background"),
        Some(Tile::Solid(rgb_value) | Tile::Gradient(rgb_value, _)) => rgb(rgb_value).into(),
    }
}
