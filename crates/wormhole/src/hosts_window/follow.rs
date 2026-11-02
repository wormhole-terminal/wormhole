//! The hosts folder followed through file events, so the Session Hosts window looks when a host appears,
//! goes or changes its meta file instead of every second.
//!
//! Only some of what the window shows is on disk. A host answering or not, its memory, and the counters of
//! its connection to this app have no event: they are asked of the processes and sockets, so the window
//! keeps a slow look as well (`SLOW_LOOK`, in `mod.rs`) while it is open, and a faster one where this watch
//! could not start (`FALLBACK_LOOK`: a network volume, or the folder not made yet).
//!
//! The pieces are those of `app/own_files.rs`: the system delivers events on a thread of its own, whose
//! callback only wakes a task, which waits for the writer to finish ([`SETTLE`]) and looks. The stream is
//! opened off the UI thread.

use std::io;
use std::path::Path;
use std::time::Duration;

use wormhole_core::platform::watch::{self as os_watch, Watch};

/// How long after the first event of a burst the folder is read: a host writes its meta file and its
/// socket within moments of each other.
pub(super) const SETTLE: Duration = Duration::from_millis(60);

/// A running watch of the hosts folder, and the channel its events wake. Dropping it stops the stream.
pub(super) struct Followed {
    _watch: Watch,
    pub woken: smol::channel::Receiver<()>,
}

/// Follow `dir` (the hosts folder). Fails when it is not there yet, the system refuses, or it is on a
/// volume that sends no events; the caller then looks on a timer alone. Blocks while the stream starts:
/// off the UI thread.
pub(super) fn follow(dir: &Path) -> io::Result<Followed> {
    let base = dir.canonicalize()?;
    let (wake, woken) = smol::channel::bounded::<()>(1);
    let watch = os_watch::watch(&[base.as_path()], move |_batch| {
        // Anything in the hosts folder is a host's file; full already means a wake-up is on its way.
        let _ = wake.try_send(());
    })?;
    Ok(Followed { _watch: watch, woken })
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// A host's file appearing in the folder wakes the window.
    #[test]
    fn a_new_file_in_the_hosts_folder_wakes_the_watch() {
        let dir = std::env::temp_dir().join(format!("wormhole-hosts-follow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let followed = match follow(&dir) {
            Ok(followed) => followed,
            // No watcher here (not macOS, or a volume that sends no events): nothing to show.
            Err(err) if err.kind() == io::ErrorKind::Unsupported => return,
            Err(err) => panic!("not followed: {err}"),
        };
        std::fs::write(dir.join("abcdefghij12.meta"), b"x").expect("a file");
        let until = Instant::now() + Duration::from_secs(10);
        while followed.woken.try_recv().is_err() {
            assert!(Instant::now() < until, "no event came for a new file");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_folder_that_is_not_there_is_not_followed() {
        let missing = std::env::temp_dir().join(format!("wormhole-hosts-missing-{}", std::process::id()));
        assert!(follow(&missing).is_err());
    }
}
