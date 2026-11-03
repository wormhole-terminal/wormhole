//! The Feature flags page: a switch for each flag of `FeatureFlag::ALL`, listed here so the page always has exactly
//! the build's flags (a catalog row per flag, keyed by its place, went wrong as soon as a flag was added).

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::config::features::{self, FeatureFlag, FlagMode};

use super::Entry;
use crate::settings::{SettingsStore, update_settings};

const GROUP: &str = "FeatureFlags:0"; // text: not shown (a key or page id of the native window, or a log line)

pub(super) fn lists(out: &mut Map<String, Value>, cx: &App) {
    let s = cx.global::<SettingsStore>().user_settings();
    let mode = FlagMode::of(features::dev_build(), s.update_channel, features::unlocked(&s.feature_flags));
    let rows = FeatureFlag::ALL.iter().map(|flag| {
        let on = features::enabled_in(*flag, &s.feature_flags, mode);
        Entry::new(flag.key(), "toggle", &super::super::words::word(flag.title()))
            .with("value", if on { "on" } else { "off" })
            .with("info", flag.description())
            .into_value()
    });
    out.insert(GROUP.into(), Value::Array(rows.collect()));
}

/// A flag's switch: the row is `FeatureFlags:0:<flag key>`, the action `on` or `off`.
pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let Some(flag) =
        key.strip_prefix(GROUP).and_then(|id| id.strip_prefix(':')).and_then(FeatureFlag::from_key)
    else {
        return false;
    };
    update_settings(cx, |s| {
        s.feature_flags.insert(flag.key().to_string(), action == "on");
    });
    true
}
