//! The agents that aren't on PATH.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::how::hang;
use super::mark;
use super::snapshot::AgentRow;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

/// The agents that aren't on PATH, folded into one row that lists them when opened.
pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let others: Vec<&AgentRow> = snapshot.others().collect();
    if others.is_empty() {
        return div().into_any_element();
    }
    let names = others.iter().map(|row| row.agent.name).collect::<Vec<_>>().join(" · ");
    let open = window.others_open;
    let head = div()
        .id("agents-others")
        .flex()
        .items_center()
        .gap_2()
        .py_2()
        .rule_top(ui)
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .on_click(cx.listener(|this, _, _, cx| {
            this.others_open = !this.others_open;
            cx.notify();
        }))
        .relative()
        .child(hang(style::lead(style::disclosure(ui, !open))).bottom_0().flex().items_center())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_body()
                        .font_weight(FontWeight::MEDIUM)
                        .child(settings_agents_others_head(whole(others.len()))),
                )
                .when(!open, |d| d.child(div().text_caption().text_color(ui.muted).child(names))),
        )
        .child(
            div().flex_none().text_caption().text_color(ui.faint).child(text(SETTINGS_AGENTS_NOT_ON_PATH)),
        );
    div()
        .flex()
        .flex_col()
        .child(head)
        .when(open, |d| {
            d.children(others.into_iter().map(|row| {
                let hooks = if row.spec.is_some() {
                    text(SETTINGS_AGENTS_STATUS_HOOKS_AVAILABLE)
                } else {
                    text(SETTINGS_AGENTS_STATUS_FROM_OUTPUT)
                };
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1()
                    .child(mark(ui, row.agent))
                    .child(div().flex_1().min_w_0().text_body().text_color(ui.muted).child(row.agent.name))
                    .child(div().text_caption().text_color(ui.faint).child(hooks))
            }))
        })
        .into_any_element()
}
