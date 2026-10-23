//! The window's reading of each terminal's repository: what git says of the folder the terminal is
//! in. The sidebar's rows, the title row, the status bar and the palette show the answer, so it is
//! the window's own fact about a terminal, not a feature's; the Changes tab and the others read it
//! through the facts they are lent, and ask for a read through [`FeatureEvent::Repo`]
//! (`super::feature`).
//!
//! `state` is the answer kept per terminal ([`GitState`]), `poll` the reads: on the tick, when a tab
//! is shown or asked to Retry, and the cheap look for a `.git` in a folder git said is no repository.
//!
//! What a read can also report is what it saw of one file ([`Observed`]): taken in the same
//! background hop as the status, so a feature that watches a file (an open diff) does not stat it on
//! the UI thread.

pub(in crate::window) mod job;
mod poll;
mod state;
#[cfg(test)]
mod tests;

pub(in crate::window) use state::GitState;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A working-tree file's modification time and size. When they differ from what a diff was read
/// from, the file was edited since.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::window) struct Stamp {
    pub(in crate::window) modified: Option<SystemTime>,
    pub(in crate::window) len: u64,
}

impl Stamp {
    /// The stamp of the file at `path`, if it is there.
    pub(in crate::window) fn of(path: &Path) -> Option<Self> {
        Some(Self::from(&std::fs::metadata(path).ok()?))
    }
}

impl From<&std::fs::Metadata> for Stamp {
    fn from(meta: &std::fs::Metadata) -> Self {
        Self { modified: meta.modified().ok(), len: meta.len() }
    }
}

/// What a status read saw of the file a feature asked it to look at, in the same hop.
pub(in crate::window) struct Observed {
    pub path: PathBuf,
    pub stamp: Option<Stamp>,
}

impl Observed {
    pub(in crate::window) fn of(path: PathBuf) -> Self {
        let stamp = Stamp::of(&path);
        Self { path, stamp }
    }
}
