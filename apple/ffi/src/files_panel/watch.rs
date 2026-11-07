//! A folder followed through the system's file events, with a slow poll as the fallback.
//!
//! The OS delivers events on a thread of its own (`platform::watch`); the callback only wakes Swift,
//! coalesced (one `wake()` until `drain()`), and Swift debounces and reads: this layer decides nothing
//! about what changed. A watch can fail or stay silent (a network volume or a FUSE mount sends no
//! events, which core's `watch` refuses; the system can also drop events), so a timer thread wakes Swift
//! too: every few seconds when no stream runs, at a slow rate when one does. Off macOS there is no
//! watcher and the poll is all there is, as in the app.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use wormhole_core::platform::watch::{self as os_watch, Watch};

use crate::files_panel::listener::{PanelListener, Waker};

/// How often Swift is woken when no stream runs: the cadence the app's tab reads on its own tick.
const POLL_WITHOUT_STREAM: Duration = Duration::from_secs(2);
/// How often it is woken as a safety net while a stream runs: events can be lost without a word.
const POLL_WITH_STREAM: Duration = Duration::from_secs(30);

struct Shared {
    waker: Waker,
    live: AtomicBool,
    stopped: Mutex<bool>,
    changed: Condvar,
}

/// A running watch of one folder; dropping it, or `stop()`, ends it.
#[derive(uniffi::Object)]
pub struct FolderWatch {
    shared: Arc<Shared>,
    stream: Mutex<Option<Watch>>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

impl FolderWatch {
    /// Follows `root`; only the paths `keep` says yes to count as a change.
    pub(super) fn start(
        root: PathBuf,
        listener: Arc<dyn PanelListener>,
        keep: impl Fn(&Path) -> bool + Send + 'static,
    ) -> Arc<Self> {
        let shared = Arc::new(Shared {
            waker: Waker::new(listener),
            live: AtomicBool::new(false),
            stopped: Mutex::new(false),
            changed: Condvar::new(),
        });
        let on_event = Arc::clone(&shared);
        let stream = os_watch::watch(&[root.as_path()], move |batch| {
            if batch.rescan || batch.paths.iter().any(|path| keep(path)) {
                on_event.waker.wake();
            }
        });
        let stream = match stream {
            Ok(stream) => {
                shared.live.store(true, Ordering::Release);
                Some(stream)
            }
            Err(err) => {
                tracing::debug!("{} is polled, not watched: {err}", root.display());
                None
            }
        };
        let timer = Arc::clone(&shared);
        let poller = std::thread::Builder::new()
            .name("wormhole-ffi-files-poll".into())
            .spawn(move || timer.poll())
            .map_err(|err| tracing::warn!("the files poll did not start: {err}"))
            .ok();
        Arc::new(Self { shared, stream: Mutex::new(stream), poller: Mutex::new(poller) })
    }
}

impl FolderWatch {
    /// A change the owner of the watch made itself (a trust, a switch): there is no file event for it.
    pub(super) fn poke(&self) {
        self.shared.waker.wake();
    }
}

impl Shared {
    fn poll(&self) {
        let mut stopped = self.stopped.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            let every =
                if self.live.load(Ordering::Acquire) { POLL_WITH_STREAM } else { POLL_WITHOUT_STREAM };
            let (guard, timeout) =
                self.changed.wait_timeout(stopped, every).unwrap_or_else(PoisonError::into_inner);
            stopped = guard;
            if *stopped {
                return;
            }
            if timeout.timed_out() {
                self.waker.wake();
            }
        }
    }
}

#[uniffi::export]
impl FolderWatch {
    /// Swift read what changed: the next change wakes again.
    pub fn drain(&self) {
        self.shared.waker.rearm();
    }

    /// Whether the system's events are followed (otherwise the poll is all there is).
    pub fn is_live(&self) -> bool {
        self.shared.live.load(Ordering::Acquire)
    }

    /// Ends the watch and its timer. Later calls do nothing.
    pub fn stop(&self) {
        *self.shared.stopped.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.shared.changed.notify_all();
        self.stream.lock().unwrap_or_else(PoisonError::into_inner).take();
        let poller = self.poller.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(poller) = poller {
            let _ = poller.join();
        }
    }
}

impl Drop for FolderWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, reason = "the tests write into a temp folder to make events")]
mod tests {
    use std::time::{Duration, Instant};

    use super::FolderWatch;
    use crate::files_panel::listener::testing::Counting;

    fn wait_for(what: impl Fn() -> bool, secs: u64) -> bool {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            if what() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        what()
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_change_under_the_root_wakes_once_until_drained() {
        let dir = tempfile::tempdir().expect("temp dir");
        let counting = Counting::new();
        let watch = FolderWatch::start(dir.path().to_path_buf(), counting.clone(), |_| true);
        assert!(watch.is_live(), "a local temp folder is followed through FSEvents");
        std::fs::write(dir.path().join("a.txt"), "a").expect("write");
        assert!(wait_for(|| counting.count() == 1, 10), "the change woke Swift");
        std::fs::write(dir.path().join("b.txt"), "b").expect("write");
        std::thread::sleep(Duration::from_millis(600));
        assert_eq!(counting.count(), 1, "coalesced until drained");
        watch.drain();
        std::fs::write(dir.path().join("c.txt"), "c").expect("write");
        assert!(wait_for(|| counting.count() == 2, 10), "drained, so the next change wakes again");
    }

    #[test]
    fn a_root_that_is_not_there_is_polled_and_stops_with_the_watch() {
        let counting = Counting::new();
        let watch = FolderWatch::start("/nonexistent/Wormhole-swift-test".into(), counting.clone(), |_| true);
        assert!(!watch.is_live(), "nothing to follow");
        assert!(wait_for(|| counting.count() == 1, 6), "the fallback poll wakes it");
        watch.stop();
        watch.stop();
    }
}
