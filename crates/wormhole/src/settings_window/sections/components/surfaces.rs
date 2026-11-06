//! Surfaces and blocks: banners, notes, cards, disclosure rows, the nav, the preview's frame and what the
//! Theme Studio's gallery already draws of them.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{BannerKind, StatusGlyph, StatusTile, TileFace};
use wormhole_core::agents::AgentId;
use wormhole_core::config::presets::{Preset, Shown};

use super::{Sheet, category, specimen};
use crate::studio;
use crate::style::{self, Edges, TextRole};

/// The tabs "all of the tools" draws in a window sketch.
const SKETCH_TOOLS: usize = 4;

pub(super) fn draw(sheet: &Sheet, cx: &App) -> AnyElement {
    let ui = sheet.ui;
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(category(
            ui,
            "Banners and notes",             // text: sample (a category's head)
            "style/messages.rs, empty_note", // text: sample (a category's head)
            banners(sheet),
        ))
        .child(category(
            ui,
            "Cards, groups and disclosure",  // text: sample (a category's head)
            "card, choice_card, disclosure", // text: sample (a category's head)
            cards(sheet, cx),
        ))
        .child(category(
            ui,
            "Nav and the preview's frame", // text: sample (a category's head)
            "also drawn by the Theme Studio's gallery", // text: sample (a category's head)
            gallery(sheet),
        ))
        .into_any_element()
}

/// Banners of each kind and the notes that stand where a list would be.
fn banners(sheet: &Sheet) -> Vec<AnyElement> {
    let ui = sheet.ui;
    let small = |id: &'static str, label: &'static str| Button::new(id).small().outline().label(label);
    vec![
        specimen(ui, "banner · info, warning, danger, success, flush", studio::banner_samples(ui)) // text: sample (a developer label)
            .into_any_element(), // text: sample (a specimen's name)
        specimen(
            ui,
            "callout", // text: sample (a specimen's name)
            style::callout(
                ui,
                BannerKind::Warning,
                "This project asks to run commands", // text: sample (a callout's title)
                [
                    div()
                        .text_caption()
                        .text_color(ui.muted)
                        .child("It lists two actions. Nothing runs until you trust it.") // text: sample (a developer label)
                        .into_any_element(), // text: sample (a callout's body)
                    div().flex().child(small("components-callout", "Review")).into_any_element(), // text: sample (a button's label)
                ],
            ),
        )
        .into_any_element(),
        specimen(
            ui,
            "reset_question · a banner that asks", // text: sample (a specimen's name)
            super::super::reset_question(ui, 3, |_, _, _| {}, |_, _, _| {}),
        )
        .into_any_element(),
        specimen(
            ui,
            "empty_note · a loading or empty line", // text: sample (a specimen's name)
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::empty_note(ui, "Looking for terminals…")) // text: sample (a loading note)
                .child(style::empty_note(ui, "Nothing to import yet.")), // text: sample (an empty note)
        )
        .into_any_element(),
        specimen(
            ui,
            "readout",                                           // text: sample (a specimen's name)
            div().flex().child(style::readout(ui, "430 · max")), // text: sample (a readout)
        )
        .into_any_element(),
    ]
}

/// Cards, the summary of the Changed page, a choice card that picks, and a disclosure that folds.
fn cards(sheet: &Sheet, cx: &App) -> Vec<AnyElement> {
    let ui = sheet.ui;
    let now = &sheet.now;
    let summary = style::card(ui, ui.border)
        .flex()
        .items_center()
        .gap_3()
        .child(style::status_tile_at(
            "components-summary-tile",
            ui,
            StatusTile {
                face: TileFace::Agent(AgentId::Claude),
                glyph: StatusGlyph::Done,
                selected: false,
                mini: false,
            },
            style::TILE,
            cx,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child("5 settings differ")) // text: sample (the Changed page's summary)
                .child(
                    div().text_caption().text_color(ui.muted).child("On four pages. Reset one from its row."), // text: sample (a developer label)
                ), // text: sample (the Changed page's summary)
        )
        .child(Button::new("components-copy").outline().small().icon(IconName::Copy).label("Copy as JSON")) // text: sample (a button's label)
        .child(Button::new("components-reset").danger().outline().small().label("Reset all…")); // text: sample (a button's label)
    let hairline = div().flex().flex_col().rule_top(ui).children(
        ["First line of a list", "Second line", "Third line"].map(|label| { // text: sample (a developer label)
            // text: sample (a list's lines)
            div().h(px(style::ROW_H)).flex().items_center().px_2().rule_bottom(ui).text_body().child(label)
        }),
    );
    let choices = div().flex().gap_2().w_full().children(
        Preset::ALL.iter().zip(["Zen", "Balanced", "Everything"]).enumerate().map(|(ix, (preset, name))| { // text: sample (a developer label)
            let state = sheet.state.clone();
            style::choice_card(("components-card", ix), ui, ix == now.card)
                .flex_1()
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        s.now.card = ix;
                        cx.notify();
                    })
                })
                .child(div().p_1().child(style::window_sketch(ui, &Shown::of(*preset), SKETCH_TOOLS)))
                .child(
                    div()
                        .p_1()
                        .rule_top(ui)
                        .text_caption()
                        .text_color(if ix == now.card { ui.primary } else { ui.text })
                        .child(name),
                )
        }),
    );
    let folded = now.folded;
    let state = sheet.state.clone();
    let disclosure = style::card(ui, ui.border)
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .id("components-disclosure")
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .child(style::lead(style::disclosure(ui, folded)))
                .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child("Ghostty")) // text: sample (a card's title)
                .on_click(move |_, _, cx| {
                    state.update(cx, |s, cx| {
                        s.now.folded = !s.now.folded;
                        cx.notify();
                    })
                }),
        )
        .when(!folded, |card| {
            card.child(
                div()
                    .pl_6()
                    .text_caption()
                    .text_color(ui.muted)
                    .child("13 settings can come over; 2 don't carry over."), // text: sample (a developer label)
            ) // text: sample (a card's body)
        });
    vec![
        specimen(ui, "card · the summary of the Changed page", summary).into_any_element(), // text: sample (a specimen's name)
        specimen(ui, "hairline group · rows between rules", hairline).into_any_element(), // text: sample (a specimen's name)
        specimen(ui, "choice_card · window_sketch, chosen on click", choices).into_any_element(), // text: sample (a specimen's name)
        specimen(ui, "card · disclosure, folds on click", disclosure).into_any_element(), // text: sample (a specimen's name)
    ]
}

/// What the Studio's gallery draws with nothing of the Studio in it.
fn gallery(sheet: &Sheet) -> Vec<AnyElement> {
    let ui = sheet.ui;
    vec![
        specimen(
            ui,
            "nav_row · nav_group_label: default, current, count, attention, hits, none, preview", // text: sample (a developer label)
            studio::nav_samples(ui),
        )
        .into_any_element(), // text: sample (a specimen's name)
        specimen(
            ui,
            "swatch, tint_chip, keycap, preview_frame, preview_stage, preview_surface, shortcut_recorder, arrange_row", // text: sample (a specimen's name)
            studio::control_samples(ui, &sheet.tokens),
        )
        .into_any_element(),
        specimen(
            ui,
            "profile_avatar, choice_row, profile_chip, popover_row, profile_edge", // text: sample (a developer label)
            studio::profile_samples(ui, &sheet.tokens),
        ) // text: sample (a specimen's name)
        .into_any_element(),
    ]
}
