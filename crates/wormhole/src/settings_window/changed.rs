//! The Changed page: every setting that differs from its default, each in the control of its own page and
//! under the name of that page, with a reset arrow of its own. Over them a card says how many differ, with
//! Copy as JSON (what differs, as a snippet for `config.json`) and Reset All…, which asks first.
//!
//! The rows are the registry's: the one renderer draws them here and on their pages, with their dot, key and
//! right-click menu, so a change made here is the change made there. Which rows differ is core's
//! (`Values::changed_rows`, which leaves out a hidden page and a choice behind a switch that is off); this
//! page only lays them out.

use std::time::Duration;

use crate::style::Button;
use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, Values};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::registry::{PageState, control};
use super::{SettingsWindow, sections};
use crate::platform::{a11y, clipboard};
use crate::settings::{SettingsStore, UiColors, settings, update_settings};
use crate::style::{self, TextRole as _};

/// How long Copy as JSON says Copied.
const COPIED_FOR: Duration = Duration::from_millis(1500);

/// How many settings differ, for the nav's badge: the rows the page lists.
pub(super) fn differing(cx: &App) -> usize {
    let store = cx.global::<SettingsStore>();
    Values::of(store.user_settings()).changed_rows(&store.settings).len()
}

impl SettingsWindow {
    /// The page: the summary card with its two buttons (and the reset's question under it), then the rows of
    /// each page that has any, under that page's name.
    pub(super) fn render_changed(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = PageState::for_changed_page(&self.front, cx);
        let ui = page.ui;
        let rows = page.values.changed_rows(settings(cx));
        let mut column = div().flex().flex_col().gap_6().child(self.changed_summary(ui, rows.len(), cx));
        // The rows come in the order of the pages, so one page's rows are next to each other.
        for (at, run) in rows.chunk_by(|a, b| a.section == b.section).enumerate() {
            let mut list = Vec::new();
            for row in run {
                let Some(setting) = control(row.entry.id) else {
                    tracing::warn!("settings: no control for {}", row.entry.id);
                    continue;
                };
                let entry = self.render_entry(row.entry, setting, &page, None, cx);
                list.push(match setting.below.and_then(|below| below(self, cx)) {
                    Some(below) => div()
                        .flex()
                        .flex_col()
                        .child(entry)
                        .child(div().pb_2().child(below))
                        .into_any_element(),
                    None => entry,
                });
            }
            let title = run[0].section.title();
            column = column.child(
                div()
                    // A group named for the page, so VoiceOver reads the rows as that page's.
                    .id(("changed-page", at))
                    .role(Role::Group)
                    .aria_label(title)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(style::setting_group_head(ui, title, None))
                    .child(style::setting_group(ui, list)),
            );
        }
        column.into_any_element()
    }

    /// "7 settings changed" and what is left ("Everything else is at its default"), with the two buttons that
    /// act on all of them; with none, the card says so and has no buttons. Pressed, Reset All… puts its
    /// question under the card.
    fn changed_summary(&self, ui: UiColors, count: usize, cx: &mut Context<Self>) -> AnyElement {
        let (title, detail): (SharedString, &str) = if count == 0 {
            (text(SETTINGS_CHANGED_NONE).into(), text(SETTINGS_CHANGED_NONE_DETAIL))
        } else {
            (settings_changed_summary(whole(count)).into(), text(SETTINGS_CHANGED_SUMMARY_DETAIL))
        };
        let tile = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(style::TILE))
            .rounded(ui.radius)
            .bg(ui.selected_row)
            .child(style::icon(IconName::RotateCcw).text_color(ui.selected_text));
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(title.clone()))
            .child(div().text_caption().text_color(ui.muted).child(detail));
        let buttons = div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("changed-copy")
                    .outline()
                    .small()
                    .icon(IconName::Copy)
                    .label(if self.changed_copied.is_some() {
                        text(SETTINGS_ROW_COPIED)
                    } else {
                        text(SETTINGS_ROW_COPY_JSON)
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.copy_changed(cx))),
            )
            .child(
                Button::new("changed-reset-all")
                    .danger()
                    .outline()
                    .small()
                    .label(text(SETTINGS_CHANGED_RESET_ALL))
                    .disabled(self.confirm_reset)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_reset = true;
                        cx.notify();
                    })),
            );
        let summary = div()
            .flex()
            .items_center()
            .gap_3()
            .py_3()
            .child(tile)
            .child(words)
            .when(count > 0, |d| d.child(buttons));
        let card = style::setting_group(ui, [summary.into_any_element()])
            .id("changed-summary")
            .role(Role::Group)
            .aria_label(title);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(card)
            .children(self.changed_question(ui, count, cx))
            .into_any_element()
    }

    /// The question under the card once Reset All… was pressed: Reset puts back the settings the page lists
    /// as they are when the answer comes, not when it was asked, and nothing else.
    fn changed_question(&self, ui: UiColors, count: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.confirm_reset || count == 0 {
            return None;
        }
        Some(
            sections::reset_question(
                ui,
                count,
                cx.listener(|this, _, _, cx| {
                    this.confirm_reset = false;
                    cx.notify();
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_reset = false;
                    let store = cx.global::<SettingsStore>();
                    let values = Values::of(store.user_settings());
                    let rows = values.changed_rows(&store.settings);
                    let keys = values.changed_keys_of(&rows);
                    update_settings(cx, |s| *s = settings_index::reset(s, &keys));
                }),
            )
            .into_any_element(),
        )
    }

    /// Copy what differs, as the snippet a person pastes into `config.json`, and say so for a moment.
    fn copy_changed(&mut self, cx: &mut Context<Self>) {
        let store = cx.global::<SettingsStore>();
        let values = Values::of(store.user_settings());
        let rows = values.changed_rows(&store.settings);
        clipboard::write(values.json(&values.changed_keys_of(&rows)), cx);
        a11y::announce(text(SETTINGS_ROW_COPIED), false);
        // A second copy replaces the first one's timer (dropping a task cancels it).
        self.changed_copied = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.changed_copied = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }
}
