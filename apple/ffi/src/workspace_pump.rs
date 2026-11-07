//! The one thread of a workspace that waits for a moment. Everything the model needs a time for is
//! asked of it as a deadline, so an idle workspace sleeps until something happens: a busy terminal's
//! output going quiet and a title waiting its turn (`WorkspaceModel::next_due`), the looks at each
//! terminal's process, the reads of git (`workspace_git`) and the debounce of a repository's file
//! events. An event that may move a deadline [`kick`](Pump::kick)s the thread; it does what is due,
//! and sleeps to the next.
//!
//! The thread holds the workspace weakly and ends with it.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use wormhole_core::workspace::StatusEvent;

use crate::workspace::Shared;
use crate::workspace_git;

/// How often a terminal's process is looked at when nothing said it changed. The system sends no event
/// for who holds a terminal, so this is the fallback; the app's tick looks every 2 s, here output and
/// the shell's marks make a look sooner (`AFTER_OUTPUT`, `AFTER_MARK`) and nothing else needs one.
const PROBE_EVERY: u64 = 5_000;
/// A terminal's first seconds are looked at more often (`YOUNG_EVERY`): its program is starting, and
/// the first look usually comes before the process has become what it will be (an agent exec'd by a
/// shell, a script's interpreter).
const YOUNG: u64 = 3_000;
const YOUNG_EVERY: u64 = 400;
/// A look after output has settled for this long: a program started or ended and printed.
pub(crate) const AFTER_OUTPUT: u64 = 250;
/// A look right after a command started or ended (the program has had a moment to exec).
pub(crate) const AFTER_MARK: u64 = 120;
/// The least the thread sleeps, so a deadline that cannot be met never makes it spin.
const LEAST_SLEEP: u64 = 5;
/// The most tick events one round feeds: a bound, since each normally ends its own deadline.
const TICKS_PER_ROUND: usize = 64;

#[derive(Default)]
pub(crate) struct Pump {
    state: Mutex<State>,
    wake: Condvar,
}

#[derive(Default)]
struct State {
    kicked: bool,
    stopped: bool,
    /// When each terminal's process is next looked at, on the workspace's clock.
    probes: HashMap<i64, u64>,
    /// When each terminal was handed over.
    born: HashMap<i64, u64>,
}

impl Pump {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Something that may have moved a deadline happened.
    pub(crate) fn kick(&self) {
        self.state().kicked = true;
        self.wake.notify_one();
    }

    /// The workspace is going away.
    pub(crate) fn stop(&self) {
        self.state().stopped = true;
        self.wake.notify_one();
    }

    /// Look at terminal `id`'s process at `at` (the earlier of this and what was already asked for).
    pub(crate) fn probe_by(&self, id: i64, at: u64) {
        self.probe_later(id, at);
        self.kick();
    }

    /// [`probe_by`](Self::probe_by) from the thread itself, which looks at its deadlines next anyway.
    fn probe_later(&self, id: i64, at: u64) {
        self.state().probes.entry(id).and_modify(|due| *due = (*due).min(at)).or_insert(at);
    }

    pub(crate) fn forget(&self, id: i64) {
        let mut state = self.state();
        state.probes.remove(&id);
        state.born.remove(&id);
    }

    /// Terminal `id` was handed over at `at`.
    pub(crate) fn born(&self, id: i64, at: u64) {
        self.state().born.insert(id, at);
    }

    /// When to look at terminal `id` again after the look at `now`.
    fn next_look(&self, id: i64, now: u64) -> u64 {
        let young = self.state().born.get(&id).is_some_and(|born| now < born + YOUNG);
        now + if young { YOUNG_EVERY } else { PROBE_EVERY }
    }

    fn take_due_probes(&self, now: u64) -> Vec<i64> {
        let mut state = self.state();
        let due: Vec<i64> = state.probes.iter().filter(|&(_, &at)| at <= now).map(|(&id, _)| id).collect();
        for id in &due {
            state.probes.remove(id);
        }
        due
    }

    fn next_probe(&self) -> Option<u64> {
        self.state().probes.values().copied().min()
    }

    /// Sleep until `wait` has passed or a kick; whether the workspace is going away.
    fn park(&self, wait: Option<Duration>) -> bool {
        let mut state = self.state();
        if !state.kicked && !state.stopped {
            state = match wait {
                Some(wait) => self.wake.wait_timeout(state, wait).unwrap_or_else(PoisonError::into_inner).0,
                None => self.wake.wait(state).unwrap_or_else(PoisonError::into_inner),
            };
        }
        state.kicked = false;
        state.stopped
    }
}

/// Start the thread for `shared`.
pub(crate) fn spawn(shared: &Arc<Shared>) {
    let (weak, pump): (Weak<Shared>, Arc<Pump>) = (Arc::downgrade(shared), shared.pump.clone());
    let started = std::thread::Builder::new().name("wormhole-ffi-pump".into()).spawn(move || {
        loop {
            // The workspace is held for a round, never while asleep.
            let wait = match weak.upgrade() {
                Some(shared) => round(&shared),
                None => return,
            };
            if pump.park(wait) {
                return;
            }
        }
    });
    if let Err(err) = started {
        // Without it nothing settles by itself; the events still feed the rows.
        eprintln!("wormhole-ffi: no pump thread: {err}");
    }
}

/// Do what is due, and say how long until the next thing is.
fn round(shared: &Arc<Shared>) -> Option<Duration> {
    let mut next: Vec<u64> = Vec::new();
    for _ in 0..TICKS_PER_ROUND {
        match shared.with_model(|model| model.next_due()) {
            Some((id, at)) if at <= shared.now() => shared.feed(id, StatusEvent::Tick),
            Some((_, at)) => {
                next.push(at);
                break;
            }
            None => break,
        }
    }
    for id in shared.pump.take_due_probes(shared.now()) {
        if shared.probe(id) {
            let now = shared.now();
            shared.pump.probe_later(id, shared.pump.next_look(id, now));
        }
    }
    next.extend(shared.pump.next_probe());
    next.extend(workspace_git::step(shared));
    let now = shared.now();
    next.into_iter().min().map(|at| Duration::from_millis(at.saturating_sub(now).max(LEAST_SLEEP)))
}
