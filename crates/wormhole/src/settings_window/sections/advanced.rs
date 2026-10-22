//! Advanced: what is wrong in config.json, the files and folders, and a way back to the
//! defaults that asks first.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use wormhole_core::config::settings_index::Values;
use wormhole_core::storage::paths::data_dir;

use super::{open_button, open_in_editor, reveal};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore};
use crate::style;
use crate::workspace::app::{app, app_mut};
use crate::workspace::settings_page::registry::Setting;
use crate::workspace::settings_page::{SettingsWindow, ui};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("open-config", |_, _| {
        open_button("open-config", IconName::CodeXml, "Open", |_| {
            open_in_editor(&SettingsStore::config_path())
        })
    }),
    Setting::custom("open-data", |_, _| {
        open_button("open-data", IconName::FolderOpen, "Show in Finder", |cx| reveal(&data_dir(), cx))
    }),
    Setting::custom("open-logs", |_, _| {
        open_button("open-logs", IconName::FolderOpen, "Show in Finder", |cx| {
            reveal(&data_dir().join("logs"), cx)
        })
    }),
    Setting::custom("reset-settings", draw_reset).below(|w, cx| w.render_reset_question(cx)),
    Setting::custom("reset-tips", draw_reset_tips),
];

/// Reset tips: the tips closed with "Don't show again" show again where they belong.
fn draw_reset_tips(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let none = app(cx).tips.is_empty();
    Button::new("reset-tips")
        .outline()
        .small()
        .icon(IconName::RotateCcw)
        .label("Reset tips")
        .disabled(none)
        .tooltip(if none { "No tip was closed for good" } else { "They show again where they belong" })
        .on_click(|_, _, cx| {
            app_mut(cx).reset_tips();
            cx.refresh_windows();
        })
        .into_any_element()
}

/// The outlined danger button; pressed, the question in place under the row.
fn draw_reset(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let changed = Values::of(cx.global::<SettingsStore>().user_settings()).changed_keys().len();
    Button::new("reset-settings")
        .danger()
        .outline()
        .small()
        .label("Reset to defaults…")
        .disabled(changed == 0 || window.confirm_reset)
        .tooltip(if changed == 0 { "Every setting is at its default" } else { "Asks first" })
        .on_click(cx.listener(|this, _, _, cx| {
            this.confirm_reset = true;
            cx.notify();
        }))
        .into_any_element()
}

impl SettingsWindow {
    /// The reset's question, in place under its row once "Reset to defaults…" was pressed.
    fn render_reset_question(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.confirm_reset {
            return None;
        }
        let ui = ui(cx);
        let changed = Values::of(cx.global::<SettingsStore>().user_settings()).changed_keys().len();
        Some(
            style::banner(
                ui,
                style::BannerKind::Danger,
                false,
                format!(
                    "Reset {changed} changed {} to {} default? Your terminals, notes and themes are kept.",
                    if changed == 1 { "setting" } else { "settings" },
                    if changed == 1 { "its" } else { "their" },
                ),
            )
            .child(Button::new("reset-cancel").ghost().small().label("Cancel").on_click(cx.listener(
                |this, _, _, cx| {
                    this.confirm_reset = false;
                    cx.notify();
                },
            )))
            .child(Button::new("reset-confirm").danger().small().label("Reset").on_click(cx.listener(
                |this, _, _, cx| {
                    this.confirm_reset = false;
                    update_settings(cx, |s| *s = Settings::default());
                },
            )))
            .into_any_element(),
        )
    }
}
