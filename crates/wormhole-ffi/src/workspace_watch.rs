//! The repositories the terminals are in, followed through their files, so git is read when
//! something in a repository changed instead of on a timer.
//!
//! One watch per repository root. The OS delivers events on a thread of its own; its callback sorts
//! them with the pure rules (`git::watch::on_batch`) and only leaves marks and wakes the pump, which
//! applies the debounce (`git::watch::Debounce`) and tells the model. A root no terminal is in any more
//! is dropped, its stream stopped. A watch that cannot start (a network volume, a repository with no
//! `.git` yet) is tried again after [`RETRY`], and until it runs the model's reads keep their fast
//! rates (`GitFeed::set_watched`).

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use wormhole_core::git::watch::{Debounce, Filter, Ignored, Layout, Marks, Relevance, on_batch};
use wormhole_core::platform::watch::{self as os_watch, Watch};

use crate::workspace_pump::Pump;

/// How long after a watch failed to start another try is made.
const RETRY: Duration = Duration::from_secs(60);

enum Entry {
    Live(Box<Live>),
    /// When it failed, for the retry.
    Failed(Instant),
}

struct Live {
    _watch: Watch,
    marks: Arc<Marks>,
    debounce: Debounce,
    filter: Arc<RwLock<Filter>>,
    layout: Layout,
    /// A listing of ignored paths is running; one more is not started meanwhile (the slow read of the
    /// repository is the backstop for what that misses).
    relisting: Arc<AtomicBool>,
}

#[derive(Default)]
pub(crate) struct Watches {
    roots: HashMap<PathBuf, Entry>,
}

/// What a round of [`Watches::sync`] found.
#[derive(Default)]
pub(crate) struct Synced {
    /// Roots with file events that matter, debounced: read them.
    pub changed: Vec<PathBuf>,
    /// Roots now followed by a running watch, and roots that no longer are.
    pub live: Vec<(PathBuf, bool)>,
    /// When a debounced event is due, if one waits.
    pub due: Option<Instant>,
}

impl Watches {
    /// Follow exactly `wanted`; collect what the running watches saw.
    pub(crate) fn sync(&mut self, wanted: &[PathBuf], pump: &Arc<Pump>, now: Instant) -> Synced {
        let mut synced = Synced::default();
        self.roots.retain(|root, entry| {
            let keep = wanted.contains(root)
                && !matches!(entry, Entry::Failed(at) if now.duration_since(*at) >= RETRY);
            if !keep && matches!(entry, Entry::Live(_)) {
                synced.live.push((root.clone(), false));
            }
            keep
        });
        for root in wanted {
            if self.roots.contains_key(root) {
                continue;
            }
            match start(root, pump) {
                Ok(live) => {
                    synced.live.push((root.clone(), true));
                    self.roots.insert(root.clone(), Entry::Live(Box::new(live)));
                }
                Err(err) => {
                    // `Unsupported` is the normal answer off macOS and on a network volume, `NotFound`
                    // a folder that is no repository (yet): the reads' own rates carry on.
                    if !matches!(err.kind(), io::ErrorKind::Unsupported | io::ErrorKind::NotFound) {
                        eprintln!("wormhole-ffi: not following {}: {err}", root.display());
                    }
                    self.roots.insert(root.clone(), Entry::Failed(now));
                }
            }
        }
        for (root, entry) in &mut self.roots {
            let Entry::Live(live) = entry else { continue };
            if !live.marks.pending() {
                continue;
            }
            live.debounce.event(live.marks.last());
            if live.debounce.take(now) {
                let (found, relist) = live.marks.take();
                if found != Relevance::None {
                    synced.changed.push(root.clone());
                }
                if found == Relevance::Rules || relist {
                    relist_ignored(live);
                }
            } else if let Some(due) = live.debounce.deadline() {
                synced.due = Some(synced.due.map_or(due, |known| known.min(due)));
            }
        }
        synced
    }
}

/// Read the layout and open the stream.
fn start(root: &Path, pump: &Arc<Pump>) -> io::Result<Live> {
    let layout = Layout::resolve(root)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no .git git would accept"))?;
    // Until the ignored paths are listed everything counts: more reads, never a missed one.
    let filter = Arc::new(RwLock::new(Filter::new(layout.clone(), Ignored::default())));
    let marks = Arc::new(Marks::new());
    let watch = os_watch::watch(&layout.roots(), {
        let (filter, marks, pump) = (filter.clone(), marks.clone(), pump.clone());
        move |batch| {
            if on_batch(&filter, &marks, &batch) {
                pump.kick();
            }
        }
    })?;
    let live =
        Live { _watch: watch, marks, debounce: Debounce::new(), filter, layout, relisting: Arc::default() };
    relist_ignored(&live);
    Ok(live)
}

/// List the ignored paths again, off the pump's thread (a large work tree takes a moment).
fn relist_ignored(live: &Live) {
    if live.relisting.swap(true, Ordering::AcqRel) {
        return;
    }
    let (filter, layout, relisting) = (live.filter.clone(), live.layout.clone(), live.relisting.clone());
    let spawned = std::thread::Builder::new().name("wormhole-ffi-ignored".into()).spawn({
        let relisting = relisting.clone();
        move || {
            // A failed or oversized listing leaves the filter as it was.
            if let Ok(ignored) = Ignored::read(layout.work_tree())
                && let Ok(mut filter) = filter.write()
            {
                *filter = Filter::new(layout, ignored);
            }
            relisting.store(false, Ordering::Release);
        }
    });
    if spawned.is_err() {
        relisting.store(false, Ordering::Release);
    }
}
