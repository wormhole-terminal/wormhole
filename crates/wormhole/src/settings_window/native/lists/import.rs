//! Import: the terminals found on this Mac, each with a button that brings over what the GPUI page ticks to
//! start with (the rows `default_on` says; a shell command is never among them) after a question that lists it.
//! The scan reads the real home, so it runs in the background when the window opens and never at launch.

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::import::{Found, Proposed, Setting, visible};
use wormhole_text::keys::*;
use wormhole_text::text;

use super::Entry;
use crate::settings::import::{self as shared, terminal_name};
use crate::settings_window::sections::{setting_label, value_text};
use crate::settings_window::with_tilde;

const GROUP: &str = "Import:0";

fn pair(found: &Found) -> bool {
    let has = |wanted: Setting| found.proposal.values.iter().any(|p| p.setting == wanted);
    has(Setting::Theme) && has(Setting::ThemeLight)
}

fn defaults(found: &Found) -> Vec<Proposed> {
    found.proposal.values.iter().filter(|proposed| proposed.default_on).cloned().collect()
}

pub(super) fn lists(out: &mut Map<String, Value>, cx: &App) {
    let mut list = Vec::new();
    match super::cache(cx).and_then(|cache| cache.import.as_ref()) {
        None => list.push(Entry::new("scanning", "value", text(SETTINGS_IMPORT_SCANNING)).into_value()),
        Some(all) if all.is_empty() => {
            list.push(
                Entry::new("none", "value", text(SETTINGS_IMPORT_NONE))
                    .with("info", text(SETTINGS_IMPORT_NONE_DETAIL))
                    .into_value(),
            );
        }
        Some(all) => {
            for found in all {
                let name = terminal_name(found.source);
                let pair = pair(found);
                let rows: Vec<String> = defaults(found)
                    .iter()
                    .map(|p| format!("{}: {}", setting_label(p.setting, pair), value_text(&p.value)))
                    .collect();
                let mut entry = Entry::new(found.source.key(), "button", name)
                    .with(
                        "hint",
                        found
                            .files
                            .first()
                            .map(|path| visible(&with_tilde(path)).to_string())
                            .unwrap_or_default(),
                    )
                    .with("chip", settings_import_count(wormhole_text::whole(found.proposal.values.len())))
                    .with("value", text(SETTINGS_IMPORT_REVIEW))
                    .with("tile", "#30b0c7")
                    .with("icon", "prompt");
                if !rows.is_empty() {
                    entry = entry
                        .with("confirm", format!("{name}: {}", text(SETTINGS_IMPORT_TAKE)))
                        .with("confirmText", rows.join("\n"))
                        .with("action", text(SETTINGS_IMPORT_APPLY));
                }
                list.push(entry.into_value());
            }
        }
    }
    out.insert(GROUP.into(), Value::Array(list));
}

pub(super) fn load(cx: &mut App) {
    cx.spawn(async move |cx| {
        let found = cx.background_executor().spawn(async { shared::scan_here() }).await;
        cx.update(|cx| {
            if cx.has_global::<crate::settings_window::native::Native>() {
                super::cache_mut(cx).import = Some(found);
                crate::settings_window::native::push(cx);
            }
        })
    })
    .detach();
}

pub(super) fn perform(key: &str, name: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some("Import"), Some(_), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if name != text(SETTINGS_IMPORT_APPLY) {
        return true;
    }
    let found = super::cache(cx)
        .and_then(|cache| cache.import.as_ref())
        .and_then(|all| all.iter().find(|f| f.source.key() == id))
        .cloned();
    if let Some(found) = found {
        let chosen = defaults(&found);
        if let Err(why) = shared::apply(cx, &found, &chosen) {
            tracing::warn!("importing from {}: {why}", found.source.key());
        }
    }
    true
}
