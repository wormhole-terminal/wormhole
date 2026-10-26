//! The made-up terminals the Settings previews draw: the sample lists of Terminals & sidebar, Coding
//! agents and Appearance's window, and the scenes of Motion. One set, so a terminal is the same
//! wherever a preview shows it, and the text that is not interface text (names, commands, folders) is
//! marked as such in one place.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::view::motion::StatusMotion;

use crate::settings::UiColors;
use crate::style::{
    self, DetailRoll, Metrics, RowTone, SessionRow, StatusGlyph, StatusShift, TextRole, TileFace,
};

/// A terminal of a sample list.
pub(super) struct Sample {
    pub(super) name: &'static str,
    pub(super) glyph: StatusGlyph,
    pub(super) detail: &'static str,
    pub(super) tone: RowTone,
    pub(super) selected: bool,
    /// Notes and open tasks.
    pub(super) notes: usize,
    /// The agent at work in it; `None` is a plain terminal, which the list shows with the first
    /// agent's logo, as any agent would (a scene of Motion draws it as a terminal).
    pub(super) agent: Option<AgentId>,
}

/// A card of a sample list: a group's head and its rows, or a terminal in no group.
pub(super) struct Card {
    pub(super) group: Option<&'static str>,
    /// The group's folder, as its head shows it.
    pub(super) folder: Option<&'static str>,
    pub(super) rows: &'static [Sample],
}

pub(super) const SCRATCH: Sample = Sample {
    name: "scratch", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::Idle,
    detail: "~", // text: sample (a made-up folder of the preview)
    tone: RowTone::Plain,
    selected: false,
    notes: 0,
    agent: None,
};
pub(super) const SIDEBAR_CARDS: Sample = Sample {
    name: "Sidebar cards", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::Working,
    detail: "Working · 4m", // text: sample (a made-up status of the preview)
    tone: RowTone::Plain,
    selected: true,
    notes: 2,
    agent: None,
};
pub(super) const FIX_PILE: Sample = Sample {
    name: "Fix pile fan-out", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::NeedsYou,
    detail: "Needs input · 2m", // text: sample (a made-up status of the preview)
    tone: RowTone::Attention,
    selected: false,
    notes: 0,
    agent: None,
};
pub(super) const CARGO_TEST: Sample = Sample {
    name: "cargo test", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::Running,
    detail: "cargo test --workspace", // text: sample (a made-up command of the preview)
    tone: RowTone::Plain,
    selected: false,
    notes: 0,
    agent: None,
};
const ZSH: Sample = Sample {
    name: "zsh",
    glyph: StatusGlyph::Idle,
    detail: "~/Projects/terminal/wormhole", // text: sample (a made-up folder of the preview)
    tone: RowTone::Plain,
    selected: false,
    notes: 0,
    agent: None,
};
pub(super) const RATE_LIMITS: Sample = Sample {
    name: "Rate limits", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::Working,
    detail: "Working · 4m", // text: sample (a made-up status of the preview)
    tone: RowTone::Plain,
    selected: true,
    notes: 0,
    agent: Some(AgentId::Claude),
};
pub(super) const FLAKY_LOGIN: Sample = Sample {
    name: "Flaky login test", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::NeedsYou,
    detail: "Needs input · Allow edit?", // text: sample (a made-up status of the preview)
    tone: RowTone::Attention,
    selected: false,
    notes: 0,
    agent: Some(AgentId::Codex),
};
pub(super) const RELEASE_NOTES: Sample = Sample {
    name: "Release notes", // text: sample (a made-up terminal of the preview)
    glyph: StatusGlyph::Done,
    detail: "Finished · 1m", // text: sample (a made-up status of the preview)
    tone: RowTone::Done,
    selected: false,
    notes: 0,
    agent: Some(AgentId::OpenCode),
};

/// The Terminals list of Terminals & sidebar and of Appearance's window.
pub(super) const SAMPLES: &[Card] = &[
    Card { group: None, folder: None, rows: &[SCRATCH] },
    Card {
        group: Some("Release 1.2"),          // text: sample (a made-up group of the preview)
        folder: Some("~/Projects/wormhole"), // text: sample (a made-up folder of the preview)
        rows: &[SIDEBAR_CARDS, FIX_PILE, CARGO_TEST],
    },
    Card { group: None, folder: None, rows: &[ZSH] },
];

/// The list of Coding agents: a group with an agent working, one asking and one finished.
pub(super) const AGENT_SAMPLES: &[Card] = &[Card {
    group: Some("api"),             // text: sample (a made-up project of the preview)
    folder: Some("~/Projects/api"), // text: sample (a made-up folder of the preview)
    rows: &[RATE_LIMITS, FLAKY_LOGIN, RELEASE_NOTES],
}];

impl Sample {
    /// The sidebar's own row for it, with its status tile: `plain` is the face of a terminal without an
    /// agent, `notes` shows its notes' count. `id` tells it from the other rows of the window (the
    /// tile's animations are keyed by it).
    #[track_caller]
    pub(super) fn row(
        &self,
        id: &str,
        ui: UiColors,
        metrics: Metrics,
        plain: &TileFace,
        notes: bool,
        cx: &App,
    ) -> AnyElement {
        self.row_shifting(id, ui, metrics, (plain, notes), None, cx)
    }

    /// [`Self::row`] while its terminal's status changes from `was`'s, as `shift` plays it: the tile's
    /// parts, the card's wash and the second line (the row's nudge is for the caller, which places it).
    #[track_caller]
    pub(super) fn row_shifting(
        &self,
        id: &str,
        ui: UiColors,
        metrics: Metrics,
        (plain, notes): (&TileFace, bool),
        shift: Option<(&Sample, StatusShift)>,
        cx: &App,
    ) -> AnyElement {
        let surface = if self.selected { ui.selected_row } else { ui.panel };
        let tile = style::status_tile_shifting(
            SharedString::from(format!("{id}-tile")),
            ui,
            style::StatusTile {
                face: self.agent.map_or_else(|| plain.clone(), TileFace::Agent),
                glyph: self.glyph,
                selected: self.selected,
                mini: false,
            },
            metrics.tile,
            surface,
            shift.map(|(_, shift)| shift),
            cx,
        );
        let mut row = SessionRow::new(div().into_any_element(), self.name);
        row.selected = self.selected;
        row.tone = self.tone;
        row.tile = Some(tile);
        row.card = true;
        row.metrics = metrics;
        if metrics.two_lines {
            row.detail = Some(self.detail.into());
        }
        // What the list builds: only the Line rolls variant rolls the second line.
        row.detail_roll =
            shift.filter(|(_, shift)| shift.motion == StatusMotion::LineRoll).map(|(was, shift)| {
                DetailRoll {
                    from: was.detail.into(),
                    from_tone: was.tone,
                    from_font: None,
                    shift,
                    to: self.glyph,
                }
            });
        if notes && self.notes > 0 {
            row.trail = Some(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(style::icon(IconName::StickyNote))
                    .child(self.notes.to_string())
                    .into_any_element(),
            );
        }
        let wash = shift.and_then(|(_, shift)| {
            shift.wash(SharedString::from(format!("{id}-wash")), ui, self.glyph, metrics.row_radius)
        });
        style::session_row(SharedString::from(format!("{id}-row")), ui, row).children(wash).into_any_element()
    }
}

/// A group's head of a sample list: what it is called and shows.
pub(super) struct SampleHead {
    /// Tells it from the other heads of the window.
    pub(super) id: SharedString,
    pub(super) name: &'static str,
    pub(super) folder: Option<&'static str>,
    /// The dot's colour.
    pub(super) color: Option<Hsla>,
    pub(super) count: usize,
    pub(super) folded: bool,
    /// Folded: the tiles and the count at this moment of their fade-in (`None`: whole).
    pub(super) summary: Option<style::SummaryFade>,
}

impl SampleHead {
    /// The sidebar's own head, open or folded, doing nothing when it is clicked.
    #[track_caller]
    pub(super) fn draw(self, ui: UiColors, metrics: Metrics) -> AnyElement {
        let Self { id, name, folder, color, count, folded, summary } = self;
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
                pinned: false,
                tiles: Vec::new(),
                count,
                summary_fade: summary,
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
        .into_any_element()
    }
}
