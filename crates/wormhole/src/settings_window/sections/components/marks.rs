//! Small marks: badges, chips, dots, key caps, colour choices, status tiles and agent logos.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use style::{StatusGlyph, StatusTile, TileFace};
use wormhole_core::agents::AgentId;

use super::{Sheet, category, specimen};
use crate::style::{self, TextRole};

pub(super) fn draw(sheet: &Sheet, cx: &App) -> AnyElement {
    let ui = sheet.ui;
    let now = &sheet.now;
    let line = |items: Vec<AnyElement>| div().flex().flex_wrap().items_center().gap_3().children(items);
    let counts = line(vec![
        style::count_badge(ui, "7").into_any_element(),
        style::count_badge(ui, "128").into_any_element(),
        style::selected_count_badge(ui, "3").into_any_element(),
    ]);
    let statuses = line(vec![
        style::status_badge(ui, ui.success, "Trusted").into_any_element(), // text: sample (a status word)
        style::status_badge(ui, ui.warning, "Needs repair").into_any_element(), // text: sample (a status word)
        style::status_badge(ui, ui.danger, "Failed").into_any_element(), // text: sample (a status word)
        style::status_badge(ui, ui.info, "Changed").into_any_element(),  // text: sample (a status word)
        style::status_badge(ui, ui.muted, "Off").into_any_element(),     // text: sample (a status word)
        style::status_badge(ui, ui.primary, "Beta").into_any_element(),  // text: sample (a status word)
    ]);
    let chips = line(vec![
        style::tint_chip(ui, Some(IconName::Folder), "api-server").into_any_element(), // text: sample (a chip)
        style::tint_chip(ui, None, "Bundled").into_any_element(), // text: sample (a chip)
        style::neutral_chip(ui, "Used by sidebar").into_any_element(), // text: sample (a chip)
        style::branch_chip(ui, Some("main"), Some(3)).into_any_element(),
        style::branch_chip(ui, None, None).into_any_element(),
        style::changed_dot(ui).into_any_element(),
    ]);
    let keys = line(vec![
        style::keycap(ui, "⌘,").into_any_element(),
        style::keycap(ui, "⇧⌘P").into_any_element(),
        style::keycap(ui, "esc").into_any_element(),
        style::key_hint(ui, "⌘F", "Find").into_any_element(), // text: sample (a key hint)
    ]);
    let swatches = {
        let palette = [ui.primary, ui.success, ui.branch, ui.warning, ui.danger];
        div().flex().items_center().gap_3().children(palette.into_iter().enumerate().map(|(ix, color)| {
            let state = sheet.state.clone();
            style::swatch(("components-swatch", ix), ui, color, ix == now.swatch).on_click(move |_, _, cx| {
                state.update(cx, |s, cx| {
                    s.now.swatch = ix;
                    cx.notify();
                })
            })
        }))
    };
    let folder_colors = div().flex().items_center().children(
        [None, Some(ui.primary), Some(ui.success), Some(ui.warning)].into_iter().enumerate().map(
            |(ix, color)| {
                let state = sheet.state.clone();
                style::color_choice(("components-color", ix), ui, "A colour", color, ix == now.choice, true) // text: sample (a colour's name for VoiceOver)
                    .on_click(move |_, _, cx| {
                        state.update(cx, |s, cx| {
                            s.now.choice = ix;
                            cx.notify();
                        })
                    })
            },
        ),
    );
    let tile = |id: &'static str, face: TileFace, glyph: StatusGlyph| {
        style::status_tile_at(
            id,
            ui,
            StatusTile { face, glyph, selected: false, mini: false },
            style::TILE,
            cx,
        )
    };
    let tiles = line(vec![
        tile("components-tile-1", TileFace::Agent(AgentId::Claude), StatusGlyph::Idle),
        tile("components-tile-2", TileFace::Agent(AgentId::Claude), StatusGlyph::Working),
        tile("components-tile-3", TileFace::Agent(AgentId::Claude), StatusGlyph::NeedsYou),
        tile("components-tile-4", TileFace::Agent(AgentId::Claude), StatusGlyph::Done),
        tile("components-tile-5", TileFace::Agent(AgentId::Gemini), StatusGlyph::Failed),
        tile("components-tile-6", TileFace::Terminal, StatusGlyph::Shell),
        tile("components-tile-7", TileFace::Agent(AgentId::Aider), StatusGlyph::Done),
    ]);
    let logos = line(vec![
        style::agent_logo(
            wormhole_term::agents::agent(AgentId::Claude).icon(),
            "CC", // text: sample (a developer label)
            style::TITLE_GLYPH,
            ui.muted,
        ), // text: sample (a monogram)
        style::agent_logo(None, "AI", style::TITLE_GLYPH, ui.muted), // text: sample (a monogram)
        style::agent_logo(None, "Pi", style::TITLE_GLYPH, ui.text),  // text: sample (a monogram)
    ]);
    category(
        ui,
        "Badges, chips and marks", // text: sample (a category's head)
        "style/labels.rs, tile, folder_colors", // text: sample (a category's head)
        [
            specimen(ui, "count_badge · selected_count_badge", counts).into_any_element(), // text: sample (a developer label)
            specimen(ui, "status_badge · success, warning, danger, info, neutral, primary", statuses).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "tint_chip · neutral_chip · branch_chip · changed_dot", chips).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "keycap · key_hint", keys).into_any_element(), // text: sample (a developer label)
            specimen(ui, "swatch · chosen on click", swatches).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "color_choice · none and colours", folder_colors).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "status_tile_at · agent faces and states", tiles).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "agent_logo · a logo, a monogram", logos).into_any_element(), // text: sample (a specimen's name)
            div().text_caption().text_color(ui.faint).child("Also in the gallery's controls below: tint chips and key caps beside the preview's frame.").into_any_element(), // text: sample (a note)
        ],
    )
    .into_any_element()
}
