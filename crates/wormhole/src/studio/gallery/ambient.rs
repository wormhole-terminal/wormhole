//! What the app shows outside the lists: the quick panel and the
//! command palette with their parts, and the readout beside a resize handle.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use style::StatusGlyph;

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
    let app: &App = cx;
    let glyph = |id: &'static str, glyph: StatusGlyph| style::status_glyph(id, ui, glyph, app);

    let (code_font, _) = crate::fonts::code_font(app);
    let time = |text: &'static str| {
        div().text_caption().tabular_nums().text_color(ui.faint).child(text).into_any_element()
    };
    let result = |lead: AnyElement, title: &'static str| style::PaletteResult::new(lead, title);
    let mut needs = result(glyph("g-qp-needs-glyph", StatusGlyph::NeedsYou).into_any_element(), "diff perf");
    needs.strong = true;
    needs.detail = Some("Approve cargo test?".into());
    needs.detail_color = Some(ui.attention_text);
    needs.trailing.push(time("2m"));
    let mut tests = result(glyph("g-qp-tests-glyph", StatusGlyph::Running).into_any_element(), "tests");
    tests.strong = true;
    tests.detail = Some("cargo test -p wormhole-core".into());
    tests.detail_font = Some(code_font);
    tests.trailing.push(time("1:12"));
    let mut build = result(glyph("g-qp-build-glyph", StatusGlyph::Failed).into_any_element(), "build");
    build.strong = true;
    build.detail = Some("cargo build · exit 101".into());
    build.detail_color = Some(ui.danger_text);
    build.trailing.push(time("2m ago"));
    let mut new = result(style::icon(IconName::Plus).into_any_element(), "New terminal");
    new.detail = Some("in ~/Projects/terminal".into());
    new.trailing.push(style::keycap(ui, "⇧⌘N").into_any_element());
    let panel = style::palette_surface(ui, false)
        .w_full()
        .max_w(px(style::QUICK_PANEL_W))
        .child(
            style::palette_search(ui)
                .child(div().flex_1().text_color(ui.faint).child("Search terminals and actions"))
                .child(style::keycap(ui, "⇧⌘Space")),
        )
        .child(style::palette_label(ui, "Needs input", Some(2)))
        .child(style::palette_result("g-qp-needs", ui, true, needs))
        .child(style::palette_label(ui, "Running", None))
        .child(style::palette_result("g-qp-tests", ui, false, tests))
        .child(style::palette_result("g-qp-build", ui, false, build))
        .child(style::palette_result("g-qp-new", ui, false, new))
        .child(style::palette_footer(ui).child(style::key_hint(ui, "⏎", "open")).child(style::key_hint(
            ui,
            "⌘⏎",
            "answer here",
        )));

    // The command palette: a search's marks, a question under the terminal that asks, a
    // setting's value; floating over the window.
    let mut theme = result(style::icon(IconName::Palette).into_any_element(), "Theme");
    theme.marks.push(0..5);
    theme.detail = Some("Settings › Appearance".into());
    theme.trailing.push(
        div().text_caption().text_color(ui.list_selected_muted).child("Tokyo Night").into_any_element(),
    );
    theme.trailing.push(style::keycap(ui, "↩").into_any_element());
    let mut studio =
        result(style::icon(IconName::Palette).into_any_element(), "Translucent Theme Studio window");
    studio.marks.push(12..17);
    studio.detail = Some("Appearance".into());
    let mut asking =
        result(glyph("g-pal-needs-glyph", StatusGlyph::NeedsYou).into_any_element(), "diff perf");
    asking.strong = true;
    asking.detail = Some("Window 1 · terminal".into());
    asking.second = Some("Claude Code is asking: Approve cargo test?".into());
    asking.trailing.push(style::keycap(ui, "⌘2").into_any_element());
    let palette = style::palette_surface(ui, true)
        .w_full()
        .max_w(px(style::QUICK_PANEL_W))
        .child(
            style::palette_search(ui)
                .child(div().flex_1().child(style::marked_text(ui, "theme".into(), &[], false)))
                .child(style::keycap(ui, "esc")),
        )
        .child(style::palette_label(ui, "Settings", Some(2)))
        .child(env.mark("palette.row", style::palette_result("g-pal-theme", ui, true, theme)))
        .child(style::palette_result("g-pal-studio", ui, false, studio))
        .child(style::palette_label(ui, "Terminals", Some(1)))
        .child(style::palette_result("g-pal-asking", ui, false, asking))
        .child(
            style::palette_footer(ui)
                .child(style::key_hint(ui, "↩", "open in Settings"))
                .child(style::key_hint(ui, "⌘C", "copy setting key"))
                .child(div().flex_1())
                .child("3 results"),
        );

    // A panel's edge being dragged: the handle's line, and the readout beside it.
    let handle = div().w(px(style::DROP_LINE_W)).h(px(style::CONTROL_H)).bg(env.tk("resize.handle.active"));
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(panel)
        .child(palette)
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(env.mark("resize.handle", handle))
                .child(env.mark("readout", style::readout(ui, "430 · max")))
                .child(style::readout(ui, "350"))
                .child(
                    div().text_caption().text_color(ui.muted).child("← the width beside a dragged handle"),
                ),
        )
        // A hovered command block's readout: its exit code, its time and Copy output.
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    style::block_readout(ui, Some(101), Some("3.5s")).child(
                        style::icon_button("g-block-copy", ui, IconName::Copy, "Copy output").in_row(),
                    ),
                )
                .child(style::block_readout(ui, Some(0), Some("12s")))
                .child(div().text_caption().text_color(ui.muted).child("← a hovered command block")),
        )
        .into_any_element()
}
