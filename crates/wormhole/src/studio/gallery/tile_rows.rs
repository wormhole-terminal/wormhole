//! The Settings window's tile row in each of its states, the solid chips, and the empty list.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::agents::AgentId;

use crate::settings::UiColors;
use crate::style::{self, ChipKind, RowTile};

/// As wide as the Settings content column.
const COLUMN_W: f32 = 560.0;

pub(super) fn tile_rows(ui: UiColors) -> Div {
    let row = |id: &'static str,
               tile: RowTile,
               name: &'static str,
               chip: Option<(ChipKind, &'static str)>,
               hint: &'static str,
               buttons: &[&'static str]| {
        let mut parts = style::TileRowParts::new(tile, name);
        parts.chip = chip.map(|(kind, word)| (kind, word.into()));
        parts.hint = Some(hint.into());
        parts.controls = buttons
            .iter()
            .map(|label| {
                style::form_button(SharedString::from(format!("{id}-{label}")))
                    .outline()
                    .label(*label)
                    .into_any_element()
            })
            .collect();
        style::tile_row(id, ui, parts)
    };
    let folder = RowTile::Icon { fill: ui.primary, icon: IconName::Folder };
    div()
        .flex()
        .flex_col()
        .gap_4()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(
            div()
                .flex()
                .flex_col()
                .child(row(
                    "g-tile-agent",
                    RowTile::Agent(AgentId::Claude),
                    "Claude Code", // text: sample (the gallery's specimen)
                    Some((ChipKind::Good, "Status hooks on")), // text: sample (the gallery's specimen)
                    "Installed · hooks in ~/.claude/settings.json", // text: sample (the gallery's specimen)
                    &["Remove hooks"], // text: sample (the gallery's specimen)
                ))
                .child(row(
                    "g-tile-probe",
                    folder,
                    "Port watcher",                    // text: sample (the gallery's specimen)
                    Some((ChipKind::Warn, "Changed")), // text: sample (the gallery's specimen)
                    "ports · version 1.1.0 · by acme", // text: sample (the gallery's specimen)
                    &["Approve again…"],               // text: sample (the gallery's specimen)
                ))
                .child(row(
                    "g-tile-project",
                    folder,
                    "api", // text: sample (the gallery's specimen)
                    None,
                    "~/Projects/api",        // text: sample (the gallery's specimen)
                    &["Settings", "Revoke"], // text: sample (the gallery's specimen)
                )),
        )
        .child(
            div().flex().gap_2().children(
                [AgentId::Claude, AgentId::Codex, AgentId::Gemini]
                    .map(|agent| style::brand_tile(ui, style::TileFace::Agent(agent), style::ROW_TILE)),
            ),
        )
        .child(
            div().flex().flex_wrap().gap_2().children(
                [
                    (ChipKind::Good, "Trusted"),            // text: sample (the gallery's specimen)
                    (ChipKind::Warn, "Needs repair"),       // text: sample (the gallery's specimen)
                    (ChipKind::Danger, "Can’t be enabled"), // text: sample (the gallery's specimen)
                    (ChipKind::Quiet, "Not trusted"),       // text: sample (the gallery's specimen)
                ]
                .map(|(kind, word)| style::solid_chip(ui, kind, word)),
            ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(style::more_row(
                    "g-more-closed",
                    ui,
                    "Other agents Wormhole recognises", // text: sample (the gallery's specimen)
                    Some("17".into()),                  // text: sample (the gallery's specimen)
                    false,
                )) // text: sample (the gallery's specimen)
                .child(style::more_row(
                    "g-more-open",
                    ui,
                    "Other agents Wormhole recognises", // text: sample (the gallery's specimen)
                    Some("17".into()),                  // text: sample (the gallery's specimen)
                    true,
                )), // text: sample (the gallery's specimen)
        )
        .child(style::empty_list(
            ui,
            style::EmptyList {
                icon: IconName::Puzzle,
                title: "No Probes yet".into(), // text: sample (the gallery's specimen)
                hint: "Make one with wh probe new, or copy a folder in.".into(), // text: sample (the gallery's specimen)
                actions: vec![
                    style::form_button("g-empty-show").outline().label("Show in Finder").into_any_element(), // text: sample (the gallery's specimen)
                ],
            },
        ))
}
