//! The ids terminals and splits are given, when several windows share them.
//!
//! A terminal's id is what its notes, its saved row and the agent hooks that name it (`WORMHOLE_TAB_ID`)
//! go by, so it must be unique in the whole app, not only in one window: a terminal moved to another window
//! keeps it. One counter, shared by every window's model (the GPUI app's `AppState::alloc_id_in`).

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

/// The next free id, one counter for terminals and splits. Cloning shares it.
#[derive(Clone, Debug)]
pub struct IdCounter(Arc<AtomicI64>);

impl IdCounter {
    /// A counter whose first id is `first` (`Store::next_ids` says where the saved ones end).
    pub fn starting_at(first: i64) -> Self {
        Self(Arc::new(AtomicI64::new(first.max(1))))
    }

    pub fn next(&self) -> i64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    /// Never hand out an id below `first`: ids that came from elsewhere (a window being restored) are taken.
    pub fn keep_above(&self, taken: i64) {
        self.0.fetch_max(taken.saturating_add(1), Ordering::Relaxed);
    }

    /// The id `next` would hand out now: every id below it may be taken, none from it on is yet.
    pub fn peek(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}
