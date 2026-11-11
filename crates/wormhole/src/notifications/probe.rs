//! A notification a Probe's hook asked for (`probe::active::Effect::Notify`).
//!
//! The words are the hook's (planned from a template of fixed words and cleaned fields, and capped); the
//! subtitle is always the Probe's name, so a banner cannot pass for one of Wormhole's own. One at a time
//! per Probe: a newer one replaces the older. It goes through the same route as every other banner, so the
//! system's permission and Focus apply, and a hook's rate is limited twice: by its budget, and here by the
//! route's own replacing.

use gpui_kit::*;
use wormhole_core::notify::Level;
use wormhole_core::probe::hidden::reveal;

use super::{permission, post};
use crate::platform::notifications::{Category, Notice, Permission};

/// Tell the user what `probe` (named `name`) said. Nothing is posted while macOS has notifications off.
pub(crate) fn probe_notice(probe: &str, name: &str, title: &str, body: &str, cx: &mut App) {
    if !matches!(permission(cx), Permission::Allowed { .. } | Permission::OutsideBundle) {
        return;
    }
    let id = format!("wormhole/probe/{probe}");
    let notice = Notice {
        thread: id.clone(),
        id,
        title: title.to_string(),
        subtitle: format!("{} (extension)", reveal(name)),
        body: body.to_string(),
        level: Level::Active,
        sound: false,
        category: Category::Plain,
    };
    if let Some(why) = post(notice, cx) {
        tracing::warn!(probe, "a Probe's notification was not shown: {why}");
    }
}
