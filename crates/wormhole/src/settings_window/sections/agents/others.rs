//! "Other agents Wormhole recognises": the agents that aren't on this Mac's PATH, behind one row that opens
//! them. Closed it says how many there are; open, each is a tile row with how its status would come.

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::snapshot::AgentRow;
use crate::settings_window::{SettingsWindow, ui};
use crate::style;

pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(snapshot) = &window.agents else { return div().into_any_element() };
    let others: Vec<&AgentRow> = snapshot.others().collect();
    if others.is_empty() {
        return div().into_any_element();
    }
    let open = window.others_open;
    let head = style::more_row(
        "agents-others",
        ui,
        text(SETTINGS_AGENTS_OTHER),
        Some(whole(others.len()).to_string().into()),
        open,
        cx,
    )
    .on_click(cx.listener(|this, _, _, cx| {
        this.others_open = !this.others_open;
        cx.notify();
    }));
    div()
        .flex()
        .flex_col()
        .child(head)
        .children(
            open.then(|| {
                div().flex().flex_col().children(others.into_iter().map(|row| other_row(ui, row, cx)))
            }),
        )
        .into_any_element()
}

/// One agent that isn't on PATH: its name, and that it isn't, with what its status would come from.
fn other_row(ui: crate::settings::UiColors, row: &AgentRow, cx: &App) -> AnyElement {
    let status = text(if row.spec.is_some() {
        SETTINGS_AGENTS_STATUS_HOOKS_AVAILABLE
    } else {
        SETTINGS_AGENTS_STATUS_FROM_OUTPUT
    });
    let mut parts = style::TileRowParts::new(style::RowTile::Agent(row.agent.id), row.agent.name);
    parts.hint = Some(format!("{} · {status}", text(SETTINGS_AGENTS_NOT_ON_PATH)).into());
    style::tile_row(SharedString::from(format!("agent-other-{:?}", row.agent.id)), ui, parts, cx)
        .into_any_element()
}
