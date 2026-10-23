//! The terminal's title row content: where the selected
//! terminal is and what it does, as the folder (cut in its middle where it is long, with the
//! hover card of `path_card` and the path menu of `path_menu`), the branch chip, and the terminal's tool and name with its live
//! status. It is one keyboard stop. Open in… and the splits sit at the right end of the row
//! (`window::title`).
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
use wormhole_core::view::layout::title_row::{Chip, Face, PathShown, RowWants, fit_row, path_steps};

use crate::settings::{UiColors, settings_in, ui_in};
use crate::style::{self, TextRole};
use crate::terminal_view::{Status, TerminalView};
use crate::window::Workspace;
use crate::window::title::path_card::Over;
use crate::window::title::{Facts, MORE_W, ROW_GAP, SPLITS_W, Tint, TitlePlan};
use wormhole_core::config::TabId;

impl Workspace {
    /// The terminal's title row content: its breadcrumb, the row's keyboard stop with its ring.
    pub(in crate::window) fn render_title_strip(
        &self,
        plan: &TitlePlan,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        // The strip is one keyboard stop; its ring round the breadcrumb.
        let ring = self.ring(wormhole_core::view::focus::Area::Tabs, window);
        let crumb = div()
            .relative()
            .min_w_0()
            .flex()
            .child(self.render_breadcrumb(plan, cx))
            .when(ring, |d| d.child(style::focus_ring(ui, style::RingPlacement::Outside, ui.radius)));
        let strip = div()
            .id("title-strip")
            // Clicks here are for the breadcrumb, not for dragging the window.
            .occlude()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .max_w_full()
            .overflow_hidden()
            .child(crumb);
        self.area_container(wormhole_core::view::focus::Area::Tabs, strip, cx).into_any_element()
    }

    /// What the title row says about the active terminal, as of now.
    fn title_facts(&self, cx: &App) -> Facts {
        let session = self.active_session();
        let terminal = session.terminal().read(cx);
        let folder = terminal.cwd.clone();
        let shown = folder
            .as_deref()
            .map(|cwd| wormhole_core::view::sidebar::display_path(cwd, crate::window::session::home_dir()));
        // The branch only while git's answer is about the folder the terminal is in now.
        let branch = session
            .git
            .status
            .as_ref()
            .filter(|_| session.git.cwd == terminal.cwd)
            .map(|status| status.branch.clone());
        let parent = session
            .git
            .status
            .as_ref()
            .filter(|_| session.git.cwd == terminal.cwd)
            .and_then(|status| status.superproject.as_ref())
            .map(|parent| parent.phrase());
        Facts {
            id: session.id,
            folder,
            shown,
            status: terminal.status(),
            agent: terminal.agent,
            told: terminal.source().told(),
            label: session.pane.label(cx),
            // Shown, the status bar has the state at its end; the title row doesn't repeat it.
            words: crumb_words(terminal).filter(|_| !settings_in(&self.profile, cx).status_bar),
            branch,
            parent,
            extras: self.extra_facts(cx),
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
        self.crumb_drawn.replace(facts.words.clone());
        let measure = Measure::new(window, cx);
        let steps = facts
            .shown
            .as_deref()
            .map(|shown| {
                path_steps(shown, PATH_LAST_MAX, |face, text| match face {
                    Face::Parents => measure.label(text, FontWeight::NORMAL),
                    Face::Last => measure.label(text, FontWeight::MEDIUM),
                })
            })
            .unwrap_or_default();
        // The split buttons are always there; Open in… and its chevron only for a terminal that
        // has a folder to open.
        let (group, folded) = match facts.folder {
            Some(_) => (ROW_GAP + crate::window::title::OPEN_IN_W + SPLITS_W, ROW_GAP + MORE_W),
            None => (ROW_GAP + SPLITS_W, ROW_GAP + MORE_W),
        };
        // A crumb before the terminal brings a gap, the separator and a gap.
        let before = 2.0 * CRUMB_GAP + style::GLYPH;
        let (extras, ports_narrower) = Self::extras_wanted(&facts.extras, &measure);
        let wants = RowWants {
            path: steps.iter().map(|step| CRUMB_GAP + step.width).collect(),
            // The status words and, in a submodule, whose it is: they give way together.
            words: [facts.words.as_deref(), facts.parent.as_deref()]
                .into_iter()
                .flatten()
                .map(|text| CRUMB_GAP + measure.caption(text))
                .sum(),
            chip: facts.branch.as_ref().map(|branch| {
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
            name: measure.label(&facts.label, FontWeight::MEDIUM).min(style::TITLE_TAB_MAX_W),
            name_min: CRUMB_TEXT_MIN,
            // The padding, the folder's icon and its separator, the branch's separator, and the
            // terminal's status tile and the gap before its name.
            fixed: 2.0 * CRUMB_PAD
                + facts.folder.as_ref().map_or(0.0, |_| style::GLYPH + before)
                + facts.branch.as_ref().map_or(0.0, |_| before)
                + style::TILE_TITLE
                + CRUMB_GAP,
        };
        let fit = fit_row(self.title_row_width(window), &wants);
        TitlePlan { app: crate::window::open_in::OpenIn::resolved(cx), facts, steps, fit, tint }
    }

    /// The selected terminal's breadcrumb: folder › branch › the terminal's tool and name.
    /// The folder shows in Finder, has its hover card and a menu; the branch opens the compare
    /// picker; the name is what runs, with its live status next to the tool's icon.
    fn render_breadcrumb(&self, plan: &TitlePlan, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let Facts { id, folder, status, agent, told, label, words, branch, parent, .. } = &plan.facts;
        let fit = &plan.fit;
        // Under a tint the folder's icon and its parents step up to the main text color, so they
        // keep their contrast on it.
        let soft = if plan.tint.is_some() { ui.text } else { ui.muted };

        let separator = || div().flex_none().text_color(ui.faint).child(style::icon(IconName::ChevronRight));
        let path = folder.clone().map(|folder| {
            let text = match fit.path {
                PathShown::Step(ix) => Some(&plan.steps[ix].text),
                PathShown::Icon | PathShown::None => None,
            };
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
        });
        let branch = branch.as_ref().zip(fit.chip).map(|(branch, width)| {
            let (color, fill, name) = match branch {
                Some(branch) => (ui.branch, ui.branch_bg, branch.as_str()),
                None => (ui.warning_text, ui.banner_warning, "detached"),
            };
            // Down to its glyph, which the chip keeps.
            let text = width >= CHIP_CHROME + CRUMB_TEXT_MIN / 2.0;
            div()
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
                    this.pick_branch_in(TabId::Changes, window, cx);
                }))
        });

        // The terminal: a small status tile (the tool's own icon, its live status round it), then
        // the name and how it is doing.
        let tile = terminal_tile(("terminal-tile", *id as u64), *agent, *told, *status, ui, cx);
        let attention = *status == Status::Attention;
        let failed = *status == Status::Failed;
        // Room round the tile: the box clips a long name sideways, and GPUI's clip is never one axis
        // only, so a box hugging the tile would cut its shadow off at the left and the bottom. The
        // room on the left is taken back by the margin, so nothing moves.
        let process = div()
            .id("crumb-process")
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
                    .max_w(px(fit.name))
                    .truncate()
                    .text_label()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ui.text)
                    .child(label.clone()),
            )
            .children(words.clone().filter(|_| fit.words).map(|words| {
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .text_caption()
                    .text_color(match () {
                        _ if attention => ui.attention_text,
                        _ if failed => ui.danger_text,
                        _ => ui.muted,
                    })
                    .child(words)
            }));

        // The branch's status (a dot, ↑2 ↓1) follows its chip, before the separator.
        let status = branch.is_some().then(|| self.render_branch_status(plan, cx)).flatten();
        // A terminal in a submodule says whose it is, after the branch: "in terminal".
        let whose = parent.clone().filter(|_| fit.words).map(|phrase| {
            let tip = SharedString::from(format!("A submodule: {phrase}"));
            div()
                .id("crumb-superproject")
                .flex_none()
                .whitespace_nowrap()
                .text_caption()
                .text_color(ui.muted)
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .child(phrase)
                .into_any_element()
        });
        let mut crumbs: Vec<AnyElement> = Vec::new();
        for part in [
            path.map(|path| vec![path.into_any_element()]),
            branch.map(|chip| std::iter::once(chip.into_any_element()).chain(whose).chain(status).collect()),
        ]
        .into_iter()
        .flatten()
        {
            crumbs.extend(part);
            crumbs.push(separator().into_any_element());
        }
        crumbs.push(process.into_any_element());
        div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .overflow_hidden()
            .px_2()
            .children(crumbs)
            .into_any_element()
    }
}

/// The breadcrumb's spacing (`gap_2`, `px_2`), and the least of a cut text worth showing (about
/// three letters and the ellipsis).
pub(in crate::window) const CRUMB_GAP: f32 = 8.0;
/// The room the crumb's clip leaves round the terminal's tile, for its shadow.
const SHADOW_ROOM: f32 = 4.0;
const CRUMB_PAD: f32 = 8.0;
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
    pub(in crate::window) fn crumb_tick(&self, cx: &mut Context<Self>) {
        // The status bar's tick redraws its own copy.
        if settings_in(&self.profile, cx).status_bar {
            return;
        }
        let now = crumb_words(self.active_session().terminal().read(cx));
        // Taken as drawn: if nothing draws it (a diff is over the title row), it asks once.
        if self.crumb_drawn.replace(now.clone()) != now {
            cx.notify();
        }
    }
}

/// What the breadcrumb says after a terminal's name: how it is doing, working time in whole
/// minutes ("working · <1m", "working · 4m"), or nothing while it is idle.
pub(in crate::window) fn crumb_words(terminal: &TerminalView) -> Option<String> {
    let took = || wormhole_core::view::layout::status_bar::elapsed(terminal.status_since().elapsed());
    match terminal.status() {
        Status::Busy => Some(format!("working · {}", took())),
        Status::Attention => Some("needs you".to_string()),
        Status::Done => Some(format!("done · {}", took())),
        // The exit code when the shell gave one; else how long ago, as for done.
        Status::Failed => Some(match terminal.failure() {
            Some(failure) => format!("failed · exit {}", failure.code),
            None => format!("failed · {}", took()),
        }),
        Status::Exited => Some("exited".to_string()),
        Status::Idle => None,
    }
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
