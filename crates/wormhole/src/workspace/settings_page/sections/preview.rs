//! Terminals & sidebar's live preview: the Terminals list drawn with the sidebar's own
//! rows and bands from the current options, beside the option under the pointer. Appearance's
//! stage is `stage`.

use wormhole_core::config::StatusPosition;

use super::*;
use style::{RowTone, SessionRow, StatusGlyph};

/// A preview list is as wide as the sidebar at its least.
const SIDEBAR_W: f32 = 350.0;

/// Terminals & sidebar → the Terminals list as the options make it, and beside it the list
/// with the option under the pointer.
pub(super) fn draw_sidebar_preview(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let now = Look::of(&s);
    let label =
        |text: String| div().text_caption().font_weight(FontWeight::MEDIUM).text_color(ui.muted).child(text);
    let mut panes = div().flex().gap_4().w_full().child(
        div()
            .flex_1()
            .min_w_0()
            .max_w(px(SIDEBAR_W))
            .flex()
            .flex_col()
            .gap_2()
            .child(label(format!("Now · {}", now.density())))
            .child(list("now", now, ui, cx)),
    );
    if let Some(SidebarLook::Compact(compact)) = window.sidebar_preview
        && compact != now.compact
    {
        let previewed = Look { compact, ..now };
        panes = panes.child(
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(SIDEBAR_W))
                .flex()
                .flex_col()
                .gap_2()
                .child(label(format!("Previewed while hovering {}", previewed.density())))
                .child(list("previewed", previewed, ui, cx)),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().p_4().rounded(ui.radius_lg).outlined(ui, ui.border).bg(ui.background).child(panes))
        .child(div().text_caption().text_color(ui.muted).child(
            "Live preview of the Terminals list. Hovering a density previews it next to the current one",
        ))
        .into_any_element()
}

/// The row options a list is drawn with.
#[derive(Clone, Copy)]
struct Look {
    compact: bool,
    dot: bool,
    dot_right: bool,
    logo: bool,
    branch: bool,
    notes: bool,
}

impl Look {
    fn of(s: &Settings) -> Self {
        Self {
            compact: s.compact_sidebar,
            dot: s.sidebar_status_dot,
            dot_right: s.sidebar_status_position == StatusPosition::Right,
            logo: s.sidebar_agent_icon,
            branch: s.sidebar_branch,
            notes: s.sidebar_notes,
        }
    }

    fn density(self) -> &'static str {
        if self.compact { "Compact" } else { "Comfortable" }
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

/// A folder of the sample list.
struct Folder {
    name: &'static str,
    branch: &'static str,
    rows: &'static [Sample],
}

const SAMPLES: &[Folder] = &[
    Folder {
        name: "terminal",
        branch: "ui-unify",
        rows: &[
            Sample {
                name: "sidebar polish",
                glyph: StatusGlyph::Working,
                detail: "Editing sidebar.rs…",
                tone: RowTone::Plain,
                selected: true,
                notes: 2,
            },
            Sample {
                name: "diff perf",
                glyph: StatusGlyph::NeedsYou,
                detail: "Needs you · Approve cargo test?",
                tone: RowTone::Attention,
                selected: false,
                notes: 0,
            },
            Sample {
                name: "zsh",
                glyph: StatusGlyph::Idle,
                detail: "",
                tone: RowTone::Plain,
                selected: false,
                notes: 0,
            },
        ],
    },
    Folder {
        name: "api-server",
        branch: "feature/auth",
        rows: &[
            Sample {
                name: "auth tokens",
                glyph: StatusGlyph::Done,
                detail: "Finished · 12m ago",
                tone: RowTone::Done,
                selected: false,
                notes: 1,
            },
            Sample {
                name: "rate limits",
                glyph: StatusGlyph::Idle,
                detail: "",
                tone: RowTone::Plain,
                selected: false,
                notes: 0,
            },
        ],
    },
];

/// The sample list in `look`, with the sidebar's own bands and rows.
fn list(id: &'static str, look: Look, ui: UiColors, cx: &App) -> AnyElement {
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
    let mut column = div()
        .flex()
        .flex_col()
        .rounded(ui.radius)
        .overflow_hidden()
        .outlined(ui, ui.border)
        .bg(ui.panel)
        .child(header);
    for (fx, folder) in SAMPLES.iter().enumerate() {
        for (rx, sample) in folder.rows.iter().enumerate() {
            let glyph_id = SharedString::from(format!("preview-{id}-glyph-{fx}-{rx}"));
            let idle = sample.glyph == StatusGlyph::Idle;
            let glyph = || style::status_glyph(glyph_id.clone(), ui, sample.glyph, cx);
            // The agent's logo stands in for an idle glyph (state beats identity).
            let lead = match (look.dot && !look.dot_right, look.logo && idle) {
                (_, true) => style::agent_logo(
                    claude.and_then(|agent| agent.icon()),
                    claude.map_or("CC", |agent| agent.monogram),
                    style::GLYPH,
                    ui.muted,
                ),
                (true, false) => glyph(),
                (false, false) => div().into_any_element(),
            };
            let mut row = SessionRow::new(lead, sample.name);
            row.selected = sample.selected;
            row.tone = sample.tone;
            if !look.compact {
                // A quiet row says where it is: its branch under a folder's band, else its path.
                row.detail = Some(match sample.detail {
                    "" => format!("~/Projects/{}", folder.name).into(),
                    detail => detail.into(),
                });
                if look.branch && sample.detail.is_empty() {
                    row.detail = Some(format!("{} · {}", folder.name, folder.branch).into());
                }
            }
            row.trail = match (look.dot && look.dot_right, look.notes && sample.notes > 0) {
                (true, _) => Some(glyph()),
                (false, true) => Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(style::icon(IconName::StickyNote))
                        .child(sample.notes.to_string())
                        .into_any_element(),
                ),
                _ => None,
            };
            column = column.child(style::session_row(
                SharedString::from(format!("preview-{id}-row-{fx}-{rx}")),
                ui,
                row,
            ));
        }
    }
    column.into_any_element()
}
