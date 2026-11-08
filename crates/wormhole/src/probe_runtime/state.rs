//! The global the Probes' effects are kept in.

use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::probe::runtime::{Effects, RuleSet, StatusItem};
use wormhole_core::probe::tone::Tone;

#[derive(Default)]
struct ProbeRuntime {
    rules: Option<Arc<RuleSet>>,
    /// Changes with the rules, so rows built with the old ones are built again.
    generation: u64,
    /// Changes when the user approved or turned off a Probe: `app::probes` loads them again.
    approvals: u64,
    /// The Probes that are on, what their hooks set and the blocks they tinted.
    effects: Effects,
}

impl Global for ProbeRuntime {}

fn runtime(cx: &mut App) -> &mut ProbeRuntime {
    cx.default_global::<ProbeRuntime>()
}

/// The rules of the Probes that are on; `None` while there are none (or Probes are off).
pub(crate) fn rules(cx: &App) -> Option<Arc<RuleSet>> {
    cx.try_global::<ProbeRuntime>()?.rules.clone()
}

/// Which rules the rows were built with.
pub(crate) fn generation(cx: &App) -> u64 {
    cx.try_global::<ProbeRuntime>().map_or(0, |runtime| runtime.generation)
}

/// Make `rules` the ones in force, and keep what the Probes of `active` (id and name) set: the others' is gone.
pub(crate) fn install(rules: Option<RuleSet>, active: Vec<(String, String)>, cx: &mut App) {
    let runtime = runtime(cx);
    runtime.rules = rules.filter(|rules| !rules.is_empty()).map(Arc::new);
    runtime.generation += 1;
    runtime.effects.install(active);
    cx.refresh_windows();
}

/// The name of the Probe `id`, when it is on.
pub(crate) fn probe_name(id: &str, cx: &App) -> Option<String> {
    cx.try_global::<ProbeRuntime>()?.effects.probe_name(id).map(str::to_string)
}

pub(crate) fn status_items(cx: &App) -> Vec<StatusItem> {
    cx.try_global::<ProbeRuntime>().map(|runtime| runtime.effects.status_items().to_vec()).unwrap_or_default()
}

/// A hook set `item`: it replaces what that Probe's item said.
pub(crate) fn set_status(item: StatusItem, cx: &mut App) {
    if runtime(cx).effects.set_status(item) {
        cx.refresh_windows();
    }
}

/// A hook gave the rows of `block` in `terminal` a tone.
pub(crate) fn mark_block(terminal: i64, block: u64, tone: Tone, cx: &mut App) {
    runtime(cx).effects.mark_block(terminal, block, tone);
    cx.refresh_windows();
}

pub(crate) fn block_tone(terminal: i64, block: u64, cx: &App) -> Option<Tone> {
    cx.try_global::<ProbeRuntime>()?.effects.block_tone(terminal, block)
}

/// The user approved or turned off a Probe: the ones that are on are read again.
pub(crate) fn approvals_changed(cx: &mut App) {
    runtime(cx);
    // An update, not a plain write: the observers of the global (`app::probes`) are told.
    cx.update_global(|runtime: &mut ProbeRuntime, _| runtime.approvals += 1);
}

pub(crate) fn approvals_epoch(cx: &App) -> u64 {
    cx.try_global::<ProbeRuntime>().map_or(0, |runtime| runtime.approvals)
}

/// Call `f` when the user approved or turned off a Probe (told as an event, after the update that said it).
pub(crate) fn on_approvals_changed(cx: &mut App, mut f: impl FnMut(&mut App) + 'static) -> Subscription {
    runtime(cx);
    let mut seen = approvals_epoch(cx);
    cx.observe_global::<ProbeRuntime>(move |cx| {
        let now = approvals_epoch(cx);
        if now != seen {
            seen = now;
            f(cx);
        }
    })
}
