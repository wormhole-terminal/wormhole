//! The global the Probes' effects are kept in.

use std::collections::VecDeque;
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::probe::tone::Tone;

use super::rules::RuleSet;

/// Blocks a hook marked that are remembered: the oldest is forgotten first, as scrollback forgets
/// its rows. A hook is rate-limited, so this is a ceiling a program cannot raise.
const MARKED_BLOCKS: usize = 256;

/// A status item a hook set: the Probe's, labelled so (its name is in the tooltip).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusItem {
    pub probe: String,
    pub probe_name: String,
    pub item: String,
    pub text: String,
    pub tone: Option<Tone>,
    pub tooltip: Option<String>,
}

#[derive(Default)]
struct ProbeRuntime {
    rules: Option<Arc<RuleSet>>,
    /// The Probes that are on: id and name.
    probes: Vec<(String, String)>,
    /// Changes with the rules, so rows built with the old ones are built again.
    generation: u64,
    /// Changes when the user approved or turned off a Probe: `app::probes` loads them again.
    approvals: u64,
    status: Vec<StatusItem>,
    /// `(terminal, block)` and the tone a hook gave it, oldest first.
    marked: VecDeque<((i64, u64), Tone)>,
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
    runtime.status.retain(|item| active.iter().any(|(id, _)| *id == item.probe));
    runtime.marked.clear();
    runtime.probes = active;
    cx.refresh_windows();
}

/// The name of the Probe `id`, when it is on.
pub(crate) fn probe_name(id: &str, cx: &App) -> Option<String> {
    let runtime = cx.try_global::<ProbeRuntime>()?;
    runtime.probes.iter().find(|(probe, _)| probe == id).map(|(_, name)| name.clone())
}

pub(crate) fn status_items(cx: &App) -> Vec<StatusItem> {
    cx.try_global::<ProbeRuntime>().map(|runtime| runtime.status.clone()).unwrap_or_default()
}

/// A hook set `item`: it replaces what that Probe's item said.
pub(crate) fn set_status(item: StatusItem, cx: &mut App) {
    let runtime = runtime(cx);
    match runtime.status.iter_mut().find(|old| old.probe == item.probe && old.item == item.item) {
        Some(old) if *old == item => return,
        Some(old) => *old = item,
        None => runtime.status.push(item),
    }
    cx.refresh_windows();
}

/// A hook gave the rows of `block` in `terminal` a tone.
pub(crate) fn mark_block(terminal: i64, block: u64, tone: Tone, cx: &mut App) {
    let runtime = runtime(cx);
    runtime.marked.retain(|(key, _)| *key != (terminal, block));
    runtime.marked.push_back(((terminal, block), tone));
    while runtime.marked.len() > MARKED_BLOCKS {
        runtime.marked.pop_front();
    }
    cx.refresh_windows();
}

pub(crate) fn block_tone(terminal: i64, block: u64, cx: &App) -> Option<Tone> {
    let runtime = cx.try_global::<ProbeRuntime>()?;
    runtime.marked.iter().rev().find(|(key, _)| *key == (terminal, block)).map(|(_, tone)| *tone)
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
