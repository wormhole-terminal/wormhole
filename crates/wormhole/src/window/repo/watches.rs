//! Repositories followed through file events, so a terminal's git state is read when something in the
//! repository changed instead of on a timer.
//!
//! One watch per repository root, shared by every terminal in it across all windows: a window
//! [`claim`](RepoWatches::claim)s the roots its terminals are in on each tick, and a root nobody claims
//! any more (for a few seconds, so a `cd` out and back does not restart it) is dropped: its OS stream
//! stopped, its tasks cancelled. A window that closes lets go of its claims at once. It is a global of its
//! own and not an `AppState` field, which may only shrink.
//!
//! The pieces: the OS delivers events on a thread of its own (`platform::watch`), a callback there sorts
//! them with the pure rules (`git::watch::Filter`) and only leaves marks, and a task on the UI executor
//! applies the debounce (`git::watch::Debounce`) and tells the windows. Starting a watch (reading the
//! `.git`, opening the stream) happens off the UI thread. Polling stays as the fallback
//! (`GitState::poll`): a root whose watch is not running is not [`live`](RepoWatches::live), and the
//! polling then keeps its fast rates; a watch that failed is tried again after [`RETRY`].

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::git::watch::{Debounce, Filter, Ignored, Layout, Relevance};
use wormhole_core::platform::watch::{self as os_watch, Batch, Watch};

use crate::window::Workspace;

/// How long a root nobody claims is kept: a few ticks.
pub(super) const GRACE: Duration = Duration::from_secs(2);

/// How long after a watch failed to start another try is made (a volume that was not mounted yet, a
/// repository that had no `.git` yet).
const RETRY: Duration = Duration::from_secs(60);

/// What the callback on the OS's thread leaves for the task: the strongest thing seen since the last
/// read, and when the last event came. Atomics, so a flood of events costs a few stores and never a
/// queue that grows.
struct Marks {
    started: Instant,
    /// 0 nothing, 1 status, 2 the ignore rules changed too.
    level: AtomicU8,
    /// Milliseconds after `started` of the last event that mattered.
    last_ms: AtomicU64,
    /// The list of ignored paths may be stale whatever `level` says (the system lost events).
    relist: AtomicBool,
}

impl Marks {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            level: AtomicU8::new(0),
            last_ms: AtomicU64::new(0),
            relist: AtomicBool::new(false),
        }
    }

    fn mark(&self, found: Relevance) {
        let level = match found {
            Relevance::None => return,
            Relevance::Status | Relevance::TopLevel => 1,
            Relevance::Rules => 2,
        };
        self.level.fetch_max(level, Ordering::AcqRel);
        self.last_ms.store(self.started.elapsed().as_millis() as u64, Ordering::Release);
    }

    fn mark_relist(&self) {
        self.relist.store(true, Ordering::Release);
    }

    /// Whether something happened since the last [`take`](Self::take).
    fn pending(&self) -> bool {
        self.level.load(Ordering::Acquire) != 0
    }

    fn last(&self) -> (u64, Instant) {
        let ms = self.last_ms.load(Ordering::Acquire);
        (ms, self.started + Duration::from_millis(ms))
    }

    /// What was marked since the last call, and whether the ignored list is stale.
    fn take(&self) -> (Relevance, bool) {
        let found = match self.level.swap(0, Ordering::AcqRel) {
            0 => Relevance::None,
            1 => Relevance::Status,
            _ => Relevance::Rules,
        };
        (found, self.relist.swap(false, Ordering::AcqRel))
    }
}

/// Keeps the filter's list of ignored paths current: one listing at a time per root, and when more are
/// asked for meanwhile, one more after it, so the last answer is always from the last request and an
/// answer never overwrites a newer one. Only used on the UI thread.
struct Relister {
    filter: Arc<RwLock<Filter>>,
    layout: Layout,
    busy: Cell<bool>,
    again: Cell<bool>,
}

impl Relister {
    fn new(filter: Arc<RwLock<Filter>>, layout: Layout) -> Rc<Self> {
        Rc::new(Self { filter, layout, busy: Cell::new(false), again: Cell::new(false) })
    }

    /// Whether the caller starts [`run`](Self::run) (none is running), else one more round is noted.
    fn begin(&self) -> bool {
        if self.busy.replace(true) {
            self.again.set(true);
            return false;
        }
        true
    }

    fn request(self: &Rc<Self>, cx: &mut App) {
        if self.begin() {
            let (this, bg) = (self.clone(), cx.background_executor().clone());
            cx.spawn(async move |_| this.run(bg).await).detach();
        }
    }

    fn request_async(self: &Rc<Self>, cx: &mut AsyncApp) {
        if self.begin() {
            let (this, bg) = (self.clone(), cx.background_executor().clone());
            cx.spawn(async move |_| this.run(bg).await).detach();
        }
    }

    async fn run(self: Rc<Self>, bg: BackgroundExecutor) {
        loop {
            self.again.set(false);
            let work_tree = self.layout.work_tree().to_path_buf();
            // A failed or oversized listing leaves the filter as it was.
            match bg.spawn(async move { Ignored::read(&work_tree) }).await {
                Ok(ignored) => {
                    if let Ok(mut filter) = self.filter.write() {
                        *filter = Filter::new(self.layout.clone(), ignored);
                    }
                }
                Err(err) => tracing::debug!("ignored paths not listed: {err:#}"),
            }
            if !self.again.get() {
                break;
            }
        }
        self.busy.set(false);
    }
}

/// A watch that started, with what its tasks need (made off the UI thread).
struct Started {
    watch: Watch,
    layout: Layout,
    filter: Arc<RwLock<Filter>>,
    marks: Arc<Marks>,
    woken: smol::channel::Receiver<()>,
}

enum State {
    Starting,
    Live,
    /// When it failed, for the retry.
    Failed(Instant),
}

/// One followed root.
struct Entry {
    owners: Vec<WeakEntity<Workspace>>,
    state: State,
    /// Since when nobody claims it.
    idle_since: Option<Instant>,
    /// Held for their drop: the start is cancelled, the stream stops, the tasks end.
    _starter: Option<Task<()>>,
    _watch: Option<Watch>,
    _tasks: Vec<Task<()>>,
}

/// The followed repositories (see the module). Created on first use.
#[derive(Default)]
pub(crate) struct RepoWatches {
    roots: HashMap<PathBuf, Entry>,
    /// The workspaces that have a release hook, so a closing window lets go of its claims.
    hooked: HashSet<EntityId>,
}

impl Global for RepoWatches {}

impl RepoWatches {
    /// This window's terminals are in exactly `roots` now: start following the new ones, and let go of
    /// those nobody is in. Cheap to call on every tick.
    pub(crate) fn claim(roots: &[PathBuf], cx: &mut Context<Workspace>) {
        let (owner, id) = (cx.weak_entity(), cx.entity_id());
        if cx.default_global::<Self>().hooked.insert(id) {
            cx.on_release(move |_, cx| Self::release(id, cx)).detach();
        }
        let now = Instant::now();
        let this = cx.global_mut::<Self>();
        let mut restart = Vec::new();
        this.roots.retain(|root, entry| {
            let wanted = roots.contains(root);
            let held = entry.owners.iter().position(|o| *o == owner);
            match (wanted, held) {
                (false, Some(at)) => {
                    entry.owners.swap_remove(at);
                }
                (true, None) => entry.owners.push(owner.clone()),
                _ => {}
            }
            // A closed window's claim goes with it.
            entry.owners.retain(|o| o.upgrade().is_some());
            if entry.owners.is_empty() {
                let since = *entry.idle_since.get_or_insert(now);
                return now.duration_since(since) < GRACE;
            }
            entry.idle_since = None;
            if matches!(entry.state, State::Failed(at) if now.duration_since(at) >= RETRY) {
                restart.push(root.clone());
                return false;
            }
            true
        });
        for root in roots {
            if !cx.global::<Self>().roots.contains_key(root) {
                let entry = start(root, owner.clone(), cx);
                cx.global_mut::<Self>().roots.insert(root.clone(), entry);
            }
        }
    }

    /// A workspace is going away: its claims end now, and with the last claim the watch.
    fn release(id: EntityId, cx: &mut App) {
        let this = cx.global_mut::<Self>();
        this.hooked.remove(&id);
        this.roots.retain(|_, entry| {
            entry.owners.retain(|o| o.entity_id() != id && o.upgrade().is_some());
            !entry.owners.is_empty()
        });
    }

    /// Whether `root` is followed by a running watch, so polling is only the backstop.
    pub(crate) fn live(root: &Path, cx: &App) -> bool {
        cx.try_global::<Self>()
            .and_then(|this| this.roots.get(root))
            .is_some_and(|entry| matches!(entry.state, State::Live))
    }

    fn owners(root: &Path, cx: &App) -> Vec<WeakEntity<Workspace>> {
        cx.try_global::<Self>()
            .and_then(|this| this.roots.get(root))
            .map_or_else(Vec::new, |entry| entry.owners.clone())
    }
}

/// Begin following `root`: an entry that says it is starting, while the work happens in the background.
fn start(root: &Path, owner: WeakEntity<Workspace>, cx: &mut App) -> Entry {
    let (bg, root) = (cx.background_executor().clone(), root.to_path_buf());
    let starter = cx.spawn(async move |cx| {
        let work = root.clone();
        let started = bg.spawn(async move { begin(&work) }).await;
        cx.update(|cx| install(root, started, cx));
    });
    Entry {
        owners: vec![owner],
        state: State::Starting,
        idle_since: None,
        _starter: Some(starter),
        _watch: None,
        _tasks: Vec::new(),
    }
}

/// Read the repository's layout and open the stream. Runs off the UI thread.
fn begin(root: &Path) -> io::Result<Started> {
    let layout = Layout::resolve(root)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no .git git would accept"))?;
    // Until the ignored paths are listed everything counts: more reads, never a missed one.
    let filter = Arc::new(RwLock::new(Filter::new(layout.clone(), Ignored::default())));
    let marks = Arc::new(Marks::new());
    let (wake, woken) = smol::channel::bounded::<()>(1);
    let watch = os_watch::watch(&layout.roots(), {
        let (filter, marks) = (filter.clone(), marks.clone());
        move |batch| {
            if on_batch(&filter, &marks, &batch) {
                // Full already means a wake-up is on its way.
                let _ = wake.try_send(());
            }
        }
    })?;
    Ok(Started { watch, layout, filter, marks, woken })
}

/// What one event means, left in `marks`. Whether it mattered. Runs on the OS's thread: quick, no
/// blocking but a `stat` of a path directly under the work tree.
fn on_batch(filter: &RwLock<Filter>, marks: &Marks, batch: &Batch) -> bool {
    let Ok(filter) = filter.read() else {
        marks.mark(Relevance::Status);
        return true;
    };
    let paths = batch.paths.iter().map(PathBuf::as_path);
    let mut found = filter.classify_all(paths);
    if found == Relevance::TopLevel {
        // A new folder may be an ignored one the list lacks (`node_modules`): list again. A new file
        // is only a change.
        let folder =
            batch.paths.iter().any(|path| filter.classify(path) == Relevance::TopLevel && path.is_dir());
        found = if folder { Relevance::Rules } else { Relevance::Status };
    }
    if batch.rescan {
        // Events were lost under these paths. With none named anything may have changed, the ignore
        // rules too; otherwise what is under them is read, and the ignored list checked again.
        if batch.paths.is_empty() {
            found = Relevance::Rules;
        } else {
            found = found.max(Relevance::Status);
            marks.mark_relist();
        }
    }
    marks.mark(found);
    found != Relevance::None
}

/// A start came back: run the tasks of a watch that started, or note the failure.
fn install(root: PathBuf, started: io::Result<Started>, cx: &mut App) {
    if !cx.global::<RepoWatches>().roots.contains_key(&root) {
        return; // nobody wants it any more; dropping `started` stops the stream
    }
    let (state, watch, tasks) = match started {
        Ok(started) => {
            let relister = Relister::new(started.filter.clone(), started.layout);
            relister.request(cx);
            let follow = cx.spawn({
                let root = root.clone();
                async move |cx| follow(root, relister, started.woken, started.marks, cx).await
            });
            (State::Live, Some(started.watch), vec![follow])
        }
        Err(err) => {
            // `Unsupported` is the normal answer off macOS and on a network volume, `NotFound` a folder
            // that is no repository (yet); anything else is worth a line.
            if !matches!(err.kind(), io::ErrorKind::Unsupported | io::ErrorKind::NotFound) {
                tracing::warn!(root = ?root, "not following the repository's files: {err}");
            }
            (State::Failed(Instant::now()), None, Vec::new())
        }
    };
    if let Some(entry) = cx.global_mut::<RepoWatches>().roots.get_mut(&root) {
        (entry.state, entry._watch, entry._tasks) = (state, watch, tasks);
    }
}

/// Wait for marks, debounce them, and tell the windows. Ends when the watch is dropped (the channel
/// closes with the callback that held its sender) or the entry's task is cancelled.
async fn follow(
    root: PathBuf,
    relister: Rc<Relister>,
    woken: smol::channel::Receiver<()>,
    marks: Arc<Marks>,
    cx: &mut AsyncApp,
) {
    let mut debounce = Debounce::new();
    loop {
        // A wake-up can be left over from events an earlier round already took: only marks count.
        if !marks.pending() {
            if woken.recv().await.is_err() {
                break;
            }
            continue;
        }
        let (mut seen, last) = marks.last();
        debounce.event(last);
        // Sleep to the deadline and look again: events that came meanwhile move it, up to the cap.
        let (found, relist) = loop {
            let Some(due) = debounce.deadline() else { break (Relevance::None, false) };
            let now = Instant::now();
            if due > now {
                cx.background_executor().timer(due - now).await;
            }
            let (ms, last) = marks.last();
            if ms != seen {
                seen = ms;
                debounce.event(last);
            }
            if debounce.take(Instant::now()) {
                break marks.take();
            }
        };
        if found == Relevance::None {
            continue;
        }
        let owners = cx.update(|cx| RepoWatches::owners(&root, cx));
        for owner in owners {
            let root = root.clone();
            let _ = owner.update(cx, |workspace, cx| workspace.repo_changed(&root, cx));
        }
        if found == Relevance::Rules || relist {
            relister.request_async(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use wormhole_core::git::watch::{Filter, Ignored, Layout, Relevance};
    use wormhole_core::platform::watch::Batch;

    use super::{Marks, on_batch};

    #[::core::prelude::v1::test]
    fn marks_are_taken_once_and_the_strongest_wins() {
        let marks = Marks::new();
        assert!(!marks.pending());
        marks.mark(Relevance::None);
        assert!(!marks.pending(), "nothing that matters leaves no mark");
        marks.mark(Relevance::Status);
        marks.mark(Relevance::Rules);
        marks.mark(Relevance::Status);
        marks.mark_relist();
        assert!(marks.pending());
        assert_eq!(marks.take(), (Relevance::Rules, true));
        assert!(!marks.pending(), "a wake-up left over finds nothing to do");
        assert_eq!(marks.take(), (Relevance::None, false));
    }

    #[::core::prelude::v1::test]
    fn a_lost_batch_with_no_path_asks_for_everything_and_one_with_paths_only_those() {
        let filter = RwLock::new(Filter::new(
            Layout::new("/w/app".into(), "/w/app/.git".into(), "/w/app/.git".into()),
            Ignored::parse(b"target/\0"),
        ));
        let marks = Marks::new();
        assert!(on_batch(&filter, &marks, &Batch { paths: Vec::new(), rescan: true }));
        assert_eq!(marks.take(), (Relevance::Rules, false));
        let lost = Batch { paths: vec!["/w/app/src".into()], rescan: true };
        assert!(on_batch(&filter, &marks, &lost));
        assert_eq!(
            marks.take(),
            (Relevance::Status, true),
            "read what is under it, list ignored paths again"
        );
        let quiet = Batch { paths: vec!["/w/app/target/x".into()], rescan: false };
        assert!(!on_batch(&filter, &marks, &quiet));
        assert!(!marks.pending());
    }
}
