//! Titles, notes and toolbars: the text that heads or fills a panel.

use super::*;

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    div()
        .flex()
        .flex_col()
        .gap_3()
        .w_full()
        .max_w(px(320.0))
        .child(style::panel_title(ui, "Next steps"))
        .child(style::section_label(ui, "Section label"))
        .child(style::empty_note(ui, "Nothing here yet."))
        // A bar of controls, with the branch marks that git panels use.
        .child(
            style::toolbar()
                .child(style::branch_label(ui, Some("main")))
                .child(style::branch_label(ui, None))
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_caption()
                        .child(style::branch_mark(ui))
                        .child("a select's own text"),
                ),
        )
        // The count on a selected tint: a stronger tint, in the tint's text color.
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .h(px(style::CONTROL_H))
                .rounded(ui.radius)
                .bg(ui.selected)
                .text_color(ui.selected_text)
                .child("Changes")
                .child(style::selected_count_badge(ui, "3")),
        )
        .into_any_element()
}
