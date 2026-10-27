//! The tool calls in flight: how many may run at once, and how a `notifications/cancelled` finds the one
//! it names. A call is a thread of its own (a `wait` can take two minutes and the loop that reads the
//! client's messages must go on, or it could never hear the cancellation).

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::ipc::exchange::Cancel;

/// How many calls may run at once. A fifth is told so at once: blocking the loop that reads would stop
/// the cancellations from being heard.
pub(super) const CALLS_MAX: usize = 4;

#[derive(Default)]
struct Table {
    /// Calls and batches running.
    running: usize,
    /// The calls that can be cancelled, by their request ids (as JSON, so `1` and `"1"` are two).
    by_id: HashMap<String, Arc<Cancel>>,
}

#[derive(Default)]
pub(super) struct Calls {
    table: Mutex<Table>,
    idle: Condvar,
}

/// A place among the [`CALLS_MAX`], given back when dropped.
pub(super) struct Permit {
    calls: Arc<Calls>,
}

/// A request that can be cancelled while it runs; forgotten when dropped.
pub(super) struct Tracked {
    calls: Arc<Calls>,
    key: String,
    pub(super) cancel: Arc<Cancel>,
}

impl Calls {
    fn table(&self) -> MutexGuard<'_, Table> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A place to run a call or a batch, or `None` when [`CALLS_MAX`] are running.
    pub(super) fn reserve(self: &Arc<Self>) -> Option<Permit> {
        let mut table = self.table();
        if table.running >= CALLS_MAX {
            return None;
        }
        table.running += 1;
        Some(Permit { calls: Arc::clone(self) })
    }

    /// Make the request `id` cancellable. A second request with an id still in use (a client's mistake)
    /// replaces the first: a cancellation names the newer.
    pub(super) fn track(self: &Arc<Self>, id: &Value) -> Tracked {
        let cancel = Arc::new(Cancel::default());
        let key = id.to_string();
        self.table().by_id.insert(key.clone(), Arc::clone(&cancel));
        Tracked { calls: Arc::clone(self), key, cancel }
    }

    /// `notifications/cancelled` for `id`: end the call if it is still running. An unknown or finished one
    /// is the race the spec expects, and is ignored.
    pub(super) fn cancel(&self, id: &Value) {
        let found = self.table().by_id.get(&id.to_string()).cloned();
        if let Some(cancel) = found {
            cancel.cancel();
        }
    }

    pub(super) fn cancel_all(&self) {
        let all: Vec<_> = self.table().by_id.values().cloned().collect();
        for cancel in all {
            cancel.cancel();
        }
    }

    /// Wait until nothing runs, at most `within`; whether nothing does.
    pub(super) fn wait_idle(&self, within: Duration) -> bool {
        let deadline = Instant::now() + within;
        let mut table = self.table();
        while table.running > 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else { return false };
            table = self.idle.wait_timeout(table, left).unwrap_or_else(PoisonError::into_inner).0;
        }
        true
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut table = self.calls.table();
        table.running = table.running.saturating_sub(1);
        self.calls.idle.notify_all();
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        let mut table = self.calls.table();
        // Only our own entry: a later request with the same id may have taken the place.
        if table.by_id.get(&self.key).is_some_and(|cancel| Arc::ptr_eq(cancel, &self.cancel)) {
            table.by_id.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;

    use super::{CALLS_MAX, Calls};

    #[test]
    fn only_so_many_calls_run_at_once_and_a_place_comes_back_when_one_ends() {
        let calls = Arc::new(Calls::default());
        let permits: Vec<_> = (0..CALLS_MAX).map(|_| calls.reserve().expect("room")).collect();
        assert!(calls.reserve().is_none());
        assert!(!calls.wait_idle(Duration::from_millis(20)));
        drop(permits);
        assert!(calls.reserve().is_some());
        assert!(calls.wait_idle(Duration::from_secs(1)));
    }

    #[test]
    fn a_cancellation_finds_its_call_by_id_and_a_number_is_not_its_string() {
        let calls = Arc::new(Calls::default());
        let one = calls.track(&json!(1));
        let text = calls.track(&json!("1"));
        calls.cancel(&json!(1));
        assert!(one.cancel.is_cancelled() && !text.cancel.is_cancelled());
        // Unknown and finished ones are ignored.
        calls.cancel(&json!(99));
        drop(one);
        calls.cancel(&json!(1));
        calls.cancel_all();
        assert!(text.cancel.is_cancelled());
    }

    #[test]
    fn a_request_that_reuses_an_id_does_not_lose_the_newer_ones_entry_when_the_older_ends() {
        let calls = Arc::new(Calls::default());
        let old = calls.track(&json!(7));
        let new = calls.track(&json!(7));
        drop(old);
        calls.cancel(&json!(7));
        assert!(new.cancel.is_cancelled());
    }
}
