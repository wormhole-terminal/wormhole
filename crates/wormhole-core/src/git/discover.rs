//! Noticing that a folder became a repository, without asking git every few seconds.
//!
//! Once git has said "not a repository" for a folder, the panel stops polling it (each poll is
//! a git process for an answer that stays the same). What can change that answer is only
//! `.git` appearing in the folder or one of its parents (`git init`, `git clone` into it, a
//! worktree added) or going away. [`find_root`](super::find_root) looks for it with a `stat`
//! per ancestor and no process, so it is cheap enough to run on every tick; [`Absent`] decides
//! from its answers when git is worth asking again.

use std::path::{Path, PathBuf};

/// What a look at the disk means for the folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sighting {
    /// Nothing differs from what git last answered for.
    Unchanged,
    /// Something differs, seen once: look again shortly, to be sure it is not mid-write.
    LookAgain,
    /// The difference is confirmed: ask git.
    AskGit,
}

/// What the app knows about a folder git said is not in a repository: which `.git` (if any) the
/// last looks found, and a different one seen once and waiting to be seen again.
///
/// Asking only after the same sighting twice is the debounce: `git init` and `git clone` create
/// `.git` a piece at a time, and git asked in the middle of that fails ("not a git repository" or a
/// half-written config) and would flash an error over the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Absent {
    cwd: PathBuf,
    /// The repository root the looks found when git last answered (`None`: there was none).
    /// Git saying "no" while a `.git` is there (a broken one) is not asked about again until
    /// something else changes.
    settled: Option<PathBuf>,
    /// A change of `settled` seen once.
    candidate: Option<Option<PathBuf>>,
}

impl Absent {
    /// Git has just said `cwd` is not in a repository, and no `.git` was in sight.
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd, settled: None, candidate: None }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Whether this is about `cwd`.
    pub fn is_for(&self, cwd: &Path) -> bool {
        self.cwd == cwd
    }

    /// A look at the disk found `found` (the nearest ancestor of the folder with a `.git`, else
    /// `None`). Git is asked again ([`Sighting::AskGit`]) when the sighting differs from what git
    /// last answered for and the next look agrees, so the change is not mid-write.
    pub fn observe(&mut self, found: Option<PathBuf>) -> Sighting {
        if found == self.settled {
            self.candidate = None;
            return Sighting::Unchanged;
        }
        if self.candidate.as_ref() == Some(&found) {
            self.settled = found;
            self.candidate = None;
            return Sighting::AskGit;
        }
        self.candidate = Some(found);
        Sighting::LookAgain
    }
}

/// The nearest repository above `cwd` (`cwd` included), by its `.git` alone; see
/// [`find_root`](super::find_root). A pure `stat` walk: run it off the UI thread anyway, since a
/// network volume can make a `stat` wait.
pub fn probe(cwd: &Path) -> Option<PathBuf> {
    super::find_root(cwd)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Absent, Sighting, probe};

    fn absent() -> Absent {
        Absent::new(PathBuf::from("/work/new"))
    }

    #[test]
    fn nothing_new_on_disk_never_asks_git() {
        let mut absent = absent();
        for _ in 0..100 {
            assert_eq!(absent.observe(None), Sighting::Unchanged);
        }
    }

    #[test]
    fn a_repository_appearing_asks_once_the_sighting_is_confirmed() {
        let mut absent = absent();
        let root = Some(PathBuf::from("/work/new"));
        // First sight: `git init` may still be writing.
        assert_eq!(absent.observe(root.clone()), Sighting::LookAgain);
        assert_eq!(absent.observe(root.clone()), Sighting::AskGit);
        // Git is asked once; the same sighting does not ask again if git still says no.
        for _ in 0..10 {
            assert_eq!(absent.observe(root.clone()), Sighting::Unchanged);
        }
    }

    #[test]
    fn a_sighting_that_flickers_away_is_forgotten() {
        let mut absent = absent();
        let new = || Some(PathBuf::from("/work/new"));
        assert_eq!(absent.observe(new()), Sighting::LookAgain);
        assert_eq!(absent.observe(None), Sighting::Unchanged);
        assert_eq!(absent.observe(new()), Sighting::LookAgain);
        assert_eq!(absent.observe(new()), Sighting::AskGit);
    }

    #[test]
    fn a_different_repository_is_a_new_question() {
        let mut absent = absent();
        let a = Some(PathBuf::from("/work"));
        let b = Some(PathBuf::from("/work/new"));
        assert_eq!(absent.observe(a.clone()), Sighting::LookAgain);
        assert_eq!(absent.observe(a), Sighting::AskGit);
        // A nearer `.git` appeared: it takes two looks again.
        assert_eq!(absent.observe(b.clone()), Sighting::LookAgain);
        assert_eq!(absent.observe(b), Sighting::AskGit);
    }

    #[test]
    fn it_belongs_to_one_folder() {
        let absent = absent();
        assert!(absent.is_for(std::path::Path::new("/work/new")));
        assert!(!absent.is_for(std::path::Path::new("/work")));
    }

    #[test]
    fn the_probe_finds_a_git_init_without_a_process() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("a/b");
        std::fs::create_dir_all(&sub).expect("mkdir");
        let mut absent = Absent::new(sub.clone());
        assert_eq!(absent.observe(probe(&sub)), Sighting::Unchanged);
        // `git init` in the parent: seen from the subfolder.
        std::fs::create_dir(dir.path().join(".git")).expect("mkdir .git");
        let found = probe(&sub);
        assert_eq!(found.as_deref(), Some(dir.path()));
        assert_eq!(absent.observe(found.clone()), Sighting::LookAgain);
        assert_eq!(absent.observe(found), Sighting::AskGit);
    }
}
