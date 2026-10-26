//! Reading the files of other terminals: capped, one at a time, with a limit on how many and how
//! deep includes go, and a guard against a file that includes itself. Every read of the domain goes
//! through a [`Reader`], so what a hostile dotfiles repository can cost is bounded in one place.
//! Nothing is written, and nothing in a file is run.
//!
//! A source's own settings files are read wherever the system puts them. A file the settings *name*
//! (an include, an import, a theme, a preferences folder) is read only inside the person's home
//! (`*_in_home`): with every link resolved it must still be there, so a dotfiles repository cannot
//! point the importer at `/etc` or at a network volume. The check is by path first, which touches
//! nothing, and then by `storage::files`'s `LinkPolicy::Within`.

use std::io;
use std::path::{Component, Path, PathBuf};

use super::proposal::{Reason, Skipped};
use crate::storage::files::{self, LinkPolicy, Refusal};

/// Largest settings file read; real ones are a few KiB.
pub(crate) const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// Largest property list read: iTerm2 keeps every profile and its key map in one.
pub(crate) const MAX_PLIST_BYTES: u64 = 16 * 1024 * 1024;
/// Most files one source reads, its includes and themes counted.
pub(crate) const MAX_FILES: usize = 32;

/// Where a file may be read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Where the source keeps it: the person's own settings file, or an app's own folder.
    Anywhere,
    /// A file the settings name: inside the home folder only.
    Home,
}

/// Reads one source's files and remembers which, and what went wrong.
pub(crate) struct Reader {
    /// The person's home: where a file the settings name must be.
    home: PathBuf,
    /// The files that were there, in the order they were read.
    files: Vec<PathBuf>,
    skipped: Vec<Skipped>,
    /// The files being read now, outermost first, as the system names them (links resolved).
    active: Vec<PathBuf>,
    max_depth: usize,
}

impl Reader {
    /// A reader for files under `home` that follows includes at most `max_depth` files deep.
    pub(crate) fn new(home: &Path, max_depth: usize) -> Self {
        Self {
            home: home.to_path_buf(),
            files: Vec::new(),
            skipped: Vec::new(),
            active: Vec::new(),
            max_depth,
        }
    }

    /// Whether `path` is below the home folder by its name, and climbs nowhere: `~/../../Volumes/x`
    /// begins with the home and leaves it. A link inside the home can still lead out; the read itself
    /// checks that.
    pub(crate) fn names_home(&self, path: &Path) -> bool {
        path.starts_with(&self.home) && !path.components().any(|part| part == Component::ParentDir)
    }

    /// A source's own file's text. `None` when it is not there (not a problem: most includes are
    /// optional) or cannot be read (which is noted).
    pub(crate) fn text(&mut self, path: &Path) -> Option<String> {
        self.text_in(path, Scope::Anywhere)
    }

    /// The text of a file the settings name: inside the home folder only.
    pub(crate) fn text_in_home(&mut self, path: &Path) -> Option<String> {
        self.text_in(path, Scope::Home)
    }

    fn text_in(&mut self, path: &Path, scope: Scope) -> Option<String> {
        let bytes = self.bytes_in(path, MAX_FILE_BYTES, scope)?;
        match String::from_utf8(bytes) {
            Ok(text) => Some(text),
            Err(_) => {
                self.note(path, Reason::Unreadable);
                None
            }
        }
    }

    /// A source's own file's bytes, at most `cap` of them.
    pub(crate) fn bytes(&mut self, path: &Path, cap: u64) -> Option<Vec<u8>> {
        self.bytes_in(path, cap, Scope::Anywhere)
    }

    /// [`bytes`](Self::bytes) of a file the settings name.
    pub(crate) fn bytes_in_home(&mut self, path: &Path, cap: u64) -> Option<Vec<u8>> {
        self.bytes_in(path, cap, Scope::Home)
    }

    pub(crate) fn bytes_in(&mut self, path: &Path, cap: u64, scope: Scope) -> Option<Vec<u8>> {
        if self.files.len() >= MAX_FILES {
            self.note(path, Reason::IncludeLimit);
            return None;
        }
        if scope == Scope::Home && !self.names_home(path) {
            // By name alone: nothing outside the home is touched, not even to see if it is there.
            self.note(path, Reason::OutsideHome);
            return None;
        }
        let policy = match scope {
            Scope::Anywhere => LinkPolicy::Follow,
            Scope::Home => LinkPolicy::Within(&self.home),
        };
        match files::read_capped(path, cap, policy) {
            Ok(bytes) => {
                self.files.push(path.to_path_buf());
                Some(bytes)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) if matches!(files::refusal(&err), Some(Refusal::OutsideRoot(_))) => {
                self.note(path, Reason::OutsideHome);
                None
            }
            Err(err) => {
                tracing::debug!("import: {}: {err}", path.display());
                self.files.push(path.to_path_buf());
                self.note(path, Reason::Unreadable);
                None
            }
        }
    }

    /// A file that may include others: read it, and run `read` over its text with this reader, which
    /// knows the file is being read, so an include of it, or too deep a chain, ends there. A source's
    /// own file is read in [`Scope::Anywhere`], one the settings name (an include, an import) in
    /// [`Scope::Home`].
    pub(crate) fn nested_in<T>(
        &mut self,
        path: &Path,
        scope: Scope,
        read: impl FnOnce(&mut Self, String) -> T,
    ) -> Option<T> {
        if scope == Scope::Home && !self.names_home(path) {
            self.note(path, Reason::OutsideHome);
            return None;
        }
        let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.active.contains(&real) {
            self.note(path, Reason::IncludeCycle);
            return None;
        }
        if self.active.len() >= self.max_depth {
            self.note(path, Reason::IncludeLimit);
            return None;
        }
        let text = self.text_in(path, scope)?;
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
