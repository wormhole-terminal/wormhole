//! What every object of this folder shares: its error, the coalesced wake and the data folder rule.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::error::FfiError;
use crate::isolation::{self, FOLDER_PREFIX};

/// What can go wrong in the auxiliary windows' backends.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AuxError {
    /// The process (or the folder it was given) is not the experiment's own: it would have touched the real
    /// app's data.
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// The operation could not be done; the text is for the person.
    #[error("{reason}")]
    Failed { reason: String },
}

impl AuxError {
    pub(crate) fn failed(why: impl std::fmt::Display) -> Self {
        Self::Failed { reason: why.to_string() }
    }
}

/// What Swift implements to hear that something changed: called from a Rust thread with no lock held, at
/// most once between two reads of the object's state; it must return at once (note the wake and hop to the
/// main queue) and never call back into the object from inside.
#[uniffi::export(with_foreign)]
pub trait AuxListener: Send + Sync {
    fn wake(&self);
}

/// One wake until the state was read: a burst of changes is one call.
pub(crate) struct Waker {
    woken: AtomicBool,
    listener: Arc<dyn AuxListener>,
}

impl Waker {
    pub(crate) fn new(listener: Arc<dyn AuxListener>) -> Self {
        Self { woken: AtomicBool::new(false), listener }
    }

    pub(crate) fn notify(&self) {
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }

    /// Called by the read that Swift does after a wake: the next change wakes again.
    pub(crate) fn rearm(&self) {
        self.woken.store(false, Ordering::Release);
    }
}

pub(crate) fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The data folder an object works in: the one given (a folder named `Wormhole-swift*`: tests and previews
/// pass their own) or core's own, which must be the experiment's (`WORMHOLE_DATA_DIR`, see `isolation`).
pub(crate) fn resolve_data_dir(explicit: Option<String>) -> Result<PathBuf, AuxError> {
    let Some(dir) = explicit else {
        isolation::require().map_err(|err| match err {
            FfiError::NotIsolated { reason } => AuxError::NotIsolated { reason },
            other => AuxError::failed(other),
        })?;
        return Ok(wormhole_core::storage::paths::data_dir());
    };
    let dir = PathBuf::from(dir);
    let named = dir.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(FOLDER_PREFIX));
    if named {
        Ok(dir)
    } else {
        Err(AuxError::NotIsolated {
            reason: format!("{} is not a folder named {FOLDER_PREFIX}*", dir.display()),
        })
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::AuxListener;

    /// Counts wakes.
    #[derive(Default)]
    pub(crate) struct Wakes(pub(crate) AtomicUsize);

    impl AuxListener for Wakes {
        fn wake(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Wakes {
        pub(crate) fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        pub(crate) fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    /// An isolated data folder in a temp dir that lives as long as the returned guard.
    pub(crate) fn scratch() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().expect("a temp dir");
        let dir = root.path().join("Wormhole-swift-t");
        std::fs::create_dir_all(&dir).expect("data dir");
        (root, dir)
    }
}
