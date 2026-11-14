//! The app's quit as one desk sees it. A start or an attach still under way when the app quits ends on its own thread
//! with nobody to take what it made: the desk lets go of that host itself (a plain quit) or ends it (Quit and End All),
//! as the GPUI app's `kept_past_quit` (`terminal_view/hosted.rs`) does. An ending quit also waits for what is under
//! way, so every host this desk started is there to be ended. The state is the desk's: a desk made after another one
//! quit in the same process starts as any desk does.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How long an ending quit waits for the starts and attaches under way: a host's start gives up after five seconds
/// (`wormhole_term::client::START_TIMEOUT`).
const UNDER_WAY_WAIT: Duration = Duration::from_secs(6);

/// What a terminal made now is to do: run on, be let go of, or end with its host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fate {
    Runs,
    LetGo,
    Ends,
}

#[derive(Default)]
pub(super) struct QuitGate {
    state: Mutex<State>,
    settled: Condvar,
}

#[derive(Default)]
struct State {
    /// `Some(end_hosts)` once the app quit.
    quit: Option<bool>,
    under_way: usize,
}

/// A start or an attach under way, counted until it is dropped.
pub(super) struct UnderWay<'a>(&'a QuitGate);

impl QuitGate {
    /// A start or an attach begins.
    pub(super) fn begin(&self) -> UnderWay<'_> {
        self.lock().under_way += 1;
        UnderWay(self)
    }

    /// What a terminal made now is to do.
    pub(super) fn fate(&self) -> Fate {
        match self.lock().quit {
            None => Fate::Runs,
            Some(false) => Fate::LetGo,
            Some(true) => Fate::Ends,
        }
    }

    /// The app quits: from now on what is made is let go of, or ended with `end_hosts`, which also waits (a few seconds
    /// at most) for the starts and attaches under way.
    pub(super) fn quit(&self, end_hosts: bool) {
        let mut state = self.lock();
        state.quit = Some(end_hosts);
        if !end_hosts {
            return;
        }
        let until = Instant::now() + UNDER_WAY_WAIT;
        while state.under_way > 0 {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                tracing::warn!(under_way = state.under_way, "quit before every session host had started");
                return;
            }
            state = self.settled.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    #[cfg(test)]
    pub(super) fn under_way(&self) -> usize {
        self.lock().under_way
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for UnderWay<'_> {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.under_way = state.under_way.saturating_sub(1);
        self.0.settled.notify_all();
    }
}
