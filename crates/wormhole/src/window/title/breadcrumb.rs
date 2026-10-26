//! The terminal's title row's crumbs: where the selected
//! terminal is and what it does, as the folder (cut in its middle where it is long, with the
//! hover card of `path_card` and the path menu of `path_menu`), the branch chip, and the terminal's
//! tool and name with its live status. `row` lays them out where the person put them, and the end
//! that holds the name is one keyboard stop.
//!
//! When the row is narrow its parts give way in the order core's `layout::title_row` says; this
//! measures them, asks, and draws what is left.
use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::view::layout::title_parts::TitlePart;
use wormhole_core::view::layout::title_row::{Chip, Face, PathShown, RowWants, fit_row, path_steps};

use crate::settings::{UiColors, settings_in, ui_in};
use crate::style::{self, TextRole};
use crate::terminal_view::{Status, TerminalSummary};
use crate::window::Workspace;
use crate::window::title::path_card::Over;
use crate::window::title::row::{self, has};
use crate::window::title::{Facts, MORE_W, OPEN_IN_W, ROW_GAP, SPLITS_W, Tint, TitlePlan};

impl Workspace {
    /// What the title row says about the active terminal, as of now.
    pub(in crate::window) fn title_facts(&self, cx: &App) -> Facts {
        let session = self.active_session();
        let summary = &session.pane.summary;
        let folder = summary.cwd.clone();
        let shown = folder
            .as_deref()
            .map(|cwd| wormhole_core::view::sidebar::display_path(cwd, crate::style::home_dir()));
        // The branch only while git's answer is about the folder the terminal is in now.
        let status = session.git.status.as_ref().filter(|_| session.git.cwd == summary.cwd);
        let branch = status.map(|status| status.branch.clone());
        let parent = status.and_then(|status| status.superproject.as_ref()).map(|parent| parent.phrase());
        Facts {
            id: session.id,
            folder,
            shown,
            status: summary.status,
            agent: summary.agent,
            told: summary.source.told(),
            label: session.pane.label(),
            words: crumb_words(summary).filter(|_| self.crumb_words_shown(cx)),
            branch,
            parent,
            extras: self.extra_facts(cx),
            row: settings_in(&self.profile, cx).title_row().into_owned(),
        }
    }

    /// Measures the row and asks core what it keeps: the whole of `fit_row`'s input.
    pub(in crate::window) fn plan_title(
        &self,
        tint: Option<Tint>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> TitlePlan {
        let facts = self.title_facts(cx);
        let ends = row::ends(&facts);
        let drawn = |part| ends.0.parts.contains(&part) || ends.1.parts.contains(&part);
        let measure = Measure::new(window, cx);
        let steps = facts
            .shown
            .as_deref()
            .filter(|_| drawn(TitlePart::Folder))
            .map(|shown| {
                path_steps(shown, PATH_LAST_MAX, |face, text| match face {
                    Face::Parents => measure.label(text, FontWeight::NORMAL),
                    Face::Last => measure.label(text, FontWeight::MEDIUM),
                })
            })
            .unwrap_or_default();
        // Open in… (which `has` keeps for a terminal with a folder) and the splits, each where it is
        // arranged; folded they are one `⋯`.
        let buttons = [(TitlePart::OpenIn, OPEN_IN_W), (TitlePart::Splits, SPLITS_W)]
            .into_iter()
            .filter(|(part, _)| drawn(*part))
            .map(|(_, width)| width)
            .collect::<Vec<_>>();
        let (group, folded) = match buttons.is_empty() {
            true => (0.0, 0.0),
            false => (ROW_GAP + buttons.iter().sum::<f32>(), ROW_GAP + MORE_W),
        };
        let (extras, ports_narrower) = Self::extras_wanted(&facts.extras, &measure);
        let terminal = has(&facts, TitlePart::Terminal);
        let wants = RowWants {
            path: steps.iter().map(|step| CRUMB_GAP + step.width).collect(),
            // The status words and, in a submodule, whose it is: they give way together.
            words: [
                facts.words.as_deref().filter(|_| drawn(TitlePart::TerminalStatus)),
                facts.parent.as_deref().filter(|_| drawn(TitlePart::Branch)),
            ]
            .into_iter()
            .flatten()
            .map(|text| CRUMB_GAP + measure.caption(text))
            .sum(),
            chip: facts.branch.as_ref().filter(|_| drawn(TitlePart::Branch)).map(|branch| {
                let text = branch.as_deref().unwrap_or("detached");
                Chip {
                    natural: (measure.caption_medium(text) + CHIP_CHROME).min(style::BRANCH_CHIP_MAX),
                    icon: CHIP_ICON_W,
                }
            }),
            extras,
            ports_narrower,
            group,
            folded,
            name: match terminal {
                true => measure.label(&facts.label, FontWeight::MEDIUM).min(style::TITLE_TAB_MAX_W),
                false => 0.0,
            },
            name_min: if terminal { CRUMB_TEXT_MIN } else { 0.0 },
            fixed: row::fixed_width(&ends),
        };
        let fit = fit_row(self.title_row_width(window), &wants);
        TitlePlan { app: crate::open_in::OpenIn::resolved(cx), facts, ends, steps, fit, tint }
    }

    /// The folder: it shows in Finder, has its hover card and the Finder-style path menu.
    pub(in crate::window) fn render_folder_crumb(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let ui = ui_in(&self.profile, cx);
        let folder = plan.facts.folder.clone()?;
        // Under a tint the folder's icon and its parents step up to the main text color, so they
        // keep their contrast on it.
        let soft = if plan.tint.is_some() { ui.text } else { ui.muted };
        let text = match plan.fit.path {
            PathShown::Step(ix) => Some(&plan.steps[ix].text),
            PathShown::Icon | PathShown::None => None,
        };
        Some(
            div()
                .id("crumb-folder")
                .debug_selector(|| "crumb-folder".to_string())
                // Cut in the middle, the folder is still read whole.
                .role(Role::Button)
                .aria_label(format!(
                    "Open folder {} in Finder",
                    plan.facts.shown.as_deref().unwrap_or_default()
                ))
                // The card and the menu open under the folder, which the probe measures.
                .relative()
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .text_label()
                .font_weight(FontWeight::NORMAL)
                .text_color(soft)
                .cursor_pointer()
                .hover(|d| d.text_color(ui.text))
                .child(self.path_anchor.probe())
                .child(div().flex_none().child(style::icon(IconName::Folder)))
                .children(text.map(|text| {
                    div().flex().flex_none().child(div().flex_none().child(text.parents.clone())).child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(ui.text)
                            .child(text.last.clone()),
                    )
                }))
                // The folder itself in Finder, not its parent with it selected.
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.path_card.reset(cx);
                    cx.open_with_system(&folder);
                }))
                .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                    this.hover_path(Over::Path, *hovered, window, cx)
                }))
                // The Finder-style path menu.
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.path_card.reset(cx);
                        this.open_path_menu(window, cx);
                    }),
                )
                .into_any_element(),
        )
    }

    /// The branch chip, which opens the compare picker, and in a submodule whose it is ("in
    /// terminal") after it.
    pub(in crate::window) fn render_branch_crumb(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> Option<Vec<AnyElement>> {
        let ui = ui_in(&self.profile, cx);
        let (branch, width) = plan.facts.branch.as_ref().zip(plan.fit.chip)?;
        let (color, fill, name) = match branch {
            Some(branch) => (ui.branch, ui.branch_bg, branch.as_str()),
            None => (ui.warning_text, ui.banner_warning, "detached"),
        };
        // Down to its glyph, which the chip keeps.
        let text = width >= CHIP_CHROME + CRUMB_TEXT_MIN / 2.0;
        let chip = div()
            .id("crumb-branch")
            .flex_none()
            .w(px(width))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(CHIP_H))
                    .w_full()
                    .gap_1()
                    .pl_1()
                    .when(text, |d| d.pr_2())
                    .when(!text, |d| d.pr_1())
                    .overflow_hidden()
                    .rounded(ui.radius_chip)
                    .bg(fill)
                    .text_color(color)
                    .text_caption()
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(IconName::GitBranch).xsmall())
                    .when(text, |d| d.child(div().min_w_0().truncate().child(name.to_string()))),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.pick_branch(window, cx);
            }));
        let whose = plan.facts.parent.clone().filter(|_| plan.fit.words).map(|phrase| {
            let tip = SharedString::from(format!("A submodule: {phrase}"));
            div()
                .id("crumb-superproject")
                .flex_none()
                .ml_2()
                .whitespace_nowrap()
                .text_caption()
                .text_color(ui.muted)
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .child(phrase)
                .into_any_element()
        });
        Some(std::iter::once(chip.into_any_element()).chain(whose).collect())
    }

    /// The terminal: a small status tile (the tool's own icon, its live status round it), then its
    /// name, cut at its end where the row is short of room.
    pub(in crate::window) fn render_terminal_crumb(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let Facts { id, status, agent, told, label, .. } = &plan.facts;
        let tile = terminal_tile(("terminal-tile", *id as u64), *agent, *told, *status, ui, cx);
        // Room round the tile: the box clips a long name sideways, and GPUI's clip is never one axis
        // only, so a box hugging the tile would cut its shadow off at the left and the bottom. The
        // room on the left is taken back by the margin, so nothing moves.
        div()
            .id("crumb-process")
            .debug_selector(|| "crumb-process".to_string())
            .py(px(SHADOW_ROOM))
            .pl(px(SHADOW_ROOM))
            .ml(px(-SHADOW_ROOM))
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .overflow_hidden()
            .child(tile)
            .child(
                div()
                    .min_w_0()
                    .max_w(px(plan.fit.name))
                    .truncate()
                    .text_label()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ui.text)
                    .child(label.clone()),
            )
            .into_any_element()
    }

    /// How the terminal is doing ("working · 4m"), in the colour of its state.
    pub(in crate::window) fn render_terminal_words(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let ui = ui_in(&self.profile, cx);
        let words = plan.facts.words.clone().filter(|_| plan.fit.words)?;
        let color = match plan.facts.status {
            Status::Attention => ui.attention_text,
            Status::Failed => ui.danger_text,
            _ => ui.muted,
        };
        Some(
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_caption()
                .text_color(color)
                .child(words)
                .into_any_element(),
        )
    }

    /// Whether the row says how the terminal is doing: it shows the part, and the status bar, which
    /// has the state at its end, isn't shown (the row doesn't repeat it).
    fn crumb_words_shown(&self, cx: &App) -> bool {
        let settings = settings_in(&self.profile, cx);
        !settings.status_bar && settings.title_shows(TitlePart::TerminalStatus)
    }
}

/// The breadcrumb's spacing (`gap_2`, `px_2`), and the least of a cut text worth showing (about
/// three letters and the ellipsis).
pub(in crate::window) const CRUMB_GAP: f32 = 8.0;
/// The room the crumb's clip leaves round the terminal's tile, for its shadow.
const SHADOW_ROOM: f32 = 4.0;
pub(in crate::window) const CRUMB_PAD: f32 = 8.0;
const CRUMB_TEXT_MIN: f32 = 24.0;
/// A branch chip's glyph, gap and padding round its text, and its width when it is the glyph alone.
const CHIP_CHROME: f32 = 28.0;
const CHIP_ICON_W: f32 = 20.0;
/// A branch chip's height: the chip height of a text line (`style::branch_chip`).
const CHIP_H: f32 = 20.0;
/// The last folder of the path is cut in its own middle when it alone is wider than this.
const PATH_LAST_MAX: f32 = 240.0;

/// How wide the breadcrumb's texts are in the interface font, in their type roles.
pub(in crate::window) struct Measure<'a> {
    window: &'a Window,
    family: SharedString,
}

impl<'a> Measure<'a> {
    fn new(window: &'a Window, cx: &App) -> Self {
        Self { window, family: cx.theme().font_family.clone() }
    }

    fn width(&self, text: &str, size_rems: f32, weight: FontWeight) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let run = TextRun {
            len: text.len(),
            font: Font { weight, ..font(self.family.clone()) },
            ..Default::default()
        };
        let size = rems(size_rems).to_pixels(self.window.rem_size());
        f32::from(self.window.text_system().layout_line(text, size, &[run], None).width).ceil()
    }

    /// `text_label` (13).
    fn label(&self, text: &str, weight: FontWeight) -> f32 {
        self.width(text, 0.8125, weight)
    }

    /// `text_caption` (12).
    fn caption(&self, text: &str) -> f32 {
        self.width(text, 0.75, FontWeight::NORMAL)
    }

    pub(in crate::window) fn caption_medium(&self, text: &str) -> f32 {
        self.width(text, 0.75, FontWeight::MEDIUM)
    }
}

impl Workspace {
    /// On the tick: the breadcrumb's "working · 4m" moves on by itself once a minute, with nothing
    /// else to redraw the window (an agent thinking quietly); redraw when it would read
    /// differently.
    pub(in crate::window) fn crumb_tick(&mut self, cx: &mut Context<Self>) {
        // The status bar's tick redraws its own copy, and a row without the words has nothing to redraw.
        if !self.crumb_words_shown(cx) {
            return;
        }
        let now = crumb_words(&self.active_session().pane.summary);
        // Taken as drawn: if nothing draws it (a diff is over the title row), it asks once.
        if std::mem::replace(&mut self.regions.drawn.crumb, now.clone()) != now {
            cx.notify();
        }
    }

    /// The words the title row is drawn with, for the tick to compare the next ones with.
    pub(in crate::window) fn crumb_drawn(&self, cx: &App) -> Option<String> {
        crumb_words(&self.active_session().pane.summary).filter(|_| self.crumb_words_shown(cx))
    }
}

/// What the breadcrumb says after a terminal's name: how it is doing, working time in whole
/// minutes ("working · <1m", "working · 4m"), or nothing while it is idle.
pub(in crate::window) fn crumb_words(summary: &TerminalSummary) -> Option<String> {
    let took = || wormhole_core::view::layout::status_bar::elapsed(summary.status_since.elapsed());
    match summary.status {
        Status::Busy => Some(format!("working · {}", took())),
        Status::Attention => Some("needs input".to_string()),
        Status::Done => Some(format!("done · {}", took())),
        // The exit code when the shell gave one; else how long ago, as for done.
        Status::Failed => Some(match &summary.failure {
            Some(failure) => format!("failed · exit {}", failure.code),
            None => format!("failed · {}", took()),
        }),
        Status::Exited => Some("exited".to_string()),
        Status::Idle => None,
    }
}

/// What [`crumb_words`] is made of: the status, since when, and a failure's exit code. The words move with
/// the clock and the tick owns them (`crumb_tick`, `refresh_status_bar`), up to a tick late, so the check on
/// the chrome compares these instead.
#[cfg(any(test, debug_assertions))]
pub(in crate::window) fn crumb_source(
    summary: &TerminalSummary,
) -> (Status, std::time::Instant, Option<i32>) {
    (summary.status, summary.status_since, summary.failure.as_ref().map(|failure| failure.code))
}

/// The live status next to the tool's icon, the sidebar row's own glyph and motion (so one terminal
/// never shows two states): the working spinner, needs-you halo, finished check or idle ring of
/// an agent or a program that reports its status (`told`); for a plain terminal only a running
/// command (the spinner), a ring bell or a command that failed. Nothing for an exited one, whose
/// words say so.
fn indicator_glyph(told: bool, status: Status) -> Option<style::StatusGlyph> {
    use style::StatusGlyph as G;
    match (told, status) {
        (_, Status::Exited) => None,
        (true, Status::Busy) => Some(G::Working),
        (true, Status::Attention) => Some(G::NeedsYou),
        (true, Status::Done) => Some(G::Done),
        (_, Status::Failed) => Some(G::Failed),
        (true, Status::Idle) => Some(G::Idle),
        (false, Status::Busy) => Some(G::Running),
        (false, Status::Attention) => Some(G::Bell),
        (false, Status::Done | Status::Idle) => None,
    }
}

/// A terminal's status tile in a 28 pt row (the title row, a split's pane headers): the tool's own
/// icon with the live status its sidebar row shows, so one terminal looks the same everywhere.
pub(in crate::window) fn terminal_tile(
    id: impl Into<ElementId>,
    agent: Option<AgentId>,
    told: bool,
    status: Status,
    ui: UiColors,
    cx: &App,
) -> AnyElement {
    let face = match agent {
        Some(agent) => style::TileFace::Agent(agent),
        None => style::TileFace::Terminal,
    };
    let glyph = indicator_glyph(told, status).unwrap_or(style::StatusGlyph::Idle);
    style::status_tile_at(
        id,
        ui,
        style::StatusTile { face, glyph, selected: false, mini: true },
        style::TILE_TITLE,
        cx,
    )
}

#[cfg(test)]
mod tests {
    use super::indicator_glyph;
    use crate::style::StatusGlyph;
    use crate::terminal_view::Status;

    /// The sidebar row's glyphs for an agent; a plain terminal shows only what it is doing.
    #[test]
    fn the_indicator_is_the_sidebar_rows_glyph() {
        assert_eq!(indicator_glyph(true, Status::Busy), Some(StatusGlyph::Working));
        assert_eq!(indicator_glyph(true, Status::Attention), Some(StatusGlyph::NeedsYou));
        assert_eq!(indicator_glyph(true, Status::Done), Some(StatusGlyph::Done));
        assert_eq!(indicator_glyph(true, Status::Failed), Some(StatusGlyph::Failed));
        assert_eq!(indicator_glyph(false, Status::Failed), Some(StatusGlyph::Failed));
        assert_eq!(indicator_glyph(true, Status::Idle), Some(StatusGlyph::Idle));
        assert_eq!(indicator_glyph(false, Status::Busy), Some(StatusGlyph::Running));
        assert_eq!(indicator_glyph(false, Status::Attention), Some(StatusGlyph::Bell));
        assert_eq!(indicator_glyph(false, Status::Idle), None);
        assert_eq!(indicator_glyph(false, Status::Done), None);
        assert_eq!(indicator_glyph(true, Status::Exited), None);
        assert_eq!(indicator_glyph(false, Status::Exited), None);
    }
}
