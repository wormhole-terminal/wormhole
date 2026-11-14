//! Learning that a repository changed without asking git on a timer: core's rules (`git::watch`: which
//! paths matter, when to read after them) over the OS's file events (`platform::watch`), one thread per
//! watch. The OS callback only classifies and queues; the thread sleeps in the queue until an event or the
//! debounce's deadline, then calls the listener once. When the watch cannot start (a network volume, a
//! folder that is no repository yet) or is quiet for a long time, a slow poll calls it anyway, as core's
//! rules ask of every watched fact: a watch can drop events.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use wormhole_core::git::find_root;
use wormhole_core::git::watch::{Debounce, Filter, Ignored, Layout, Relevance};
use wormhole_core::platform::watch::{self as os_watch, Batch, Watch};

use super::error::GitBackendError;

/// How long a live watch may be quiet before the listener is called anyway (a lost event).
const POLL_LIVE: Duration = Duration::from_secs(30);
/// The same while no watch runs: the panel's fast backstop rate.
const POLL_DOWN: Duration = Duration::from_secs(3);
/// How long after a watch failed to start another try is made.
const RETRY: Duration = Duration::from_secs(60);

/// What Swift implements to hear that the repository changed. Called from the watch's thread, with no lock
/// held; it must only note the change and hop to its own queue.
#[uniffi::export(with_foreign)]
pub trait GitListener: Send + Sync {
    fn changed(&self);
}

enum Msg {
    /// The strongest thing a batch meant, and whether the ignored list may be stale.
    Batch(Relevance, bool),
    Stop,
}

/// A repository followed until it is stopped or dropped.
#[derive(uniffi::Object)]
pub struct GitWatch {
    queue: Sender<Msg>,
    live: Arc<AtomicBool>,
}

#[uniffi::export]
impl GitWatch {
    /// Follow the repository `folder` is in. Fails at once only when it is in none; a watch the system
    /// refuses is the poll's business (`is_live` says which it is).
    #[uniffi::constructor]
    pub fn start(folder: String, listener: Arc<dyn GitListener>) -> Result<Arc<Self>, GitBackendError> {
        crate::isolation::require()?;
        let root =
            find_root(&crate::files_panel::names::folder(&folder)).ok_or(GitBackendError::NotARepository)?;
        let (queue, inbox) = mpsc::channel();
        let live = Arc::new(AtomicBool::new(false));
        let worker = Worker { root, inbox, queue: queue.clone(), listener, live: live.clone() };
        std::thread::Builder::new()
            .name("wormhole-ffi-git-watch".into())
            .spawn(move || worker.run())
            .map_err(|err| GitBackendError::failed(format!("could not start the watch: {err}")))?;
        Ok(Arc::new(Self { queue, live }))
    }

    /// Whether the OS's events are being followed, rather than only the slow poll.
    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire)
    }

    /// End the watch and its thread. Also what dropping it does.
    pub fn stop(&self) {
        let _ = self.queue.send(Msg::Stop);
    }
}

impl Drop for GitWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A watch that started: held for its drop (the OS stream stops), with the rules its callback reads.
struct Started {
    _watch: Watch,
    filter: Arc<RwLock<Filter>>,
    layout: Layout,
}

struct Worker {
    root: PathBuf,
    inbox: mpsc::Receiver<Msg>,
    queue: Sender<Msg>,
    listener: Arc<dyn GitListener>,
    live: Arc<AtomicBool>,
}

impl Worker {
    fn run(self) {
        let mut started = self.begin();
        let mut tried = Instant::now();
        let mut debounce = Debounce::new();
        let mut rules = false;
        loop {
            let live = started.is_some();
            self.live.store(live, Ordering::Release);
            let wait = match debounce.deadline() {
                Some(due) => due.saturating_duration_since(Instant::now()),
                None if live => POLL_LIVE,
                None => POLL_DOWN,
            };
            match self.inbox.recv_timeout(wait) {
                Ok(Msg::Batch(found, stale)) => {
                    debounce.event(Instant::now());
                    rules |= found == Relevance::Rules || stale;
                }
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) if debounce.pending() => {
                    if debounce.take(Instant::now()) {
                        if std::mem::take(&mut rules)
                            && let Some(started) = &started
                        {
                            relist(started);
                        }
                        self.listener.changed();
                    }
                }
                // Quiet for the whole wait: the backstop, and a new try at a watch that failed.
                Err(RecvTimeoutError::Timeout) => {
                    if started.is_none() && tried.elapsed() >= RETRY {
                        started = self.begin();
                        tried = Instant::now();
                    }
                    self.listener.changed();
                }
            }
        }
    }

    /// Resolve the layout and open the stream. Until the ignored paths are listed everything counts: more
    /// reads, never a missed one.
    fn begin(&self) -> Option<Started> {
        let layout = Layout::resolve(&self.root)?;
        let filter = Arc::new(RwLock::new(Filter::new(layout.clone(), Ignored::default())));
        let (rules, queue) = (filter.clone(), self.queue.clone());
        let watch = os_watch::watch(&layout.roots(), move |batch| {
            if let Some((found, stale)) = classify(&rules, &batch) {
                let _ = queue.send(Msg::Batch(found, stale));
            }
        })
        .map_err(|err| tracing::debug!(root = ?self.root, "not following the repository's files: {err}"))
        .ok()?;
        let started = Started { _watch: watch, filter, layout };
        relist(&started);
        Some(started)
    }
}

/// Read the ignored paths again; a failed or oversized listing leaves the filter as it was.
fn relist(started: &Started) {
    match Ignored::read(started.layout.work_tree()) {
        Ok(ignored) => {
            if let Ok(mut filter) = started.filter.write() {
                *filter = Filter::new(started.layout.clone(), ignored);
            }
        }
        Err(err) => tracing::debug!("ignored paths not listed: {err:#}"),
    }
}

/// What one batch means, on the OS's thread: quick, no blocking but a `stat` of a path directly under the
/// work tree. `None` when nothing in it matters to `git status`.
fn classify(filter: &RwLock<Filter>, batch: &Batch) -> Option<(Relevance, bool)> {
    let Ok(filter) = filter.read() else { return Some((Relevance::Status, false)) };
    let mut found = filter.classify_all(batch.paths.iter().map(PathBuf::as_path));
    if found == Relevance::TopLevel {
        // A new folder may be an ignored one the list lacks (`node_modules`): list again.
        let folder = batch.paths.iter().any(|p| filter.classify(p) == Relevance::TopLevel && p.is_dir());
        found = if folder { Relevance::Rules } else { Relevance::Status };
    }
    let mut stale = false;
    if batch.rescan {
        // Events were lost: with no path anything may have changed, else what is under them.
        if batch.paths.is_empty() {
            found = Relevance::Rules;
        } else {
            found = found.max(Relevance::Status);
            stale = true;
        }
    }
    (found != Relevance::None).then_some((found, stale))
}
