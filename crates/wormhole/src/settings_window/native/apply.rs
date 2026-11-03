//! One event of the native window, carried out.

use gpui_kit::App;
use serde_json::Value;
use wormhole_core::config::Settings;
use wormhole_core::config::settings_index::{self};

use super::codec::{self, Ctx};
use super::{actions, push, recorder, rows, themes};
use crate::settings::{SettingsStore, update_settings};

pub(super) fn event(text: &str, cx: &mut App) {
    let Ok(Value::Object(event)) = serde_json::from_str::<Value>(text) else {
        tracing::warn!("native settings: an event that is not JSON");
        return;
    };
    let field = |name: &str| event.get(name).and_then(Value::as_str).unwrap_or_default().to_string();
    match field("t").as_str() {
        "set" => set(&field("k"), &field("v"), cx),
        "reset" => reset(&field("k"), cx),
        "reset_all" => update_settings(cx, |s| *s = Settings::default()),
        "action" => actions::perform(&field("k"), &field("v"), cx),
        "record" => match field("a").as_str() {
            "start" => recorder::start(&field("k"), cx),
            "stop" => recorder::stop(cx),
            _ => recorder::key(&field("m"), &field("n"), cx),
        },
        "closed" => recorder::stop(cx),
        other => tracing::warn!("native settings: unknown event {other}"),
    }
    // The window shows what the person set at once; this puts back what the app made of it.
    push(cx);
}

fn set(key: &str, value: &str, cx: &mut App) {
    let base = rows::base(key);
    let (suffix, Some(id)) = (&key[base.len()..], rows::id_of(base)) else { return };
    let entries = themes::entries(cx);
    if id == "theme-pairing" {
        super::set_separate(value != codec::PAIRING_BOTH, cx);
    }
    let ctx = Ctx { themes: &entries, separate: super::separate(cx) };
    let (suffix, value) = (suffix.to_string(), value.to_string());
    let mut understood = false;
    update_settings(cx, |s| understood = codec::write(id, &suffix, s, &value, &ctx));
    if !understood {
        tracing::debug!("native settings: {key} = {value:?} was not understood");
    }
}

fn reset(key: &str, cx: &mut App) {
    let Some(id) = rows::id_of(rows::base(key)) else { return };
    let Some((_, _, entry)) = settings_index::entry(id) else { return };
    let keys = entry.keys;
    if keys.is_empty() {
        return;
    }
    let user = cx.global::<SettingsStore>().user_settings().clone();
    let reset = settings_index::reset(&user, keys);
    update_settings(cx, |s| *s = reset);
}
