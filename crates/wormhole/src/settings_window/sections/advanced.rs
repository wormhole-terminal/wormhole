//! Advanced: what is wrong in config.json, the files and folders, and a way back to the
//! defaults that asks first.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use wormhole_core::config::settings_index::Values;
use wormhole_core::storage::paths::data_dir;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::{open_button, open_in_editor, reveal};
use crate::app_state::{app, app_mut};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore};
use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("open-config", |w, cx| {
        open_button(w, cx, "open-config", IconName::CodeXml, text(SETTINGS_ADVANCED_OPEN), |_| {
            open_in_editor(&SettingsStore::config_path())
        })
    }),
    Setting::custom("open-data", |w, cx| {
        open_button(w, cx, "open-data", IconName::FolderOpen, text(SETTINGS_ADVANCED_SHOW_IN_FINDER), |cx| {
            reveal(&data_dir(), cx)
        })
    }),
    Setting::custom("open-logs", |w, cx| {
        open_button(w, cx, "open-logs", IconName::FolderOpen, text(SETTINGS_ADVANCED_SHOW_IN_FINDER), |cx| {
            reveal(&data_dir().join("logs"), cx)
        })
    }),
    Setting::custom("reset-settings", draw_reset).below(|w, cx| w.render_reset_question(cx)),
    Setting::custom("reset-tips", draw_reset_tips),
];

/// Reset tips: the tips closed with "Don't show again" show again where they belong.
fn draw_reset_tips(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let none = app(cx).tips.is_empty();
    let tooltip = if none {
        text(SETTINGS_ADVANCED_NO_TIP_WAS_CLOSED_FOR_GOOD)
    } else {
        text(SETTINGS_ADVANCED_THEY_SHOW_AGAIN_WHERE_THEY_BELONG)
    };
    if let Some(button) = window.native_button_with_tooltip(
        "reset-tips",
        text(SETTINGS_ADVANCED_RESET_TIPS_BUTTON),
        !none,
        false,
        Some(tooltip.into()),
        cx,
        |_, cx| {
            if !app(cx).tips.is_empty() {
                app_mut(cx).reset_tips();
                cx.refresh_windows();
            }
        },
    ) {
        return button;
    }
    Button::new("reset-tips")
        .outline()
        .small()
        .icon(IconName::RotateCcw)
        .label(text(SETTINGS_ADVANCED_RESET_TIPS_BUTTON))
        .disabled(none)
        .tooltip(tooltip)
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
        .label(text(SETTINGS_ADVANCED_RESET_TO_DEFAULTS))
        .disabled(changed == 0 || window.confirm_reset)
        .tooltip(if changed == 0 {
            text(SETTINGS_ADVANCED_EVERY_SETTING_IS_AT_ITS_DEFAULT)
        } else {
            text(SETTINGS_ADVANCED_ASKS_FIRST)
        })
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
        let this = cx.weak_entity();
        let cancel = self
            .native_button("reset-cancel", text(SETTINGS_ADVANCED_CANCEL), true, cx, move |_, cx| {
                this.update(cx, |this, cx| {
                    this.confirm_reset = false;
                    cx.notify();
                })
                .ok();
            })
            .unwrap_or_else(|| {
                Button::new("reset-cancel")
                    .ghost()
                    .small()
                    .label(text(SETTINGS_ADVANCED_CANCEL))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_reset = false;
                        cx.notify();
                    }))
                    .into_any_element()
            });
        let this = cx.weak_entity();
        let confirm = self
            .native_button_with_options(
                "reset-confirm",
                text(SETTINGS_ADVANCED_RESET),
                true,
                false,
                true,
                None,
                cx,
                move |_, cx| {
                    this.update(cx, |this, cx| {
                        if this.confirm_reset {
                            this.confirm_reset = false;
                            update_settings(cx, |s| *s = Settings::default());
                        }
                    })
                    .ok();
                },
            )
            .unwrap_or_else(|| {
                Button::new("reset-confirm")
                    .danger()
                    .small()
                    .label(text(SETTINGS_ADVANCED_RESET))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_reset = false;
                        update_settings(cx, |s| *s = Settings::default());
                    }))
                    .into_any_element()
            });
        Some(
            style::banner(
                ui,
                style::BannerKind::Danger,
                false,
                settings_advanced_reset_question(whole(changed)),
            )
            .child(cancel)
            .child(confirm)
            .into_any_element(),
        )
    }
}
