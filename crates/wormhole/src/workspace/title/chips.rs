//! The title row's extras as chips: how wide each is, which core's
//! `layout::title_row` needs to decide what fits, and drawing the ones it keeps. What each says
//! comes from `title_extras`; the pieces are `style::extras`.
//!
//! The branch's status follows the branch chip in the breadcrumb. The agent's model and context,
//! the ports and the pull request sit at the right end of the row, in front of Open in…, in the
//! stretch that is otherwise the window's title bar. Every chip is its own button for the pointer
//! and for VoiceOver.

use gpui_kit::assets::IconName;
use gpui_kit::component::Selectable;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::*;
use wormhole_core::agents::usage::Gauge;
use wormhole_core::git::pr::{Checks, PrState, PullRequest};
use wormhole_core::view::layout::title_row::{Extra, Extras};
use wormhole_core::view::title_extras::{more_ports_words, port_label, port_url, port_words, ports_shown};

use crate::select;
use crate::settings::{UiColors, ui_in};
use crate::style;
use crate::workspace::breadcrumb::{CRUMB_GAP, Measure};
use crate::workspace::title_extras::ExtraFacts;
use crate::workspace::title_row::TitlePlan;
use crate::workspace::{RightTab, Workspace};

/// The gap in front of each chip at the right end of the row (`ml_2`).
const CHIP_GAP: f32 = 8.0;
/// The menu under "+n", wide enough for `localhost:65535`.
const MORE_PORTS_W: f32 = 168.0;

impl Workspace {
    /// How wide each extra is with the gap in front of it (what `fit_row` weighs), and how wide
    /// the ports are with a chip fewer each time (`RowWants::ports_narrower`).
    pub(in crate::workspace) fn extras_wanted(facts: &ExtraFacts, measure: &Measure) -> (Extras, Vec<f32>) {
        let text = |text: &str| measure.caption_medium(text);
        let chip = |icon: bool, text: f32| {
            2.0 * style::EXTRA_PAD + text + if icon { style::EXTRA_ICON + style::EXTRA_GAP } else { 0.0 }
        };
        let mut extras = Extras::default();
        if let Some(status) = &facts.branch {
            let counts = [status.ahead, status.behind].into_iter().filter(|n| *n > 0);
            let parts: Vec<f32> = status
                .dirty
                .then_some(style::STATUS_DOT)
                .into_iter()
                .chain(counts.map(|n| style::EXTRA_ICON + text(&n.to_string())))
                .collect();
            let gaps = parts.len().saturating_sub(1) as f32 * style::EXTRA_GAP;
            extras = extras.with(Extra::BranchStatus, Some(CRUMB_GAP + parts.iter().sum::<f32>() + gaps));
        }
        if let Some(gauge) = &facts.gauge {
            // The name, then the meter and its share: each a part, with a gap between parts.
            let mut parts: Vec<f32> = Vec::new();
            if let Some(name) = &gauge.model {
                parts.push(text(name).min(style::GAUGE_MODEL_MAX));
            }
            if let Some(percent) = gauge.percent() {
                parts.extend([style::METER_W, text(&format!("{percent}%"))]);
            }
            let gaps = parts.len().saturating_sub(1) as f32 * style::EXTRA_GAP;
            let width = 2.0 * style::EXTRA_PAD + parts.iter().sum::<f32>() + gaps;
            extras = extras.with(Extra::Gauge, Some(CHIP_GAP + width));
        }
        let mut narrower = Vec::new();
        if !facts.ports.is_empty() {
            // The ports as `chips` chips of their own and a "+n" for the others.
            let ports = |chips: usize| -> f32 {
                let own: f32 = facts.ports[..chips]
                    .iter()
                    .map(|port| CHIP_GAP + chip(true, text(&port_label(*port))))
                    .sum();
                let rest = facts.ports.len() - chips;
                own + if rest > 0 { CHIP_GAP + chip(false, text(&format!("+{rest}"))) } else { 0.0 }
            };
            let (shown, _) = ports_shown(&facts.ports);
            extras = extras.with(Extra::Ports, Some(ports(shown.len())));
            // A chip fewer at a time, down to the one.
            narrower = (1..shown.len()).rev().map(ports).collect();
        }
        if let Some(pr) = &facts.pull_request {
            let checks = if pr.open_checks().is_some() { style::EXTRA_GAP + style::EXTRA_ICON } else { 0.0 };
            extras = extras.with(Extra::PullRequest, Some(CHIP_GAP + chip(true, text(&pr.label())) + checks));
        }
        (extras, narrower)
    }

    /// The branch's status after its chip: a dot while the tree has uncommitted changes, and the
    /// commits ahead and behind. A click shows the Changes tab, where what changed is.
    pub(in crate::workspace) fn render_branch_status(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let status =
            plan.facts.extras.branch.as_ref().filter(|_| plan.fit.extras.shows(Extra::BranchStatus))?;
        let ui = ui_in(&self.profile, cx);
        let words = SharedString::from(status.words());
        let tip = words.clone();
        Some(
            style::branch_status(ui, status.dirty, status.ahead, status.behind)
                .id("crumb-branch-status")
                .role(Role::Button)
                .aria_label(words)
                .cursor_pointer()
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .on_click(cx.listener(|this, _, _, cx| this.show_tab(RightTab::Changes, cx)))
                .into_any_element(),
        )
    }

    /// The model and context, the ports and the pull request, the ones the row had room for.
    pub(in crate::workspace) fn render_title_extras(
        &self,
        plan: &TitlePlan,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let ui = ui_in(&self.profile, cx);
        let (facts, extras) = (&plan.facts.extras, &plan.fit.extras);
        let mut chips: Vec<AnyElement> = Vec::new();
        if let Some(gauge) = facts.gauge.as_ref().filter(|_| extras.shows(Extra::Gauge)) {
            chips.push(gauge_chip(gauge, ui));
        }
        if extras.shows(Extra::Ports) {
            chips.extend(self.port_chips(&facts.ports, plan.fit.ports_step, ui, cx));
        }
        if let Some(pr) = facts.pull_request.as_ref().filter(|_| extras.shows(Extra::PullRequest)) {
            chips.push(pull_request_chip(pr, ui));
        }
        (!chips.is_empty()).then(|| {
            // Its stretch of the row is no longer the window's title bar: clicks stop here.
            div()
                .id("title-extras")
                .occlude()
                .flex_none()
                .flex()
                .items_center()
                .children(chips)
                .into_any_element()
        })
    }

    /// A chip for each of the first ports, as many as the row has room for (`step` chips fewer
    /// than usual, at least one), and a "+n" menu for the others.
    fn port_chips(
        &self,
        ports: &[u16],
        step: usize,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (shown, _) = ports_shown(ports);
        let shown = &shown[..shown.len().saturating_sub(step).max(1).min(shown.len())];
        let more = ports.len() - shown.len();
        let mut chips: Vec<AnyElement> = shown
            .iter()
            .map(|&port| {
                let words = SharedString::from(port_words(port));
                let tip = words.clone();
                div()
                    .id(("title-port", u64::from(port)))
                    .ml_2()
                    .role(Role::Button)
                    .aria_label(words)
                    .cursor_pointer()
                    .child(style::extra_chip(ui, true, Some((IconName::Globe, ui.text)), port_label(port)))
                    .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                    .on_click(move |_, _, cx| cx.open_url(&port_url(port)))
                    .into_any_element()
            })
            .collect();
        if more > 0 {
            let rest: Vec<u16> = ports[shown.len()..].to_vec();
            chips.push(self.more_ports_chip(&rest, ui, cx));
        }
        chips
    }

    /// The ports without a chip, in a menu under "+n": each opens in the browser.
    fn more_ports_chip(&self, rest: &[u16], ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.weak_entity();
        let on_open = this.clone();
        let ports = rest.to_vec();
        let trigger = MoreTrigger {
            ui,
            text: format!("+{}", rest.len()).into(),
            words: more_ports_words(rest).into(),
            open: false,
        };
        let popover = Popover::new("title-more-ports-menu")
            .anchor(Anchor::TopRight)
            .offset(px(4.0))
            .open(self.ports_menu)
            .on_open_change(move |open, _, cx| {
                on_open
                    .update(cx, |this, cx| {
                        this.ports_menu = *open;
                        cx.notify();
                    })
                    .ok();
            })
            .trigger(trigger);
        select::menu_surface(popover, ui, px(MORE_PORTS_W), select::menu_bg(ui, cx))
            .p_1()
            .content(move |_, _, _| {
                div().flex().flex_col().children(ports.iter().map(|&port| {
                    let this = this.clone();
                    style::row(("title-more-port", u64::from(port)), ui, false)
                        .child(div().flex_1().child(port_label(port)))
                        .on_click(move |_, _, cx| {
                            cx.open_url(&port_url(port));
                            this.update(cx, |this, cx| {
                                this.ports_menu = false;
                                cx.notify();
                            })
                            .ok();
                        })
                }))
            })
            .into_any_element()
    }
}

/// The agent's model and how full its context window is.
fn gauge_chip(gauge: &Gauge, ui: UiColors) -> AnyElement {
    let context = gauge.percent().zip(gauge.context).map(|(percent, used)| style::ContextUse {
        share: used / 100.0,
        warn: gauge.warns(),
        percent: format!("{percent}%").into(),
    });
    let words = SharedString::from(gauge.words());
    let tip = words.clone();
    div()
        .id("title-gauge")
        .ml_2()
        .role(Role::Group)
        .aria_label(words)
        .child(style::agent_gauge(ui, gauge.model.clone().map(SharedString::from), context))
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .into_any_element()
}

/// How a pull request is drawn, in the title row and on its folder's band: its icon in the colour
/// of how it stands (a draft's muted), and while it is open its checks: a green check once they
/// pass, a red cross when one fails, a dashed circle while they run.
pub(in crate::workspace) fn pr_look(pr: &PullRequest, ui: UiColors) -> style::PrLook {
    let (icon, color) = match pr.state {
        PrState::Open if pr.draft => (IconName::GitPullRequestDraft, ui.muted),
        PrState::Open => (IconName::GitPullRequest, ui.success_text),
        PrState::Merged => (IconName::GitPullRequest, ui.branch),
        PrState::Closed => (IconName::GitPullRequest, ui.danger_text),
    };
    let checks = pr.open_checks().map(|checks| match checks {
        Checks::Passing => (IconName::CircleCheck, ui.success_text),
        Checks::Failing => (IconName::CircleX, ui.danger_text),
        Checks::Pending => (IconName::CircleDashed, ui.muted),
    });
    style::PrLook { icon, color, label: pr.label().into(), checks }
}

/// The branch's pull request: its number, in the colour of how it stands, and its checks. A click
/// opens it.
fn pull_request_chip(pr: &PullRequest, ui: UiColors) -> AnyElement {
    let (url, words) = (pr.url.clone(), SharedString::from(pr.words()));
    let tip = words.clone();
    div()
        .id("title-pull-request")
        .ml_2()
        .role(Role::Link)
        .aria_label(words)
        .cursor_pointer()
        .child(style::pull_request_chip(ui, pr_look(pr, ui)))
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .on_click(move |_, _, cx| cx.open_url(&url))
        .into_any_element()
}

/// "+2": the chip the ports without one of their own are behind. `Popover` marks it open while its
/// menu shows, and it takes the neutral fill then.
#[derive(IntoElement)]
struct MoreTrigger {
    ui: UiColors,
    text: SharedString,
    words: SharedString,
    open: bool,
}

impl Selectable for MoreTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for MoreTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let tip = self.words.clone();
        div()
            .id("title-more-ports")
            .ml_2()
            .role(Role::Button)
            .aria_label(self.words)
            .cursor_pointer()
            .child(style::extra_chip(self.ui, self.open, None, self.text))
            .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
    }
}

/// What a plan's extras are, for devctl's `title`: what is drawn and what the row gave up.
#[cfg(feature = "devctl")]
pub(in crate::workspace) fn describe(plan: &TitlePlan) -> String {
    let (facts, fit) = (&plan.facts.extras, &plan.fit.extras);
    let one = |extra: Extra, text: Option<String>| match text {
        None => "none".to_string(),
        Some(text) if fit.shows(extra) => text,
        Some(text) => format!("({text} given up)"),
    };
    let ports = (!facts.ports.is_empty()).then(|| format!("{:?}", facts.ports));
    format!(
        "branch={} gauge={} ports={} pr={}",
        one(Extra::BranchStatus, facts.branch.as_ref().map(|status| status.words())),
        one(Extra::Gauge, facts.gauge.as_ref().map(Gauge::words)),
        one(Extra::Ports, ports),
        one(Extra::PullRequest, facts.pull_request.as_ref().map(PullRequest::words)),
    )
}
