//! Following a profile's `config.json` (and the theme files that resolve with it) so a hand edit, a
//! script or another window reaches the open Settings window without anyone polling it.
//!
//! The shape is the app's `own_files`: the OS delivers events on a thread of its own whose callback
//! only sorts the paths and sends a mark; one thread of ours waits for the writer to finish
//! ([`SETTLE`]) and then asks the owner to look (`check`), which compares the disk with what it
//! holds and does nothing when it is the same. So a write we made ourselves, a stray event or a lost
//! one costs a `stat`, never a wrong reload.
//!
//! One watch on the profile's folder, not on the file: an atomic write replaces `config.json` and a
//! watch on the old file would follow the old file. A watch can fail (a network home, a folder not
//! made yet) and drop events, so `check` also runs on a timer: slowly while the watch lives
//! ([`SLOW_POLL`]), quickly while it does not ([`NO_WATCH_POLL`]). The thread ends when the
//! [`Follower`] is dropped.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use wormhole_core::platform::watch::{self as os_watch, Batch, Watch};
use wormhole_core::storage::paths::ProfilePaths;

/// How long after the first event of a burst the file is read: an editor's save is a few events and
/// the read should see the finished file.
const SETTLE: Duration = Duration::from_millis(40);
/// The backstop while events arrive (the app's tick does the same, about every ten seconds).
const SLOW_POLL: Duration = Duration::from_secs(10);
/// Without a watch the timer is all there is; the app's own tick is 400 ms, this window needs less.
const NO_WATCH_POLL: Duration = Duration::from_secs(2);

/// Keeps the watch and its thread alive; dropping it stops both.
pub(crate) struct Follower {
    _watch: Option<Watch>,
    /// Held for its drop: the thread sees the channel close and ends.
    _stop: Sender<()>,
}

impl Follower {
    /// Starts following `profile`. `check` runs on a thread of ours after a change (or on the timer) and
    /// returns `false` once its owner is gone, which ends the thread.
    pub(crate) fn start(profile: &ProfilePaths, check: impl Fn() -> bool + Send + 'static) -> Self {
        let (mark, marked) = mpsc::channel::<()>();
        let watch = open(profile, mark.clone());
        let poll = if watch.is_ok() { SLOW_POLL } else { NO_WATCH_POLL };
        if let Err(err) = &watch
            && !matches!(err.kind(), io::ErrorKind::Unsupported | io::ErrorKind::NotFound)
        {
            tracing::warn!("not following the profile's settings file: {err}");
        }
        std::thread::Builder::new()
            .name("settings-watch".into())
            .spawn(move || {
                loop {
                    match marked.recv_timeout(poll) {
                        Ok(()) => {
                            std::thread::sleep(SETTLE);
                            // Emptied before the look: an event after it sends a mark of its own.
                            while marked.try_recv().is_ok() {}
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    if !check() {
                        break;
                    }
                }
            })
            .ok();
        Self { _watch: watch.ok(), _stop: mark }
    }
}

/// The two places of a profile that settings read back, as the system names paths (real paths).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Roots {
    config: PathBuf,
    themes: PathBuf,
}

impl Roots {
    fn of(profile: &ProfilePaths, base: &Path) -> Self {
        let under = |path: PathBuf| path.file_name().map_or(path.clone(), |name| base.join(name));
        Self { config: under(profile.config_file()), themes: under(profile.themes_dir()) }
    }

    /// Whether a batch calls for a look: it names the file or a theme, or events were lost.
    fn matters(&self, batch: &Batch) -> bool {
        batch.rescan || batch.paths.iter().any(|path| *path == self.config || path.starts_with(&self.themes))
    }
}

fn open(profile: &ProfilePaths, mark: Sender<()>) -> io::Result<Watch> {
    // The folder may not exist yet on a first launch: made with the file, retried by the next start.
    let base = profile.dir().canonicalize()?;
    let roots = Roots::of(profile, &base);
    os_watch::watch(&[base.as_path()], move |batch| {
        if roots.matters(&batch) {
            let _ = mark.send(());
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wormhole_core::platform::watch::Batch;
    use wormhole_core::storage::paths::DataRoot;

    use super::Roots;

    fn batch(paths: &[&str], rescan: bool) -> Batch {
        Batch { paths: paths.iter().map(PathBuf::from).collect(), rescan }
    }

    #[test]
    fn only_the_file_the_themes_and_lost_events_call_for_a_look() {
        let root = DataRoot::new("/data");
        let profile = root.default_profile();
        let roots = Roots::of(&profile, profile.dir());
        let config = profile.config_file();
        assert!(roots.matters(&Batch { paths: vec![config.clone()], rescan: false }));
        assert!(roots.matters(&Batch { paths: vec![profile.themes_dir().join("mine.json")], rescan: false }));
        assert!(roots.matters(&batch(&[], true)), "events were lost");
        assert!(!roots.matters(&Batch { paths: vec![profile.dir().join("wormhole.db")], rescan: false }));
    }
}
