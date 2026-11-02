//! A folder followed through the system's file events (FSEvents on macOS), so what depends on files
//! asks when something changed instead of every few seconds.
//!
//! This is only the source: which paths matter and how often to read are the caller's (`git::watch`).
//! Events arrive as the system delivers them, one callback per event, from a thread of the watcher's
//! own; the callback must be quick and must not block. The system coalesces nothing here (the
//! backend fixes its stream latency at zero), so a caller that reads on every event is wrong: it
//! debounces.
//!
//! An event source can fail: the system says so when it drops events ([`Batch::rescan`]: read
//! everything again), and a network volume or a FUSE mount sends none at all, which a watch cannot
//! tell from a quiet folder. Whoever depends on a watch keeps a slow poll as the fallback. Off macOS
//! there is no watcher: [`watch`] fails with `Unsupported` and the caller keeps polling.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use super::linux::watch as os;
#[cfg(target_os = "macos")]
use super::macos::watch as os;

/// What one file event reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Batch {
    /// The paths that changed (absolute, as the system names them, which is the real path: a folder
    /// reached through a link is reported under its target).
    pub paths: Vec<PathBuf>,
    /// The system lost events, or says it cannot tell which paths changed: whatever was watched may
    /// have changed anywhere under it, so read it again.
    pub rescan: bool,
}

/// A running watch; dropping it stops it.
#[must_use = "a watch stops when it is dropped"]
pub struct Watch {
    _os: os::Watcher,
}

/// Follow `roots` (folders, each with everything below it, at file level) and call `on_batch` for
/// every event. Fails when a root cannot be watched (it is not there, or the system refuses), and with
/// `Unsupported` where there is no watcher.
pub fn watch(roots: &[&Path], on_batch: impl FnMut(Batch) + Send + 'static) -> io::Result<Watch> {
    let mut on_batch = on_batch;
    let relay = Box::new(move |paths, rescan| on_batch(Batch { paths, rescan }));
    Ok(Watch { _os: os::watch(roots, relay)? })
}

/// Whether this system has a watcher at all.
pub fn supported() -> bool {
    cfg!(target_os = "macos")
}
