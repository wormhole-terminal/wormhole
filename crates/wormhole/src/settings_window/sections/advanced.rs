//! Advanced: what is wrong in config.json, the files and folders, and a way back to the
//! defaults that asks first.

use gpui_kit::component::Disableable as _;
use gpui_kit::*;
use wormhole_core::config::settings_index::Values;
use wormhole_core::storage::paths::data_dir;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{open_button, open_in_editor, reveal};
use crate::app_state::{app, app_mut};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore};
use crate::settings_window::SettingsWindow;
use crate::settings_window::registry::Setting;
use crate::style;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("open-config", |_, _| {
        open_button("open-config", text(SETTINGS_ADVANCED_OPEN), |_| {
            open_in_editor(&SettingsStore::config_path())
        })
    }),
    Setting::custom("open-data", |_, _| {
        open_button("open-data", text(SETTINGS_ADVANCED_SHOW_IN_FINDER), |cx| reveal(&data_dir(), cx))
    }),
    Setting::custom("open-logs", |_, _| {
        open_button("open-logs", text(SETTINGS_ADVANCED_SHOW_IN_FINDER), |cx| {
            reveal(&data_dir().join("logs"), cx)
        })
    }),
    Setting::custom("reset-settings", draw_reset),
    Setting::custom("reset-tips", draw_reset_tips),
    Setting::custom("skip-scans", |_, _| {
        open_button(
            "skip-scans",
            text(SETTINGS_ADVANCED_OPEN_DEVELOPER_TOOLS),
            crate::platform::dev_tools::open_settings,
        )
    }),
];

/// Reset tips: the tips closed with "Don't show again" show again where they belong.
fn draw_reset_tips(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let none = app(cx).tips.is_empty();
    style::form_button("reset-tips")
        .label(text(SETTINGS_ADVANCED_RESET_TIPS_BUTTON))
        .disabled(none)
        .tooltip(if none {
            text(SETTINGS_ADVANCED_NO_TIP_WAS_CLOSED_FOR_GOOD)
        } else {
            text(SETTINGS_ADVANCED_THEY_SHOW_AGAIN_WHERE_THEY_BELONG)
        })
        .on_click(|_, _, cx| {
            app_mut(cx).reset_tips();
            cx.refresh_windows();
        })
        .into_any_element()
}

/// The outlined danger button; pressed, the system's alert asks (`changed::ask_reset`).
fn draw_reset(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let changed = changed_keys(cx);
    style::form_button("reset-settings")
        .danger()
        .label(text(SETTINGS_ADVANCED_RESET_TO_DEFAULTS))
        .disabled(changed == 0)
        .tooltip(if changed == 0 {
            text(SETTINGS_ADVANCED_EVERY_SETTING_IS_AT_ITS_DEFAULT)
        } else {
            text(SETTINGS_ADVANCED_ASKS_FIRST)
        })
        .on_click(cx.listener(|this, _, window, cx| this.ask_reset_all(window, cx)))
        .into_any_element()
}

/// How many settings differ from their defaults, counted as `config.json` has them (an object of choices is
/// one), which is what "Reset to defaults" puts back.
fn changed_keys(cx: &App) -> usize {
    Values::of(cx.global::<SettingsStore>().user_settings()).changed_keys().len()
}

impl SettingsWindow {
    /// Ask before every setting goes back to its default, then reset them (the terminals, notes and themes
    /// are not settings and stay).
    pub(in crate::settings_window) fn ask_reset_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let count = changed_keys(cx);
        if count > 0 {
            crate::settings_window::changed::ask_reset(window, cx, count, |cx| {
                update_settings(cx, |s| *s = Settings::default());
            });
        }
    }
}
