//! What the title row says besides the name: the Light | Dark control of a theme with both, a
//! small button when the theme it was copied from has changed (the rest of Update from is in the
//! theme menu), that it was made with a newer Wormhole, and that what the app shows is a preview
//! of a theme that is not in use.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::*;
use wormhole_core::theme::tokens::appearance::{DARK, LIGHT};
use wormhole_text::keys::{
    STUDIO_APPEARANCE_LABEL, STUDIO_NEWER_CHIP, STUDIO_PREVIEW_CHIP, STUDIO_PREVIEW_TOOLTIP,
    STUDIO_PREVIEW_USE, studio_newer_explain, studio_update_from,
};
use wormhole_text::text;

use super::StudioWindow;
use super::commit::appearance_word;
use super::document::places;
use super::update_from::{State, source_name};
use crate::settings::UiColors;
use crate::style;

impl StudioWindow {
    /// The extra marks of the title row, in order.
    pub(super) fn title_chips(&self, ui: UiColors, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut chips = Vec::new();
        if self.doc.is_pair() {
            chips.push(self.appearance_control(ui, cx));
        }
        if let Some(button) = self.update_button(ui, cx) {
            chips.push(button);
        }
        if self.doc.is_newer() {
            chips.push({
                // Why Studio won't save: what this version skipped of the file.
                let tip: SharedString =
                    studio_newer_explain(i64::try_from(self.doc.file.read.skipped).unwrap_or(i64::MAX))
                        .into();
                div()
                    .id("studio-newer")
                    .flex_none()
                    .child(style::status_badge(ui, ui.warning_text, text(STUDIO_NEWER_CHIP)))
                    .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                    .into_any_element()
            });
        }
        if !self.in_use(cx) {
            chips.push(self.preview_chip(ui, cx));
        }
        chips
    }

    /// Light | Dark: which appearance is being edited. Tab reaches it and the arrows switch.
    fn appearance_control(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let shown = self.doc.shown.as_deref();
        let segment = |name: &'static str| {
            let this = cx.weak_entity();
            style::segment(
                SharedString::from(format!("studio-appearance-{name}")),
                ui,
                appearance_word(name),
                shown == Some(name),
            )
            .focusable()
            .tab_stop(true)
            .focus_visible(move |d| d.bg(ui.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.set_shown(name, window, cx)))
            .on_key_down(move |event, window, cx| {
                let other = match event.keystroke.key.as_str() {
                    "left" => LIGHT,
                    "right" => DARK,
                    _ => return,
                };
                this.update(cx, |studio, cx| studio.set_shown(other, window, cx)).ok();
                cx.stop_propagation();
            })
        };
        div()
            .id("studio-appearance")
            .role(Role::RadioGroup)
            .aria_label(text(STUDIO_APPEARANCE_LABEL))
            .flex_none()
            .child(style::segmented(ui).child(segment(LIGHT)).child(segment(DARK)))
            .into_any_element()
    }

    /// A small button, with the source named in its tooltip, when the theme it was copied from has
    /// changed since. Everything else about Update from (up to date, the source gone) is in the
    /// theme menu: the title row has no room for words that say nothing is to be done.
    fn update_button(&self, ui: UiColors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let from = places::base_under(&self.doc.file, self.doc.shown.as_deref())?;
        let State::Changed(_) = self.update_check.state else { return None };
        Some(
            div()
                .flex_none()
                .child(
                    style::icon_button(
                        "studio-update-from",
                        ui,
                        IconName::RefreshCw,
                        studio_update_from(&source_name(&from.from)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.open_update_from(cx))),
                )
                .into_any_element(),
        )
    }

    /// "Preview" with Use: every window shows this theme while the Studio is open.
    fn preview_chip(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("studio-preview-chip")
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .child(style::status_badge(ui, ui.muted, text(STUDIO_PREVIEW_CHIP)))
            .child(
                Button::new("studio-preview-use")
                    .ghost()
                    .small()
                    .label(text(STUDIO_PREVIEW_USE))
                    .tooltip(text(STUDIO_PREVIEW_TOOLTIP))
                    .on_click(cx.listener(|this, _, window, cx| this.use_theme(window, cx))),
            )
            .into_any_element()
    }
}
