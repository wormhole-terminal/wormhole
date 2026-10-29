//! What the Studio shows over its content when a list is part of the question (the values an
//! update would change, the earlier saves of a theme): a card on a scrim that takes the clicks,
//! with Return and Esc answering it. A native sheet cannot list them. One is open at a time.

use gpui_kit::*;

use super::update_from::Sheet;
use super::versions::Panel;
use super::{StudioWindow, ui};
use crate::settings::UiColors;
use crate::style::{Edges as _, TextRole as _};

/// The card's width.
const CARD_W: f32 = 520.0;

pub(super) enum Modal {
    Update(Sheet),
    Versions(Panel),
}

/// A titled card: `title`, the `body`, and the `footer` with its buttons.
pub(super) fn card(ui: UiColors, title: String, body: Div, footer: Div) -> AnyElement {
    div()
        .w(px(CARD_W))
        .max_h_full()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded(ui.radius_lg)
        .bg(ui.elevated)
        .outlined(ui, ui.border)
        .child(div().text_heading().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(body)
        .child(footer)
        .into_any_element()
}

impl StudioWindow {
    pub(super) fn close_modal(&mut self, cx: &mut Context<Self>) {
        if self.modal.take().is_some() {
            // What a version was previewed as is put back.
            self.preview_file(&self.doc.file, cx);
            cx.notify();
        }
    }

    /// The open sheet or panel over the window.
    pub(super) fn render_modal(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ui = ui(cx);
        let content = match self.modal.as_ref()? {
            Modal::Update(sheet) => self.render_update_sheet(sheet, ui, cx),
            Modal::Versions(panel) => self.render_versions(panel, ui, cx),
        };
        Some(
            crate::window::overlay::scrim("studio-modal", cx)
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx)))
                .child(div().id("studio-modal-card").occlude().child(content))
                .into_any_element(),
        )
    }
}
