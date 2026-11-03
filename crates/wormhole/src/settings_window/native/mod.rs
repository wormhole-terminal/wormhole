//! The Settings window, drawn by SwiftUI (`macos/settings/`). The app keeps the settings: this module gives the
//! window a snapshot of them (`snapshot`) whenever they change, and carries out each change the person makes there
//! (`apply`) through `update_settings`, so the terminals, the file watcher, the Changed page and the profiles
//! behave as they always did.
//!
//! The window calls back on the main thread, outside any call of the app's; its events are queued and carried out
//! on the app's executor, bursts (a slider drag) coalesced to the last value of each key.
//!
//! Events, not polling (`repo-docs/docs/events.md`): the window is pushed a snapshot from an observer on the
//! settings global, so a change made anywhere (the window itself, a file edit the config watcher reads, devctl)
//! reaches it the same way, with no timer on either side and no file watching of this module's own.

mod accent;
mod actions;
mod agents_read;
mod apply;
mod cli_tool;
mod codec;
mod controls;
mod lists;
mod open_in;
mod paths;
mod preview;
mod recorder;
mod rows;
mod snapshot;
mod system;
mod themes;
mod update_words;
mod words;
mod words_table;

use std::cell::RefCell;
use std::collections::VecDeque;

use gpui_kit::{App, AsyncApp, Global, Subscription};

use super::Section;
use crate::fonts::FontCatalog;
use crate::platform::native_settings as platform;
use crate::settings::SettingsStore;

struct Native {
    /// The pairing the person chose in Appearance; until chosen it follows whether the picks are a pair.
    separate: Option<bool>,
    /// What the lists read from disk (agents, projects).
    lists: lists::Cache,
    /// The shortcut row that records, or the keys it refused.
    recording: Option<recorder::Recording>,
    /// Why the last theme import failed, until the next one.
    import_failure: Option<String>,
    /// The "New profile" sheet, while it is up.
    new_profile: Option<lists::new_profile::State>,
    /// The page the window was last asked to show.
    page: String,
    _subscriptions: Vec<Subscription>,
}

impl Global for Native {}

thread_local! {
    /// What the window said, until the executor gets to it.
    static EVENTS: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
    static ASYNC: RefCell<Option<AsyncApp>> = const { RefCell::new(None) };
    /// Where the next snapshot sends the window (`show`), once.
    static SHOW: RefCell<serde_json::Value> = const { RefCell::new(serde_json::Value::Null) };
}

/// Show the window (on `section`, else where it was), and keep it current while it is open.
pub(super) fn open(section: Option<Section>, cx: &mut App) {
    if cx.try_global::<Native>().is_none() {
        let subscriptions = vec![
            cx.observe_global::<SettingsStore>(push),
            cx.observe_global::<FontCatalog>(push),
            cx.observe_global::<crate::update::Updates>(push),
            // An approval that finishes later, a trust, a preference: the lists show it when it happens.
            crate::features::project::observe(cx, |cx| {
                lists::projects_changed(cx);
                push(cx);
            }),
        ];
        cx.set_global(Native {
            separate: None,
            lists: lists::Cache::default(),
            recording: None,
            import_failure: None,
            new_profile: None,
            page: String::new(),
            _subscriptions: subscriptions,
        });
    }
    let page = section.map(|s| format!("{s:?}")).unwrap_or_default();
    if !page.is_empty() {
        cx.global_mut::<Native>().page.clone_from(&page);
    }
    // Off macOS and in a test there is no window to fill.
    if !platform::available() {
        return;
    }
    FontCatalog::ensure_loaded(cx);
    // An app installed or removed since last time shows in the folder menu.
    crate::open_in::OpenIn::refresh(cx);
    ASYNC.with(|slot| *slot.borrow_mut() = Some(cx.to_async()));
    let behind = crate::background::keeps_new_windows_behind(cx);
    platform::open(&page, behind, on_event);
    crate::agent_access::forget_listing(cx);
    lists::load(cx);
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

/// The window closed: a recording ends, and a reopened window shows neither the sheet nor the import's failure.
fn closed(cx: &mut App) {
    recorder::stop(cx);
    if cx.has_global::<Native>() {
        let native = cx.global_mut::<Native>();
        native.new_profile = None;
        native.import_failure = None;
    }
}

pub(super) fn set_separate(separate: bool, cx: &mut App) {
    if cx.has_global::<Native>() {
        cx.global_mut::<Native>().separate = Some(separate);
    }
}

pub(super) fn separate(cx: &App) -> Option<bool> {
    cx.try_global::<Native>().and_then(|n| n.separate)
}

pub(super) fn set_import_failure(failure: Option<String>, cx: &mut App) {
    if cx.has_global::<Native>() {
        cx.global_mut::<Native>().import_failure = failure;
    }
}

/// The failure of the last theme import, in words.
pub(super) fn import_failure(cx: &App) -> Option<String> {
    let reason = cx.try_global::<Native>()?.import_failure.as_deref()?;
    Some(wormhole_text::keys::settings_window_import_failed(reason))
}

fn set_new_profile(state: Option<lists::new_profile::State>, cx: &mut App) {
    if cx.has_global::<Native>() {
        cx.global_mut::<Native>().new_profile = state;
    }
    push(cx);
}

fn new_profile_state(cx: &App) -> Option<&lists::new_profile::State> {
    cx.try_global::<Native>()?.new_profile.as_ref()
}

/// Open the window with the "New profile" sheet up (the File menu, the palette).
pub(super) fn open_new_profile(cx: &mut App) {
    open(Some(Section::Profile), cx);
    lists::new_profile::open(cx);
}

/// What the window is told to do besides showing the snapshot: go to a page, scroll to a row, search.
#[derive(Default)]
pub(super) struct Show {
    pub page: Option<String>,
    pub scroll: Option<String>,
    pub search: Option<String>,
}

/// Show the window and make it go where `show` says (the palette's setting, devctl): the snapshot `open` pushes
/// carries it.
pub(super) fn show(show: Show, cx: &mut App) {
    let show = serde_json::json!({ "page": show.page, "scroll": show.scroll, "search": show.search });
    SHOW.with(|slot| *slot.borrow_mut() = show);
    open(None, cx);
}

fn take_show() -> serde_json::Value {
    SHOW.with(|slot| slot.take())
}

/// The row of the window that stands for the setting `id`, if the window has one.
pub(super) fn row_of(id: &str) -> Option<&'static str> {
    rows::all().iter().find(|(_, row)| *row == id).map(|(key, _)| *key)
}

/// The page the window was last asked to show, once it was opened at all.
#[cfg(test)]
pub(super) fn page(cx: &App) -> Option<String> {
    cx.try_global::<Native>().map(|native| native.page.clone())
}
