//! Import: the terminals found on this Mac, each with a button that brings over what is ticked to
//! start with (the rows `default_on` says; a shell command is never among them) after a question that lists it.
//! The scan reads the real home, so it runs in the background when the window opens and never at launch.

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::import::{Proposed, visible};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{Entry, import_sheets};
use crate::settings::import::{self as shared, terminal_name};
use crate::settings_window::native::paths::with_tilde;

/// The page this list belongs to.
const PAGE: &str = "Import"; // text: not shown (a page id of the native window)
const GROUP: &str = "Import:0"; // text: not shown (a key or page id of the native window, or a log line)

pub(super) fn lists(out: &mut Map<String, Value>, sheets: &mut Map<String, Value>, cx: &App) {
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
                sheets.insert(import_sheets::review_id(found), import_sheets::review(found));
                let entry = Entry::new(found.source.key(), "review", name)
                    .with("sheet", import_sheets::review_id(found))
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
                list.push(entry.into_value());
            }
        }
    }
    // The page's own "Compare with Wormhole" row opens this one.
    if let Some(all) = super::cache(cx).and_then(|cache| cache.import.as_ref()) {
        sheets.insert(import_sheets::COMPARE.into(), import_sheets::compare(all));
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

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some(PAGE), Some(_), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    // "apply=<the places in the proposal left ticked>"
    let Some(ticked) = action.strip_prefix("apply=") else { return true };
    let found = super::cache(cx)
        .and_then(|cache| cache.import.as_ref())
        .and_then(|all| all.iter().find(|f| f.source.key() == id))
        .cloned();
    if let Some(found) = found {
        let places: Vec<usize> = ticked.split(',').filter_map(|at| at.parse().ok()).collect();
        let chosen: Vec<Proposed> = found
            .proposal
            .values
            .iter()
            .enumerate()
            .filter(|(at, _)| places.contains(at))
            .map(|(_, proposed)| proposed.clone())
            .collect();
        if let Err(why) = shared::apply(cx, &found, &chosen) {
            tracing::warn!("importing from {}: {why}", found.source.key());
        }
    }
    true
}
