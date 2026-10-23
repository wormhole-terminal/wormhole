//! The `+N` of untracked files: their line counts, remembered between refreshes.
//!
//! Git has no numbers for a file it does not track, so we read it. Every refresh of the active
//! session (every 2 s) used to read every untracked file again: with 3000 files of 8 KB that was
//! 39 ms of a 43 ms refresh, and model weights or logs in the tree made it far worse. A file is
//! now read only when its size, modification time or inode differs from the last refresh.

use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use super::{ChangeKind, FileChange, MAX_TEXT_BYTES};
use crate::platform::fs::{inode, mtime};
use crate::storage::files::{self, LinkPolicy};

/// A file modified more recently than this is counted again next time even if it looks the
/// same: a writer may be in the middle of it, and two writes within one timestamp tick and of
/// the same size would otherwise go unseen (git's own "racily clean" rule, in miniature).
const SETTLE: Duration = Duration::from_secs(2);

/// Roots whose counts are kept. A repository nobody refreshes any more only costs its last list.
const MAX_ROOTS: usize = 32;

/// What identifies a file's content for our purposes. Not a hash: a rewrite that keeps size,
/// mtime (to the nanosecond) and inode would be missed, and the settle rule covers the only
/// way that happens by accident.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    mtime: (i64, i64),
    ino: u64,
}

impl Stamp {
    fn of(meta: &Metadata) -> Self {
        Self { len: meta.len(), mtime: mtime(meta), ino: inode(meta) }
    }
}

#[derive(Clone, Copy)]
struct Counted {
    stamp: Stamp,
    lines: Option<u32>,
}

type PerRoot = HashMap<String, Counted>;

static CACHE: Mutex<Vec<(PathBuf, PerRoot)>> = Mutex::new(Vec::new());

// How many files were actually read on this thread, so a test can tell a cache hit from a read.
#[cfg(test)]
thread_local! {
    pub(super) static READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Fill in `added`/`removed` for the untracked files in `files` that have no count yet.
///
/// Only the files seen in this refresh are kept afterwards, so a file that is deleted, tracked
/// or ignored drops out of the cache by itself.
pub(super) fn count_untracked_lines(root: &Path, files: &mut [FileChange]) {
    let mut previous = take(root);
    let mut kept = PerRoot::new();
    for file in files.iter_mut().filter(|f| f.kind == ChangeKind::Untracked && f.added.is_none()) {
        let remembered = previous.remove(&file.path);
        file.added = count(&root.join(&file.path), remembered, &file.path, &mut kept);
        file.removed = Some(0);
    }
    put(root, kept);
}

fn count(path: &Path, remembered: Option<Counted>, key: &str, kept: &mut PerRoot) -> Option<u32> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    // Git shows an untracked link as one line, its target, and does not follow it.
    if meta.file_type().is_symlink() {
        return Some(1);
    }
    // Checked before anything is opened: a 200 MB file used to be read whole only to be given up
    // on, and a FIFO, socket or device is never opened (a FIFO blocks until someone writes).
    if !meta.is_file() || meta.len() > MAX_TEXT_BYTES {
        return None;
    }
    let stamp = Stamp::of(&meta);
    if let Some(known) = remembered
        && known.stamp == stamp
    {
        kept.insert(key.to_string(), known);
        return known.lines;
    }
    let lines = read_and_count(path);
    let settled =
        meta.modified().is_ok_and(|at| SystemTime::now().duration_since(at).is_ok_and(|age| age >= SETTLE));
    if settled {
        kept.insert(key.to_string(), Counted { stamp, lines });
    }
    lines
}

fn read_and_count(path: &Path) -> Option<u32> {
    #[cfg(test)]
    READS.with(|reads| reads.set(reads.get() + 1));
    let bytes = files::read_capped(path, MAX_TEXT_BYTES, LinkPolicy::Follow).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let lines = bytes.iter().filter(|&&b| b == b'\n').count();
    let trailing = !bytes.is_empty() && !bytes.ends_with(b"\n");
    Some((lines + trailing as usize) as u32)
}

/// The counts remembered for `root`, taken out of the cache (so no lock is held while reading
/// files, and two refreshes of one repository never share a map).
fn take(root: &Path) -> PerRoot {
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    match cache.iter().position(|(r, _)| r == root) {
        Some(at) => cache.swap_remove(at).1,
        None => PerRoot::new(),
    }
}

fn put(root: &Path, counts: PerRoot) {
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    cache.retain(|(r, _)| r != root);
    if cache.len() >= MAX_ROOTS {
        cache.remove(0);
    }
    if !counts.is_empty() {
        cache.push((root.to_path_buf(), counts));
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    use super::{READS, count_untracked_lines};
    use crate::git::testing::{init, write};
    use crate::git::{ChangeKind, FileChange};

    fn untracked(path: &str) -> FileChange {
        FileChange::new(path, ChangeKind::Untracked)
    }

    fn reads() -> usize {
        READS.with(std::cell::Cell::get)
    }

    /// Counts `names` and says what each came to and how many files were read for it.
    fn refresh(root: &std::path::Path, names: &[&str]) -> (Vec<Option<u32>>, usize) {
        let before = reads();
        let mut files: Vec<FileChange> = names.iter().map(|n| untracked(n)).collect();
        count_untracked_lines(root, &mut files);
        (files.iter().map(|f| f.added).collect(), reads() - before)
    }

    fn age(path: &std::path::Path, seconds: u64) {
        let file = File::options().write(true).open(path).expect("open");
        file.set_modified(SystemTime::now() - Duration::from_secs(seconds)).expect("mtime");
    }

    #[test]
    fn count_lines_is_cached_until_mtime_or_size_changes() {
        let dir = init();
        let root = dir.path();
        write(root, "a.txt", "one\ntwo\n");
        write(root, "b.txt", "x\ny\nz");
        for name in ["a.txt", "b.txt"] {
            age(&root.join(name), 3600);
        }
        assert_eq!(refresh(root, &["a.txt", "b.txt"]), (vec![Some(2), Some(3)], 2), "first time: both read");
        assert_eq!(refresh(root, &["a.txt", "b.txt"]), (vec![Some(2), Some(3)], 0), "unchanged: no reads");

        // Same size, new mtime: read again. Different size: read again. The other stays cached.
        write(root, "a.txt", "ONE\nTWO\n");
        age(&root.join("a.txt"), 1800);
        assert_eq!(refresh(root, &["a.txt", "b.txt"]), (vec![Some(2), Some(3)], 1));
        write(root, "b.txt", "x\ny\nz\nw\n");
        age(&root.join("b.txt"), 3600);
        assert_eq!(refresh(root, &["a.txt", "b.txt"]), (vec![Some(2), Some(4)], 1));
        assert_eq!(refresh(root, &["a.txt", "b.txt"]).1, 0);

        // A file that left the list is forgotten, so it is read again if it comes back.
        assert_eq!(refresh(root, &["a.txt"]).1, 0);
        assert_eq!(refresh(root, &["a.txt", "b.txt"]).1, 1);
    }

    #[test]
    fn a_file_still_being_written_is_never_trusted_from_the_cache() {
        let dir = init();
        let root = dir.path();
        write(root, "log.txt", "a\n");
        assert_eq!(refresh(root, &["log.txt"]), (vec![Some(1)], 1));
        // Same size and, within the clock's tick, maybe the same mtime: only the settle rule
        // tells this apart from an unchanged file.
        write(root, "log.txt", "b\n");
        assert_eq!(refresh(root, &["log.txt"]), (vec![Some(1)], 1), "fresh files are read every time");
        write(root, "log.txt", "b\nc\n");
        assert_eq!(refresh(root, &["log.txt"]), (vec![Some(2)], 1));
    }

    #[test]
    fn nothing_that_is_not_a_small_regular_file_is_read() {
        let dir = init();
        let root = dir.path();
        write(root, "big.txt", &"line\n".repeat(1_000_000)); // 5 MB, over the 4 MiB limit
        write(root, "bin.dat", "a\0b\n");
        crate::platform::fs::symlink("elsewhere", root.join("link")).expect("symlink");
        crate::platform::fs::symlink(root.join("pipe"), root.join("link-to-nothing")).expect("symlink");
        let (counts, read) = refresh(root, &["big.txt", "bin.dat", "link", "link-to-nothing", "missing.txt"]);
        assert_eq!(counts, [None, None, Some(1), Some(1), None]);
        assert_eq!(read, 1, "only the small binary file was opened, to look for a NUL");
    }
}
