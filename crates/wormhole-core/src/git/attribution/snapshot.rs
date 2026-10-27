//! A picture of a repository's changed files, taken in the same background hop as the status: what
//! each looked like on disk, so two pictures say which files were touched in between.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::git::{FileChange, RepoStatus};
use crate::storage::files::FileStamp;

/// The most changed files a snapshot looks at. A status this long is a checkout, a build in the
/// tree or a vendored import, not someone's edits: saying which terminal made them would cost a
/// `stat` of each every poll for an answer nobody could read, so there is none.
pub(super) const MAX_FILES: usize = 5_000;

/// What one changed file looked like: its [`FileStamp`] (nothing for a file that is not there: a
/// deletion is a change too, and a file that comes back is another one), and for a submodule what the
/// parent says of it, since an edit inside a folder that is dirty already changes neither the
/// folder's own stamp nor the fact that it is dirty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fingerprint {
    stamp: Option<FileStamp>,
    /// A digest of the submodule's state, compared and never read back.
    submodule: Option<u64>,
}

impl Fingerprint {
    /// One `lstat` of the change's path: never a read, and never what a link points to (git tracks
    /// a link by its text; following it would depend on a file outside the project, and a FIFO or
    /// a socket a repository planted costs nothing but the call).
    fn of(root: &Path, file: &FileChange) -> Self {
        let submodule = file.submodule.as_ref().map(|sub| {
            let mut digest = DefaultHasher::new();
            // The commit checked out in the folder now (what the log is up to), and the parent's
            // view of it.
            sub.log.as_ref().map(|log| log.new.as_deref()).hash(&mut digest);
            (&sub.new, sub.commit_changed, sub.removed, sub.tracked, sub.untracked).hash(&mut digest);
            digest.finish()
        });
        Self { stamp: FileStamp::of_link(&root.join(&file.path)), submodule }
    }

    #[cfg(test)]
    pub(super) fn of_path(path: &Path) -> Self {
        Self { stamp: FileStamp::of_link(path), submodule: None }
    }

    #[cfg(test)]
    pub(super) fn with_submodule(self, digest: u64) -> Self {
        Self { submodule: Some(digest), ..self }
    }
}

/// Every changed file of one repository at one moment, by the path `git status` names it with.
#[derive(Debug)]
pub struct Snapshot {
    pub(super) root: PathBuf,
    /// When the disk was read. Snapshots are ordered by it, not by when a read was asked for or
    /// when its answer arrived.
    pub(super) taken: Instant,
    /// `None`: more than [`MAX_FILES`], so nothing is known of any.
    pub(super) prints: Option<Prints>,
}

impl Snapshot {
    /// The picture of `status`'s files, taken at `taken` (the caller's clock, read just before this,
    /// in the job that read the status). `None` for a status against another branch ("Compare with"):
    /// its files are the branch's difference, committed work included, not the working tree's
    /// changes, so no picture of the disk says whose they are. While the only terminal of a
    /// repository compares with a branch no pictures come, which reads as one long gap between two.
    pub fn take(status: &RepoStatus, taken: Instant) -> Option<Self> {
        if status.compare.is_some() {
            return None;
        }
        let prints = (status.files.len() <= MAX_FILES).then(|| {
            Prints(
                status
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), Fingerprint::of(&status.root, file)))
                    .collect(),
            )
        });
        Some(Self { root: status.root.clone(), taken, prints })
    }

    #[cfg(test)]
    pub(super) fn of_prints(
        root: impl Into<PathBuf>,
        taken: Instant,
        prints: impl IntoIterator<Item = (String, Fingerprint)>,
    ) -> Self {
        Self { root: root.into(), taken, prints: Some(Prints(prints.into_iter().collect())) }
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
