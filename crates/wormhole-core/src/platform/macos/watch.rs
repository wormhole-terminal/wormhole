//! The watcher on FSEvents, through the `notify` crate that GPUI's component library already links.

use std::io;
use std::path::{Path, PathBuf};

use notify::{Config, Event, EventHandler, RecommendedWatcher, RecursiveMode, Watcher as _};

/// What the neutral file hands to the OS: called with the changed paths and whether events were lost.
type Handler = Box<dyn FnMut(Vec<PathBuf>, bool) + Send>;

/// The most paths one event carries on: a rename names two, nothing real names more, and a hostile
/// event must not make the callback allocate without end.
const MAX_PATHS: usize = 8;

/// FSEvents, through the `notify` crate's backend. It reports file-level events and flags
/// `MustScanSubDirs` (with `UserDropped` / `KernelDropped` when the cause is lost events) as
/// `Flag::Rescan`, which `need_rescan` reads. The stream's latency is not settable there, which is why
/// the caller debounces.
pub(crate) struct Watcher {
    /// Held for its drop, which stops the stream.
    _stream: RecommendedWatcher,
}

struct Relay(Handler);

impl EventHandler for Relay {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        match event {
            Ok(event) => {
                let rescan = event.need_rescan();
                (self.0)(event.paths.into_iter().take(MAX_PATHS).collect(), rescan);
            }
            // The backend failed to read the stream: what changed is unknown.
            Err(_) => (self.0)(Vec::new(), true),
        }
    }
}

pub(crate) fn watch(roots: &[&Path], on_batch: Handler) -> io::Result<Watcher> {
    let mut watcher = RecommendedWatcher::new(Relay(on_batch), Config::default()).map_err(into_io)?;
    for root in roots {
        watcher.watch(root, RecursiveMode::Recursive).map_err(into_io)?;
    }
    Ok(Watcher { _stream: watcher })
}

fn into_io(err: notify::Error) -> io::Error {
    match err.kind {
        notify::ErrorKind::Io(err) => err,
        notify::ErrorKind::PathNotFound => io::Error::new(io::ErrorKind::NotFound, err),
        _ => io::Error::other(err),
    }
}
