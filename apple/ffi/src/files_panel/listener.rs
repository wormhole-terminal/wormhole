//! How a Rust thread tells Swift that something changed: the contract `TerminalListener` and
//! `WorkspaceListener` already state, once for the panel's seams.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What Swift implements to hear that something changed. Called from a Rust thread with no lock held, at
/// most once between two `drain()`s of the object that was given it; it must return at once: note the
/// wake and hop to the main queue. Never call into the same object from inside it.
#[uniffi::export(with_foreign)]
pub trait PanelListener: Send + Sync {
    fn wake(&self);
}

/// One coalesced wake: a flood of changes is one call until the owner drains.
pub(super) struct Waker {
    listener: Arc<dyn PanelListener>,
    woken: AtomicBool,
}

impl Waker {
    pub(super) fn new(listener: Arc<dyn PanelListener>) -> Self {
        Self { listener, woken: AtomicBool::new(false) }
    }

    /// Wakes Swift unless it was woken and has not drained since.
    pub(super) fn wake(&self) {
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }

    /// Swift drained: the next change wakes again.
    pub(super) fn rearm(&self) {
        self.woken.store(false, Ordering::Release);
    }
}

#[cfg(test)]
pub(super) mod testing {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::PanelListener;

    /// A listener that counts its wakes.
    #[derive(Default)]
    pub(crate) struct Counting(pub(crate) AtomicU32);

    impl PanelListener for Counting {
        fn wake(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Counting {
        pub(crate) fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        pub(crate) fn count(&self) -> u32 {
            self.0.load(Ordering::SeqCst)
        }
    }
}
