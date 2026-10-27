//! A picture of a repository's changed files, taken in the same background hop as the status: what
//! each looked like on disk, so two pictures say which files were touched in between.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::super::RepoStatus;
use crate::storage::files::FileStamp;

/// The most changed files a snapshot looks at. A status this long is a checkout, a build in the
/// tree or a vendored import, not someone's edits: saying which terminal made them would cost a
/// `stat` of each every poll for an answer nobody could read, so there is none.
pub const MAX_FILES: usize = 5_000;

/// What one changed file looked like: its [`FileStamp`], or nothing for a file that is not there
/// (a deletion is a change too, and a file that comes back is another one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fingerprint(Option<FileStamp>);

impl Fingerprint {
    /// One `lstat` of `path` (two for a link, whose target is looked at as well): never a read, so
    /// a file a repository planted (a FIFO, a link out of the tree) costs nothing but the call.
    pub(super) fn of(path: &Path) -> Self {
        Self(FileStamp::of(path))
    }
}

/// Every changed file of one repository at one moment, by the path `git status` names it with.
#[derive(Debug)]
pub struct Snapshot {
    pub(super) root: PathBuf,
    /// `None`: more than [`MAX_FILES`], so nothing is known of any.
    pub(super) prints: Option<Prints>,
}

impl Snapshot {
    /// The picture of `status`'s files, taken now. `None` for a status against another branch
    /// ("Compare with"): its files are the branch's difference, committed work included, not the
    /// working tree's changes, so no picture of the disk says whose they are.
    pub fn take(status: &RepoStatus) -> Option<Self> {
        if status.compare.is_some() {
            return None;
        }
        let prints = (status.files.len() <= MAX_FILES).then(|| {
            Prints(
                status
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), Fingerprint::of(&status.root.join(&file.path))))
                    .collect(),
            )
        });
        Some(Self { root: status.root.clone(), prints })
    }

    #[cfg(test)]
    pub(super) fn of_prints(
        root: impl Into<PathBuf>,
        prints: impl IntoIterator<Item = (String, Fingerprint)>,
    ) -> Self {
        Self { root: root.into(), prints: Some(Prints(prints.into_iter().collect())) }
    }
}

/// The files of a [`Snapshot`] that has them.
#[derive(Debug)]
pub(super) struct Prints(HashMap<String, Fingerprint>);

impl Prints {
    /// The files that are in `self` and were not in `before`, or look different now: what was
    /// touched in between.
    pub(super) fn touched_since<'a>(&'a self, before: &'a Prints) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(|(path, print)| before.0.get(*path) != Some(*print))
            .map(|(path, _)| path.as_str())
    }

    /// Whether `path` is changed in this picture.
    pub(super) fn has(&self, path: &str) -> bool {
        self.0.contains_key(path)
    }
}
