//! The counts across the top of the page.

use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::integrations::State;

use crate::settings_window::{SettingsWindow, ui};
use crate::style::{Edges, TextRole};

/// Four counts across the column: installed, reporting through hooks, needing repair, and the
/// ones Wormhole knows that aren't on PATH.
pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else {
        return div()
            .py_3()
            .text_body()
            .text_color(ui.muted)
            .child(text(SETTINGS_AGENTS_LOOKING_FOR_AGENTS))
            .into_any_element();
    };
    let installed = snapshot.installed().count();
    let hooked = snapshot.installed().filter(|row| row.hooks() == Some(State::Installed)).count();
    let repair = snapshot.needs_repair();
    let others = snapshot.others().count();
    let cell = |n: usize, label: &'static str, color: Hsla, first: bool| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .px_4()
            .py_2()
            .when(!first, |d| d.rule_left(ui))
            .child(
                div()
                    .text_title()
                    .font_weight(FontWeight::SEMIBOLD)
                    .tabular_nums()
                    .text_color(color)
                    .child(n.to_string()),
            )
            .child(div().text_caption().text_color(ui.muted).child(label))
    };
    div()
        .flex()
        .rule_top(ui)
        .rule_bottom(ui)
        .child(cell(installed, "installed", ui.text, true))
        .child(cell(
            hooked,
            text(SETTINGS_AGENTS_REPORTING_THROUGH_HOOKS),
            if hooked > 0 { ui.success_text } else { ui.text },
            false,
        ))
        .child(cell(
            repair,
            text(SETTINGS_AGENTS_NEEDS_REPAIR),
            if repair > 0 { ui.warning_text } else { ui.text },
            false,
        ))
        .child(cell(others, text(SETTINGS_AGENTS_RECOGNISED_NOT_ON_PATH), ui.muted, false))
        .into_any_element()
}
