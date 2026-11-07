//! What a watch's callback leaves for the thread that reads: the strongest thing seen since the last
//! read, and when the last event came. The OS delivers events on a thread of its own, one callback
//! per event, so the callback only sorts the batch with the filter and leaves marks (atomics: a flood
//! of events costs a few stores and never a queue that grows); the reader applies the
//! [`Debounce`](super::Debounce) and reads once.

use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::platform::watch::Batch;

use super::{Filter, Relevance};

#[derive(Debug)]
pub struct Marks {
    started: Instant,
    /// 0 nothing, 1 status, 2 the ignore rules changed too.
    level: AtomicU8,
    /// Milliseconds after `started` of the last event that mattered.
    last_ms: AtomicU64,
    /// The list of ignored paths may be stale whatever `level` says (the system lost events).
    relist: AtomicBool,
}

impl Default for Marks {
    fn default() -> Self {
        Self::new()
    }
}

impl Marks {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            level: AtomicU8::new(0),
            last_ms: AtomicU64::new(0),
            relist: AtomicBool::new(false),
        }
    }

    pub fn mark(&self, found: Relevance) {
        let level = match found {
            Relevance::None => return,
            Relevance::Status | Relevance::TopLevel => 1,
            Relevance::Rules => 2,
        };
        self.level.fetch_max(level, Ordering::AcqRel);
        self.last_ms.store(self.started.elapsed().as_millis() as u64, Ordering::Release);
    }

    pub fn mark_relist(&self) {
        self.relist.store(true, Ordering::Release);
    }

    /// Whether something happened since the last [`take`](Self::take).
    pub fn pending(&self) -> bool {
        self.level.load(Ordering::Acquire) != 0
    }

    /// When the last event that mattered came.
    pub fn last(&self) -> Instant {
        self.started + Duration::from_millis(self.last_ms.load(Ordering::Acquire))
    }

    /// What was marked since the last call, and whether the ignored list is stale.
    pub fn take(&self) -> (Relevance, bool) {
        let found = match self.level.swap(0, Ordering::AcqRel) {
            0 => Relevance::None,
            1 => Relevance::Status,
            _ => Relevance::Rules,
        };
        (found, self.relist.swap(false, Ordering::AcqRel))
    }
}

/// What one batch of events means, left in `marks`. Whether it mattered. Runs on the OS's thread:
/// quick, no blocking but a `stat` of a path directly under the work tree.
pub fn on_batch(filter: &RwLock<Filter>, marks: &Marks, batch: &Batch) -> bool {
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

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use super::super::{Ignored, Layout};
    use super::*;

    #[test]
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

    #[test]
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
