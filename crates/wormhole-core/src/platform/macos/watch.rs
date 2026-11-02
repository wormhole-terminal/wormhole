//! The watcher on FSEvents, through the `notify` crate that GPUI's component library already links.

use std::io;
use std::path::{Path, PathBuf};

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt as _;

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
    // A network or FUSE volume sends no events, which a watch cannot tell from a quiet folder: refuse
    // it, so the caller keeps its fast polling instead of trusting a silence.
    for root in roots {
        if !on_local_volume(root)? {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "not on a local volume"));
        }
    }
    let mut watcher = RecommendedWatcher::new(Relay(on_batch), Config::default()).map_err(into_io)?;
    // All roots in one commit: each `watch` call of its own would restart the stream.
    let mut paths = watcher.paths_mut();
    for root in roots {
        paths.add(root, RecursiveMode::Recursive).map_err(into_io)?;
    }
    paths.commit().map_err(into_io)?;
    Ok(Watcher { _stream: watcher })
}

/// Whether `path` is on a volume of this machine (`MNT_LOCAL`), as opposed to SMB, NFS, AFP or a
/// FUSE mount.
fn on_local_volume(path: &Path) -> io::Result<bool> {
    let c = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a NUL in the path"))?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: `c` is NUL-terminated and outlives the call, and `stat` is the size statfs writes.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::statfs(c.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: statfs returned 0, so it filled the whole struct.
    #[allow(unsafe_code)]
    let stat = unsafe { stat.assume_init() };
    Ok(stat.f_flags & libc::MNT_LOCAL as u32 != 0)
}

fn into_io(err: notify::Error) -> io::Error {
    match err.kind {
        notify::ErrorKind::Io(err) => err,
        notify::ErrorKind::PathNotFound => io::Error::new(io::ErrorKind::NotFound, err),
        _ => io::Error::other(err),
    }
}
