//! Corner radii, the focus ring, and every chip-shaped component side by side.

use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::settings::UiColors;
use crate::studio::ui;
use crate::style::{self, Edges, TextRole};

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    // Every chip-shaped component side by side: they share `radius_chip`.
    let chip_family = div()
        .flex()
        .items_center()
        .gap_2()
        .child(style::pill("g-shape-tab", ui, true, true).child("Tab"))
        .child(style::pill("g-shape-chip", ui, true, false).child("Chip"))
        .child(style::count_badge(ui, "3"))
        .child(style::status_badge(ui, ui.success_text, "badge"))
        .child(style::keycap(ui, "⌘K"))
        .child(
            env.mark(
                "segmented",
                style::segmented(ui)
                    .child(style::segment("g-shape-seg-a", ui, "Split", true))
                    .child(style::segment("g-shape-seg-b", ui, "Unified", false)),
            ),
        );
    // The keyboard focus ring round a control, and inside a row (both drawn here always).
    let rings = div()
        .flex()
        .items_center()
        .gap_6()
        .p_2()
        .child(
            div()
                .relative()
                .px_3()
                .h(px(style::CONTROL_H))
                .flex()
                .items_center()
                .rounded(ui.radius)
                .outlined(ui, ui.border_strong)
                .text_body()
                .child("A control")
                .child(style::focus_ring(ui, style::RingPlacement::Outside, ui.radius)),
        )
        .child(
            div()
                .relative()
                .w_full()
                .max_w(px(240.0))
                .child(style::flush_row("g-ring-row", ui, false).child("A row in a list"))
                .child(style::focus_ring(ui, style::RingPlacement::Inside, px(0.0))),
        );
    let shape = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .gap_4()
                .child(studio.corner("radius_sm", ui.radius_sm, cx))
                .child(studio.corner("radius_chip", ui.radius_chip, cx))
                .child(studio.corner("radius", ui.radius, cx))
                .child(studio.corner("radius_lg", ui.radius_lg, cx))
                .child(studio.focus_ring(cx)),
        )
        .child(chip_family)
        .child(env.mark("focus.ring", rings));
    shape.into_any_element()
}

impl GalleryView {
    /// A corner radius metric as a sample box; clicking it selects the metric.
    pub(super) fn corner(
        &self,
        key: &'static str,
        radius: Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.metric_sample(key, radius, None, cx)
    }

    /// The focus ring (`border.focus`, `metrics.focus_ring_width`) round a control-shaped box.
    fn focus_ring(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        self.metric_sample("focus_ring_width", ui.radius, Some(ui.focus_ring_width), cx)
    }

    /// A metric's sample box, outlined in the border color, or with a `ring` that wide in
    /// the focus color; clicking it selects the metric.
    fn metric_sample(
        &self,
        key: &'static str,
        radius: Pixels,
        ring: Option<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ui = ui(cx);
        let name = format!("metrics.{key}");
        let selected = self.selected.as_deref() == Some(name.as_str());
        let (outline, color) = match ring {
            Some(width) => (UiColors { border_width: width, ..ui }, ui.focus),
            None => (ui, if selected { ui.primary } else { ui.border }),
        };
        div()
            .id(SharedString::from(format!("corner-{key}")))
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| this.select_token(&name, window, cx)))
            .child(div().size(px(56.0)).rounded(radius).outlined(outline, color).bg(ui.hover))
            .child(div().text_caption().text_color(if selected { ui.text } else { ui.muted }).child(key))
    }
}
