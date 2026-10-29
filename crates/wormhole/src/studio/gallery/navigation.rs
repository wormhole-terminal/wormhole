//! Tabs, chips, badges, rows, cards and a tooltip: the shared components of `style`, on the live tokens.

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::style::{self, TextRole};

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let tk = |name: &str| env.tk(name);
    // The same components the panels are built from, on the live tokens.
    let tabs = div()
        .flex()
        .gap_1()
        .child(style::pill("g-tab-usage", ui, true, true).child("Usage")) // text: sample (the gallery's specimen)
        .child(
            style::pill("g-tab-changes", ui, false, true).child("Changes").child(style::count_badge(ui, "3")), // text: sample (the gallery's specimen)
        )
        .child(style::pill("g-tab-history", ui, false, true).child("History")); // text: sample (the gallery's specimen)
    // The right panel's tabs.
    let icon_tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .child(style::icon_tab("g-itab-usage", ui, IconName::Gauge, "Usage", false, Some(2))) // text: sample (the gallery's specimen)
        // Active with a count: the tab is the selected tint, its badge `state.selected.inset`.
        .child(style::icon_tab("g-itab-changes", ui, IconName::GitCompare, "Changes", true, Some(3))) // text: sample (the gallery's specimen)
        .child(style::icon_tab("g-itab-history", ui, IconName::GitCommitHorizontal, "History", false, None)); // text: sample (the gallery's specimen)
    // The same tabs with the selection as one shape that springs between them (Motion on): the selected
    // tab has no fill of its own.
    let sliding_tabs = style::selection_bar(
        "g-sliding-tabs",
        ui,
        Some(1),
        &style::TabSlots::default(),
        [
            style::icon_tab("g-stab-usage", ui, IconName::Gauge, "Usage", false, Some(2)).into_any_element(), // text: sample (the gallery's specimen)
            style::icon_tab("g-stab-changes", ui, IconName::GitCompare, "Changes", true, Some(3)) // text: sample (the gallery's specimen)
                .bg(transparent_black())
                .into_any_element(),
            style::icon_tab("g-stab-history", ui, IconName::GitCommitHorizontal, "History", false, None) // text: sample (the gallery's specimen)
                .into_any_element(),
        ],
    );
    // The terminal's tabs: the active one quiet on `tab.active.background` with its ×,
    // one waiting for an answer in the attention tint and ring.
    let glyph = |id: &'static str, glyph| style::status_glyph(id, ui, glyph, cx);
    let title_tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(style::TITLE_ROW_H))
        .child(
            env.mark(
                "title.tab",
                style::title_tab("g-ttab-claude", ui, true, false)
                    .child(glyph("g-ttab-spin", style::StatusGlyph::Working))
                    .child("claude")
                    .child(style::close_x("g-ttab-close", ui, style::SelectedFill::Plain)),
            ),
        )
        .child(
            style::title_tab("g-ttab-cargo", ui, false, false)
                .child(glyph("g-ttab-run", style::StatusGlyph::Running))
                .child("cargo test"), // text: sample (the gallery's specimen)
        )
        .child(
            style::title_tab("g-ttab-waiting", ui, false, true)
                .child(div().text_color(ui.agent_attention).child(style::icon(IconName::CircleAlert)))
                .child("deploy"),
        );
    let chips = div()
        .flex()
        .gap_2()
        .child(style::pill("g-chip-a", ui, true, false).child("Tokyo Night")) // text: sample (the gallery's specimen)
        .child(style::pill("g-chip-b", ui, false, false).child("Nord")) // text: sample (the gallery's specimen)
        .child(style::pill("g-chip-c", ui, false, false).child("Dracula")); // text: sample (the gallery's specimen)
    let list = div()
        .flex()
        .flex_col()
        .w_full()
        .max_w(px(280.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .pt_1()
                .child(style::drag_ghost(ui, None, "Wire up the CLI".into()).flex_none()) // text: sample (the gallery's specimen)
                .child(div().text_caption().text_color(ui.muted).child("← an item being dragged")), // text: sample (the gallery's specimen)
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .pt_1()
                .child(
                    style::drag_ghost(
                        ui,
                        Some(glyph("g-drag-glyph", style::StatusGlyph::NeedsYou)),
                        "diff perf".into(), // text: sample (the gallery's specimen)
                    )
                    .flex_none(),
                )
                .child(div().text_caption().text_color(ui.muted).child("← a terminal being dragged")), // text: sample (the gallery's specimen)
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .pt_1()
                .child(
                    style::drag_lift(style::drag_ghost(ui, None, "Wire up the CLI".into()), ui, 1.0) // text: sample (the gallery's specimen)
                        .flex_none(),
                )
                .child(div().text_caption().text_color(ui.muted).child("← lifted off the list")), // text: sample (the gallery's specimen)
        )
        .child(
            div().px_2().pt_1().child(style::card_lifted(
                ui,
                style::Metrics::default(),
                false,
                240.0,
                div()
                    .px_2()
                    .h(px(40.0))
                    .flex()
                    .items_center()
                    .text_body()
                    .text_color(ui.text)
                    .child("A row, lifted"), // text: sample (the gallery's specimen)
            )),
        )
        .child(
            div()
                .relative()
                .h(px(32.0))
                .mx_2()
                .mt_1()
                .child(style::row_drop_target(ui))
                .child(style::split_mark(ui))
                .child(div().px_2().text_body().text_color(ui.text).child("A row that takes a split")), // text: sample (the gallery's specimen)
        )
        .child(style::row("g-row-a", ui, true).child("A selected row")) // text: sample (the gallery's specimen)
        .child(style::row("g-row-b", ui, false).child("A row")) // text: sample (the gallery's specimen)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .pt_2()
                .child(div().text_caption().text_color(ui.muted).child("Close the diff")) // text: sample (the gallery's specimen)
                .child(env.mark("keycap", style::keycap(ui, "esc"))),
        );
    let badges = div()
        .flex()
        .items_center()
        .gap_2()
        .child(env.mark("badge", style::count_badge(ui, "1")))
        .child(style::count_badge(ui, "12"))
        .child(style::status_badge(ui, ui.success_text, "installed"))
        .child(style::status_badge(ui, ui.warning_text, "changed"))
        .child(style::status_badge(ui, ui.muted, "not installed")) // text: sample (the gallery's specimen)
        .child(style::keycap(ui, "esc"));
    // What a token is used by (the inspector's "Used by"): grey, unlike an override's tint chip.
    let neutral = div()
        .flex()
        .flex_wrap()
        .gap_1()
        .child(env.mark("neutral.chip", style::neutral_chip(ui, "sidebar row"))) // text: sample (the gallery's specimen)
        .child(style::neutral_chip(ui, "folder band")) // text: sample (the gallery's specimen)
        .child(style::neutral_chip(ui, "settings nav")); // text: sample (the gallery's specimen)
    let card = style::card(ui, ui.border)
        .w_full()
        .max_w(px(280.0))
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_body().child("A card sets a few lines apart")) // text: sample (the gallery's specimen)
        .child(div().text_caption().text_color(ui.muted).child("Release notes, a hint.")); // text: sample (the gallery's specimen)
    let navigation = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(icon_tabs)
        .child(sliding_tabs)
        .child(title_tabs)
        .child(tabs)
        .child(chips)
        .child(badges)
        .child(neutral)
        .child(list)
        .child(env.mark("card", card))
        .child(
            env.mark(
                "tooltip",
                div()
                    .px_2()
                    .py_1()
                    .w(px(220.0))
                    .rounded(ui.radius)
                    .text_caption()
                    .bg(tk("tooltip.background"))
                    .text_color(tk("tooltip.text"))
                    .child("A tooltip explains a control"), // text: sample (the gallery's specimen)
            ),
        );
    navigation.into_any_element()
}
