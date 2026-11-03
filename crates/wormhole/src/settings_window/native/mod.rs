//! The Settings window drawn by SwiftUI (`macos/settings/`), behind the `native_settings` flag. The app keeps the
//! settings: this module gives the window a snapshot of them (`snapshot`) whenever they change, and carries out
//! each change the person makes there (`apply`) through the same `update_settings` the GPUI window uses, so the
//! terminals, the file watcher, the Changed page and the profiles behave as they always did.
//!
//! The window calls back on the main thread, outside any call of the app's; its events are queued and carried out
//! on the app's executor, bursts (a slider drag) coalesced to the last value of each key.
//!
//! Events, not polling (`repo-docs/docs/events.md`): the window is pushed a snapshot from an observer on the
//! settings global, so a change made anywhere (the window itself, a file edit the config watcher reads, devctl)
//! reaches it the same way, with no timer on either side and no file watching of this module's own.

mod actions;
mod apply;
mod codec;
mod rows;
mod snapshot;
mod themes;

use std::cell::RefCell;
use std::collections::VecDeque;

use gpui_kit::{App, AsyncApp, Global, Subscription};
use wormhole_core::config::features::FeatureFlag;

use super::Section;
use crate::fonts::FontCatalog;
use crate::platform::native_settings as platform;
use crate::settings::{SettingsStore, settings};

struct Native {
    /// The pairing the person chose in Appearance; until chosen it follows whether the picks are a pair.
    separate: Option<bool>,
    _subscriptions: Vec<Subscription>,
}

impl Global for Native {}

thread_local! {
    /// What the window said, until the executor gets to it.
    static EVENTS: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
    static ASYNC: RefCell<Option<AsyncApp>> = const { RefCell::new(None) };
}

/// Whether Settings opens as the native window.
pub(super) fn enabled(cx: &App) -> bool {
    platform::available() && settings(cx).feature_enabled(FeatureFlag::NativeSettings)
}

/// Show the window (on `section`, else where it was), and keep it current while it is open.
pub(super) fn open(section: Option<Section>, cx: &mut App) {
    FontCatalog::ensure_loaded(cx);
    if cx.try_global::<Native>().is_none() {
        let subscriptions =
            vec![cx.observe_global::<SettingsStore>(push), cx.observe_global::<FontCatalog>(push)];
        cx.set_global(Native { separate: None, _subscriptions: subscriptions });
    }
    ASYNC.with(|slot| *slot.borrow_mut() = Some(cx.to_async()));
    let behind = crate::background::keeps_new_windows_behind(cx);
    let page = section.map(|s| format!("{s:?}")).unwrap_or_default();
    platform::open(&page, behind, on_event);
    push(cx);
}

/// The window's callback: queue the event and have the executor carry it out.
fn on_event(event: &str) {
    EVENTS.with(|queue| queue.borrow_mut().push_back(event.to_string()));
    ASYNC.with(|slot| {
        if let Some(cx) = slot.borrow().as_ref() {
            cx.spawn(async move |cx| cx.update(drain)).detach();
        }
    });
}

fn drain(cx: &mut App) {
    let queued: Vec<String> = EVENTS.with(|queue| queue.borrow_mut().drain(..).collect());
    // Of several sets of one key in a burst only the last is written.
    let key_of = |event: &str| {
        let value: serde_json::Value = serde_json::from_str(event).ok()?;
        (value["t"] == "set").then(|| value["k"].as_str().unwrap_or_default().to_string())
    };
    let keys: Vec<Option<String>> = queued.iter().map(|e| key_of(e)).collect();
    for (at, event) in queued.iter().enumerate() {
        let superseded =
            keys[at].as_ref().is_some_and(|k| keys[at + 1..].iter().any(|later| later.as_ref() == Some(k)));
        if !superseded {
            apply::event(event, cx);
        }
    }
}

/// Carry out one event now (devctl's stand-in for the window).
#[cfg(feature = "devctl")]
pub(super) fn event(event: &str, cx: &mut App) {
    apply::event(event, cx);
}

/// Give the window the settings as they are now.
pub(super) fn push(cx: &mut App) {
    if !platform::is_open() {
        return;
    }
    let separate = cx.try_global::<Native>().and_then(|n| n.separate);
    let snapshot = snapshot::build(separate, cx);
    platform::update(&snapshot.to_string());
}

pub(super) fn set_separate(separate: bool, cx: &mut App) {
    if cx.has_global::<Native>() {
        cx.global_mut::<Native>().separate = Some(separate);
    }
}

pub(super) fn separate(cx: &App) -> Option<bool> {
    cx.try_global::<Native>().and_then(|n| n.separate)
}
