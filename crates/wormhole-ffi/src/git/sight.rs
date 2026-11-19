//! Whether the window a reader serves can be seen, and the watches it started: a window nobody sees is told nothing by
//! them until it is seen again (`GitWatch::set_seen`; the GPUI app's `window/repo/state.rs` `Sight`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use super::error::GitBackendError;
use super::service::Git;
use super::watch::{GitListener, GitWatch};

/// One per [`Git`]: the sight of its window, and the watches made through it.
#[derive(Default)]
pub(super) struct Sight {
    /// Inverted so that a reader nobody told anything about is seen.
    unseen: AtomicBool,
    watches: Mutex<Vec<Weak<GitWatch>>>,
}

impl Sight {
    fn watches(&self) -> std::sync::MutexGuard<'_, Vec<Weak<GitWatch>>> {
        self.watches.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[uniffi::export]
impl Git {
    /// Follow the repository `folder` is in, as [`GitWatch::start`] does, in the sight this reader was last told of.
    pub fn watch(
        &self,
        folder: String,
        listener: Arc<dyn GitListener>,
    ) -> Result<Arc<GitWatch>, GitBackendError> {
        let watch = GitWatch::start(folder, listener)?;
        // Told and registered under one lock, so a `set_seen` between the two cannot miss this watch.
        let mut watches = self.sight.watches();
        watch.set_seen(!self.sight.unseen.load(Ordering::Acquire));
        watches.retain(|held| held.strong_count() > 0);
        watches.push(Arc::downgrade(&watch));
        Ok(watch)
    }

    /// The window this reader serves can be seen, or cannot (on screen, not minimised, not covered): its watches tell
    /// nothing while it cannot, and once when it can again if anything changed meanwhile.
    pub fn set_seen(&self, seen: bool) {
        let mut watches = self.sight.watches();
        self.sight.unseen.store(!seen, Ordering::Release);
        watches.retain(|held| held.strong_count() > 0);
        for watch in watches.iter().filter_map(Weak::upgrade) {
            watch.set_seen(seen);
        }
    }
}
