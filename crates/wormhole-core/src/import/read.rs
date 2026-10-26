//! Reading the files of other terminals: capped, one at a time, with a limit on how many and how
//! deep includes go, and a guard against a file that includes itself. Every read of the domain goes
//! through a [`Reader`], so what a hostile dotfiles repository can cost is bounded in one place.
//! Nothing is written, and nothing in a file is run.

use std::io;
use std::path::{Path, PathBuf};

use super::proposal::{Reason, Skipped};
use crate::storage::files::{self, LinkPolicy};

/// Largest settings file read; real ones are a few KiB.
pub(crate) const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// Largest property list read: iTerm2 keeps every profile and its key map in one.
pub(crate) const MAX_PLIST_BYTES: u64 = 16 * 1024 * 1024;
/// Most files one source reads, its includes and themes counted.
pub(crate) const MAX_FILES: usize = 32;

/// Reads one source's files and remembers which, and what went wrong.
pub(crate) struct Reader {
    /// The files that were there, in the order they were read.
    files: Vec<PathBuf>,
    skipped: Vec<Skipped>,
    /// The files being read now, outermost first, as the system names them (links resolved).
    active: Vec<PathBuf>,
    max_depth: usize,
}

impl Reader {
    /// A reader that follows includes at most `max_depth` files deep.
    pub(crate) fn new(max_depth: usize) -> Self {
        Self { files: Vec::new(), skipped: Vec::new(), active: Vec::new(), max_depth }
    }

    /// A file's text. `None` when it is not there (not a problem: most includes are optional) or
    /// cannot be read (which is noted).
    pub(crate) fn text(&mut self, path: &Path) -> Option<String> {
        let bytes = self.bytes(path, MAX_FILE_BYTES)?;
        match String::from_utf8(bytes) {
            Ok(text) => Some(text),
            Err(_) => {
                self.note(path, Reason::Unreadable);
                None
            }
        }
    }

    /// A file's bytes, at most `cap` of them.
    pub(crate) fn bytes(&mut self, path: &Path, cap: u64) -> Option<Vec<u8>> {
        if self.files.len() >= MAX_FILES {
            self.note(path, Reason::IncludeLimit);
            return None;
        }
        match files::read_capped(path, cap, LinkPolicy::Follow) {
            Ok(bytes) => {
                self.files.push(path.to_path_buf());
                Some(bytes)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => {
                tracing::debug!("import: {}: {err}", path.display());
                self.files.push(path.to_path_buf());
                self.note(path, Reason::Unreadable);
                None
            }
        }
    }

    /// A file that may include others: read it, and run `read` over its text with this reader, which
    /// knows the file is being read, so an include of it, or too deep a chain, ends there.
    pub(crate) fn nested<T>(&mut self, path: &Path, read: impl FnOnce(&mut Self, String) -> T) -> Option<T> {
        let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.active.contains(&real) {
            self.note(path, Reason::IncludeCycle);
            return None;
        }
        if self.active.len() >= self.max_depth {
            self.note(path, Reason::IncludeLimit);
            return None;
        }
        let text = self.text(path)?;
        self.active.push(real);
        let out = read(self, text);
        self.active.pop();
        Some(out)
    }

    /// Whether another file may still be read: a scan of many files stops quietly at the limit.
    pub(crate) fn has_room(&self) -> bool {
        self.files.len() < MAX_FILES
    }

    /// Note that a file is there without reading it (one that is not read in this format).
    pub(crate) fn seen(&mut self, path: &Path) {
        self.files.push(path.to_path_buf());
    }

    /// Note a problem with a file.
    pub(crate) fn note(&mut self, path: &Path, reason: Reason) {
        self.skipped.push(Skipped { key: path.to_string_lossy().into_owned(), reason });
    }

    /// The files that were read and the problems met.
    pub(crate) fn finish(self) -> (Vec<PathBuf>, Vec<Skipped>) {
        (self.files, self.skipped)
    }
}
