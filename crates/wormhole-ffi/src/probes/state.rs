//! The Probes that are on, for the whole process: the one set of their text rules every terminal's frame is
//! decorated with and every link asks, and what their hooks set (status items, tinted blocks). The GPUI app
//! keeps the same in a global of its own (`probe_runtime`); the values are core's (`probe::runtime`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use wormhole_core::probe::runtime::{Effects, RuleSet, StatusItem};
use wormhole_core::probe::tone::Tone;

#[derive(Default)]
struct State {
    rules: Option<Arc<RuleSet>>,
    /// Changes with the rules, so a terminal works its spans out again.
    generation: u64,
    effects: Effects,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Mutex::default);
/// "Never hide lines" or the output-folds flag off: no terminal folds anything.
static FOLDS_OFF: AtomicBool = AtomicBool::new(false);

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The rules in force and which they are; `None` while there are none.
pub(crate) fn rules() -> (Option<Arc<RuleSet>>, u64) {
    let state = state();
    (state.rules.clone(), state.generation)
}

/// Make `rules` the ones in force; the Probes of `active` (id and name) keep what they set, the others' is gone.
pub(crate) fn install(rules: Option<RuleSet>, active: Vec<(String, String)>) {
    let mut state = state();
    state.rules = rules.filter(|rules| !rules.is_empty()).map(Arc::new);
    state.generation += 1;
    state.effects.install(active);
}

pub(crate) fn status_items() -> Vec<StatusItem> {
    state().effects.status_items().to_vec()
}

/// Whether the item changed what was shown.
pub(crate) fn set_status(item: StatusItem) -> bool {
    state().effects.set_status(item)
}

pub(crate) fn mark_block(terminal: i64, block: u64, tone: Tone) {
    state().effects.mark_block(terminal, block, tone);
}

pub(crate) fn block_tone(terminal: i64, block: u64) -> Option<Tone> {
    state().effects.block_tone(terminal, block)
}

/// The name of the Probe `id`, when it is on.
pub(crate) fn probe_name(id: &str) -> Option<String> {
    state().effects.probe_name(id).map(str::to_string)
}

/// Whether the view shows every line. True when it changed.
pub(crate) fn set_folds_off(off: bool) -> bool {
    FOLDS_OFF.swap(off, Ordering::Relaxed) != off
}

pub(crate) fn folds_off() -> bool {
    FOLDS_OFF.load(Ordering::Relaxed)
}

/// The tests share this state with every other test of the process: one at a time.
#[cfg(test)]
pub(crate) fn test_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}
