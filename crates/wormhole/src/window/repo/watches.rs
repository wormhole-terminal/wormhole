//! Repositories followed through file events, so a terminal's git state is read when something in the
//! repository changed instead of on a timer.
//!
//! One watch per repository root, shared by every terminal in it across all windows: a window
//! [`claim`](RepoWatches::claim)s the roots its terminals are in on each tick, and a root nobody claims
//! any more is dropped (its OS stream stopped, its task cancelled). It is a global of its own and not an
//! `AppState` field, which may only shrink.
//!
//! The pieces: the OS delivers events on a thread of its own (`platform::watch`), a callback there
//! sorts them with the pure rules (`git::watch::Filter`) and only leaves a mark, and a task on the UI
//! executor applies the debounce (`git::watch::Debounce`) and tells the windows. Polling stays as the
//! fallback (`GitState::poll`): a root whose watch could not start is not [`live`](RepoWatches::live),
//! and the polling then keeps its fast rates.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::git::watch::{Debounce, Filter, Ignored, Layout, Relevance};
use wormhole_core::platform::watch::{self as os_watch, Watch};

use crate::window::Workspace;

/// What the callback on the OS's thread leaves for the task: the strongest thing seen since the last
/// read, and when the last event came. Atomics, so a flood of events costs a few stores and never a
/// queue that grows.
struct Marks {
    started: Instant,
    /// 0 nothing, 1 status, 2 the ignore rules changed too.
    level: AtomicU8,
    /// Milliseconds after `started` of the last event that mattered.
    last_ms: AtomicU64,
}

impl Marks {
    fn new() -> Self {
        Self { started: Instant::now(), level: AtomicU8::new(0), last_ms: AtomicU64::new(0) }
    }

    fn mark(&self, found: Relevance) {
        let level = match found {
            Relevance::None => return,
            Relevance::Status => 1,
            Relevance::Rules => 2,
        };
        self.level.fetch_max(level, Ordering::AcqRel);
        self.last_ms.store(self.started.elapsed().as_millis() as u64, Ordering::Release);
    }

    fn last(&self) -> (u64, Instant) {
        let ms = self.last_ms.load(Ordering::Acquire);
        (ms, self.started + Duration::from_millis(ms))
    }

    /// What was marked since the last call.
    fn take(&self) -> Relevance {
        match self.level.swap(0, Ordering::AcqRel) {
            0 => Relevance::None,
            1 => Relevance::Status,
            _ => Relevance::Rules,
        }
    }
}

/// One followed root.
struct Entry {
    owners: Vec<WeakEntity<Workspace>>,
    /// Whether the watch started. A root whose watch failed (a network volume, a folder in no usable
    /// repository layout, no FSEvents) stays here so it is not retried every tick, and polls fast.
    live: bool,
    /// Held for their drop: the stream stops, the tasks are cancelled.
    _watch: Option<Watch>,
    _tasks: Vec<Task<()>>,
}

/// The followed repositories (see the module). Created on first use.
#[derive(Default)]
pub(crate) struct RepoWatches {
    roots: HashMap<PathBuf, Entry>,
}

impl Global for RepoWatches {}

impl RepoWatches {
    /// `owner`'s terminals are in exactly `roots` now: start following the new ones, and stop following
    /// those nobody is in. Cheap to call on every tick.
    pub(crate) fn claim(owner: &WeakEntity<Workspace>, roots: &[PathBuf], cx: &mut App) {
        let this = cx.default_global::<Self>();
        this.roots.retain(|root, entry| {
            let wanted = roots.contains(root);
            let held = entry.owners.iter().position(|o| o == owner);
            match (wanted, held) {
                (false, Some(at)) => {
                    entry.owners.swap_remove(at);
                }
                (true, None) => entry.owners.push(owner.clone()),
                _ => {}
            }
            // A closed window's claim goes with it, and with the last claim the watch.
            entry.owners.retain(|o| o.upgrade().is_some());
            !entry.owners.is_empty()
        });
        for root in roots {
            if !cx.global::<Self>().roots.contains_key(root) {
                let entry = start(root, owner.clone(), cx);
                cx.global_mut::<Self>().roots.insert(root.clone(), entry);
            }
        }
    }

    /// Whether `root` is followed by a running watch, so polling is only the backstop.
    pub(crate) fn live(root: &Path, cx: &App) -> bool {
        cx.try_global::<Self>().and_then(|this| this.roots.get(root)).is_some_and(|entry| entry.live)
    }

    fn owners(root: &Path, cx: &App) -> Vec<WeakEntity<Workspace>> {
        cx.try_global::<Self>()
            .and_then(|this| this.roots.get(root))
            .map_or_else(Vec::new, |e| e.owners.clone())
    }
}

/// Start following `root`; on failure an entry that says so.
fn start(root: &Path, owner: WeakEntity<Workspace>, cx: &mut App) -> Entry {
    let failed = |owner| Entry { owners: vec![owner], live: false, _watch: None, _tasks: Vec::new() };
    let Some(layout) = Layout::resolve(root) else { return failed(owner) };
    // Until the ignored paths are listed everything counts: more reads, never a missed one.
    let filter = Arc::new(RwLock::new(Filter::new(layout.clone(), Ignored::default())));
    let marks = Arc::new(Marks::new());
    let (wake, woken) = smol::channel::bounded::<()>(1);
    let started = os_watch::watch(&layout.roots(), {
        let (filter, marks) = (filter.clone(), marks.clone());
        move |batch| {
            let found = if batch.rescan {
                // Events were dropped: anything may have changed, the ignore rules too.
                Relevance::Rules
            } else {
                match filter.read() {
                    Ok(filter) => filter.classify_all(batch.paths.iter().map(PathBuf::as_path)),
                    Err(_) => Relevance::Status,
                }
            };
            if found != Relevance::None {
                marks.mark(found);
                // Full already means a wake-up is on its way.
                let _ = wake.try_send(());
            }
        }
    });
    let watch = match started {
        Ok(watch) => watch,
        Err(err) => {
            // `Unsupported` is the normal answer off macOS, anything else is worth a line.
            if err.kind() != std::io::ErrorKind::Unsupported {
                tracing::warn!(root = ?root, "not following the repository's files: {err}");
            }
            return failed(owner);
        }
    };
    let root = root.to_path_buf();
    let work_tree = layout.work_tree().to_path_buf();
    let listing = cx.spawn({
        let (filter, work_tree) = (filter.clone(), work_tree.clone());
        async move |cx| relist(&filter, layout, work_tree, cx).await
    });
    let follow = cx.spawn(async move |cx| follow(root, work_tree, woken, marks, filter, cx).await);
    Entry { owners: vec![owner], live: true, _watch: Some(watch), _tasks: vec![listing, follow] }
}

/// Read git's ignored paths in the background and put them into the filter. A failed or oversized
/// listing leaves the filter as it was.
async fn relist(filter: &RwLock<Filter>, layout: Layout, work_tree: PathBuf, cx: &mut AsyncApp) {
    let read = cx.background_executor().spawn(async move { Ignored::read(&work_tree) }).await;
    match read {
        Ok(ignored) => {
            if let Ok(mut filter) = filter.write() {
                *filter = Filter::new(layout, ignored);
            }
        }
        Err(err) => tracing::debug!("ignored paths not listed: {err:#}"),
    }
}

/// Wait for marks, debounce them, and tell the windows. Ends when the watch is dropped (the channel
/// closes with the callback that held its sender) or the entry's task is cancelled.
async fn follow(
    root: PathBuf,
    work_tree: PathBuf,
    woken: smol::channel::Receiver<()>,
    marks: Arc<Marks>,
    filter: Arc<RwLock<Filter>>,
    cx: &mut AsyncApp,
) {
    let mut debounce = Debounce::new();
    while woken.recv().await.is_ok() {
        let (mut seen, last) = marks.last();
        debounce.event(last);
        // Sleep to the deadline and look again: events that came meanwhile move it, up to the cap.
        let rules = loop {
            let Some(due) = debounce.deadline() else { break Relevance::None };
            let now = Instant::now();
            if due > now {
                cx.background_executor().timer(due - now).await;
            }
            let (ms, last) = marks.last();
            if ms != seen {
                (seen, _) = (ms, ());
                debounce.event(last);
            }
            if debounce.take(Instant::now()) {
                break marks.take();
            }
        };
        if rules == Relevance::None {
            continue; // woken by a mark an earlier read already took
        }
        let owners = cx.update(|cx| RepoWatches::owners(&root, cx));
        for owner in owners {
            let root = root.clone();
            let _ = owner.update(cx, |workspace, cx| workspace.repo_changed(&root, cx));
        }
        if rules == Relevance::Rules {
            let layout = filter.read().map(|filter| filter.layout().clone());
            if let Ok(layout) = layout {
                relist(&filter, layout, work_tree.clone(), cx).await;
            }
        }
    }
}
