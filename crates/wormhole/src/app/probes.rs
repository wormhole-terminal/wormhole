//! Running the Probes that are on: loading them, listening to the event bus for their hooks, and carrying
//! out what the hooks decided (`wormhole_core::probe::active`).
//!
//! The Probes are read off the main thread from their sealed copies (`active::load`), and read again when
//! something says they may have changed: the user approved or turned one off (`probe_runtime`'s epoch,
//! heard as an event), the `probes` flag was switched, or, as a slow backstop for a Probe's folder edited
//! behind the app's back, every [`BACKSTOP`] ticks while any is on.
//!
//! Hooks hear the bus, not the screen. One thread (`wormhole-probe-hooks`, core's `ipc::probe_feed`) waits on a subscription of its
//! own, for exactly the topics some hook listens to, and hands each event over a bounded channel to a task
//! on the UI executor, which plans it (pure, microseconds, under each hook's budget) and carries out the
//! effects (`probe_effects`): a notification, a status item, a tint on a block, a file offered, a program
//! started. Nothing runs in a frame, no hook can block, rewrite or answer anything a terminal does, and a
//! hook that fails is a hook that did nothing. With the flag off, or no Probe with a hook on, there is no
//! subscription and the bus is told nothing extra.
//!
//! What a terminal prints reaches `output.match` hooks the way it reaches `wh events --output`: the bus's own
//! worker reads the screen off the PTY's thread and tells the finished lines, to a subscriber that may read
//! the terminals. Only a Probe the user let read them (`terminal = ["read"]`) makes the feed one, and the
//! dispatcher still hands each Probe only what its own grant allows. A terminal the user hid from agents is
//! as hidden from the feed as from any other.
//!
//! Part of the app root: it may use every surface, and nothing below it calls up into it.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::probe_feed::{self, Feed, Listener, TabFacts};
use wormhole_core::probe::active::{self, Dispatcher};
use wormhole_core::storage::paths::DataRoot;

use super::probe_effects::carry_out;
use crate::app_state::{AppState, app};
use crate::probe_runtime::{self, RuleSet};
use crate::settings::{SettingsStore, settings};
use crate::window::ambient;

/// Ticks (400 ms) between looks at the Probes' folders while some Probe is on: about a minute.
const BACKSTOP: u64 = 150;

/// The running Probes. A global of its own (`AppState`'s fields only shrink).
#[derive(Default)]
struct Probes {
    /// The flag as the Probes were last loaded.
    flag: bool,
    /// A load under way, which a newer one replaces.
    loading: Option<Task<()>>,
    running: Option<Running>,
}

impl Global for Probes {}

/// The hook feed: what ends it when it is dropped (the listener closes its subscription).
pub(crate) struct Running {
    _listener: Listener,
    _task: Task<()>,
}

/// Follow what makes the Probes change, and load them now. Once, at launch, with the windows' state made.
pub(crate) fn follow(cx: &mut App) {
    probe_runtime::on_approvals_changed(cx, reload).detach();
    cx.observe_global::<SettingsStore>(|cx| {
        if settings(cx).feature_enabled(FeatureFlag::Probes) != cx.default_global::<Probes>().flag {
            reload(cx);
        }
    })
    .detach();
    reload(cx);
}

/// Once per tick: the slow look at what is running.
pub(crate) fn tick(ticks: u64, cx: &mut App) {
    if ticks.is_multiple_of(BACKSTOP) && cx.default_global::<Probes>().running.is_some() {
        reload(cx);
    }
}

/// Read the Probes that are on and start (or stop) running them.
pub(crate) fn reload(cx: &mut App) {
    let on = settings(cx).feature_enabled(FeatureFlag::Probes);
    cx.default_global::<Probes>().flag = on;
    if !on {
        let probes = cx.default_global::<Probes>();
        (probes.loading, probes.running) = (None, None);
        probe_runtime::install(None, Vec::new(), cx);
        retain_folds(Vec::new(), cx);
        changed(cx);
        return;
    }
    let root = DataRoot::current();
    let load = cx.spawn(async move |cx| {
        let loaded = cx.background_executor().spawn(async move { active::load(&root) }).await;
        cx.update(|cx| start(loaded, cx));
    });
    cx.default_global::<Probes>().loading = Some(load);
}

fn start(loaded: active::Loaded, cx: &mut App) {
    for (id, why) in &loaded.skipped {
        tracing::warn!(probe = %id, "a Probe that is on could not run: {why}");
    }
    // A Probe turned off, or one that lost the `fold` grant, loses its folds now, not at the next fold.
    retain_folds(
        loaded.probes.iter().filter(|probe| probe.grants.fold).map(|probe| probe.id.clone()).collect(),
        cx,
    );
    let names = loaded.probes.iter().map(|probe| (probe.id.clone(), probe.name.clone())).collect();
    let rules = RuleSet::new(active::rules_of(&loaded.probes));
    probe_runtime::install(Some(rules), names, cx);
    let dispatcher = Dispatcher::new(loaded.probes);
    let running = listen(dispatcher, cx);
    let probes = cx.default_global::<Probes>();
    probes.loading = None;
    probes.running = running;
    changed(cx);
}

/// Subscribe to the topics `dispatcher`'s hooks listen to and carry out what they decide.
pub(crate) fn listen(mut dispatcher: Dispatcher, cx: &mut App) -> Option<Running> {
    let bus = crate::automation::bus(cx)?;
    let (listener, rx) = probe_feed::listen(&bus, &dispatcher)?;
    if listener.output {
        // Where each terminal's output starts from: its cursor now, so what was printed before the Probes
        // came on is not matched as if it were new.
        for window in app(cx).windows.clone() {
            let readers = window.workspace.read_with(cx, |ws, cx| ws.control_readers(cx)).unwrap_or_default();
            for facts in readers {
                bus.watch_output(window.id, facts.terminal, &facts.reader);
            }
        }
    }
    let tx = listener.sender.clone();
    let task = cx.spawn(async move |cx| {
        while let Ok(feed) = rx.recv().await {
            cx.update(|cx| {
                let effects = plan(feed, &mut dispatcher, cx);
                if !effects.is_empty() {
                    carry_out(effects, &tx, cx);
                    changed(cx);
                }
            });
        }
    });
    Some(Running { _listener: listener, _task: task })
}

/// What one feed item makes the hooks do, given what the windows know of each terminal.
fn plan(feed: Feed, dispatcher: &mut Dispatcher, cx: &App) -> Vec<active::Effect> {
    let tabs = ambient::all_tabs(cx);
    let app_active = crate::platform::dock::app_is_active(cx);
    let facts = |id: i64| {
        let tab = tabs.iter().find(|tab| tab.tab == id)?;
        Some(TabFacts { front: tab.key && app_active, agent: tab.who().is_some(), name: tab.name.clone() })
    };
    feed.plan(dispatcher, &facts, Instant::now())
}

/// Every terminal keeps only the folds of the Probes in `probes` (the ones that are on and may fold).
fn retain_folds(probes: Vec<String>, cx: &mut App) {
    for window in app(cx).windows.clone() {
        AppState::with_workspace(&window, cx, |ws, _, cx| ws.retain_probe_folds(&probes, cx));
    }
}

/// Every window draws what changed: its terminals' rows and its status bar.
fn changed(cx: &mut App) {
    for window in app(cx).windows.clone() {
        AppState::with_workspace(&window, cx, |ws, _, cx| ws.probes_changed(cx));
    }
}
