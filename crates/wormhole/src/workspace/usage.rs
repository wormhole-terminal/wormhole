//! The right panel's Usage tab: each coding agent's plan usage limits, as its status line last
//! reported them (`wormhole_core::usage`), plus Gemini’s latest model-response tokens.
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::progress::Progress;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents;
use wormhole_core::usage::{self, PlanLimit, Usage};

use super::Workspace;
use super::app::app;
use crate::settings::{UiColors, ui_in};
use crate::style::{self, TextRole};

/// From this share used on, a bar turns to the warning color, and at the limit to danger.
const WARN_AT: f32 = 0.8;

/// The Usage tab redraws this often while it shows, so "resets 34m" and "Updated 2m ago" move
/// on (in ticks of the window, 400 ms each).
pub(super) const REDRAW_TICKS: u64 = 25;

impl Workspace {
    pub(super) fn render_usage_tab(&self, cx: &Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let reports = &app(cx).usage;
        if reports.is_empty() {
            return style::panel_card(
                ui,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px(px(style::CARD_INSET))
                    .py_2()
                    .text_body()
                    .text_color(ui.muted)
                    .child(
                        "Claude and Codex plan limits, and Gemini response tokens, show here as you use them in Wormhole.",
                    )
                    .child(
                        "Enable or update their integrations in Settings → Coding agents. Plan limits need a supported subscription; API-key logins may not report them.",
                    ),
            )
            .into_any_element();
        }
        let now = unix_now();
        div()
            .flex()
            .flex_col()
            .children(reports.iter().map(|report| style::panel_card(ui, agent_block(report, now, ui))))
            .into_any_element()
    }
}

/// One agent: its name and when the numbers came, then a row and a bar per limit.
fn agent_block(report: &Usage, now: i64, ui: UiColors) -> AnyElement {
    let agent = agents::agent(report.agent);
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::BAND_H))
        .px(px(style::CARD_INSET))
        .child(style::lead(style::agent_logo(agent.icon(), agent.monogram, style::GLYPH, ui.muted)))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_body()
                .font_weight(FontWeight::SEMIBOLD)
                .child(agent.short_name()),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex_none()
                .text_caption()
                .tabular_nums()
                .text_color(ui.muted)
                .child(usage::updated(report.updated, now)),
        );
    let limits = report.plan_limits();
    div()
        .flex()
        .flex_col()
        .child(header)
        .when(!limits.is_empty(), |card| {
            card.child(
                div().px(px(style::CARD_INSET)).pb_1().child(style::section_label(ui, "Plan usage limits")),
            )
        })
        .children(limits.iter().enumerate().map(|(index, limit)| limit_row(report, index, limit, now, ui)))
        .when_some(report.response_tokens, |card, tokens| {
            card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px(px(style::CARD_INSET))
                    .py_2()
                    .child(style::section_label(ui, "Latest model response"))
                    .child(div().text_body().tabular_nums().child(format!("{tokens} tokens")))
                    .child(
                        div()
                            .text_caption()
                            .text_color(ui.muted)
                            .child("Latest report across Gemini terminals. Plan limits are not reported."),
                    ),
            )
        })
        .into_any_element()
}

/// "5-hour limit … 25% · resets 34m" over its bar.
fn limit_row(report: &Usage, index: usize, limit: &PlanLimit, now: i64, ui: UiColors) -> AnyElement {
    let (share, words) = limit.shown(now);
    let color = match share {
        s if s >= 1.0 => ui.danger,
        s if s >= WARN_AT => ui.warning,
        _ => ui.primary,
    };
    let id = format!("usage-{:?}-{index}", report.agent);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px(px(style::CARD_INSET))
        .py_2()
        .child(
            div()
                .flex()
                .items_baseline()
                .gap_2()
                .child(div().min_w_0().truncate().text_body().child(limit.label.clone()))
                .child(div().flex_1())
                .child(
                    div().flex_none().text_caption().tabular_nums().text_color(ui.muted).child(words.clone()),
                ),
        )
        .child(
            Progress::new(SharedString::from(id))
                .value(share * 100.0)
                .color(color)
                .accessibility_label(format!("{}, {words}", limit.label.clone())),
        )
        .into_any_element()
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64)
}
