//! The select: a trigger with chevrons, and the menu it opens (a search row over roomy
//! options on a lifted surface).

use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::select;
use crate::style::TextRole;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let surface = select::menu_bg(ui, cx);
    let menu = div()
        .w(px(260.0))
        .rounded(ui.radius_lg)
        .bg(surface)
        .shadow(select::menu_shadow(ui))
        .child(select::search_row(ui, false, div().text_body().text_color(ui.faint).child("Search themes")))
        .child(
            div()
                .flex()
                .flex_col()
                .p_1()
                .child(select::option("g-opt-a", ui, true, false).child("Tokyo Night"))
                .child(select::option("g-opt-b", ui, false, false).child("Nord"))
                .child(select::option("g-opt-c", ui, false, false).child("Dracula")),
        );
    div()
        .flex()
        .flex_wrap()
        .gap_4()
        .items_start()
        .child(select::SelectTrigger::new("g-select", ui, "Tokyo Night").width(px(220.0)))
        .child(select::SelectTrigger::new("g-select-compact", ui, "main").compact())
        .child(env.mark("menu", menu))
        .into_any_element()
}
