//! How long a terminal has been in its state, in the words the status bar's button uses
//! (`view::layout::status_bar::elapsed`): "<1m", "4m", "1h 5m", and when they next read differently.

use std::time::Duration;

use wormhole_core::view::layout::status_bar::elapsed;
use wormhole_core::view::sidebar::wording::until_next;

use crate::workspace::Workspace;

/// How long ago a time was, in words.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Waited {
    pub words: String,
    /// Milliseconds until the words change: the one timer's due time.
    pub next_in_ms: u64,
}

#[uniffi::export]
impl Workspace {
    /// The time `since_ms` (a terminal summary's `since`, on this workspace's clock) as the status bar says it.
    pub fn waited(&self, since_ms: u64) -> Waited {
        let waited = Duration::from_millis(self.now_ms().saturating_sub(since_ms));
        let next = until_next(waited, Duration::from_secs(60));
        Waited { words: elapsed(waited), next_in_ms: next.as_millis() as u64 }
    }
}
