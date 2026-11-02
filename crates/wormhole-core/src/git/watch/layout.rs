//! Where a repository's files are: the work tree and the git dirs. For an ordinary repository that is
//! `<root>` and `<root>/.git`; a linked worktree has its own git dir (`<main>/.git/worktrees/<name>`:
//! its index and HEAD) beside the common one (`<main>/.git`: refs, config), both outside its tree, and
//! a submodule's git dir is `<super>/.git/modules/<name>`.

use std::path::{Path, PathBuf};

use super::super::branch::git_dir_of;
use super::super::head::normalize;

/// The folders a repository's state lives in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub(super) work_tree: PathBuf,
    /// This work tree's own git dir: `index`, `HEAD`, the merge and rebase state.
    pub(super) git_dir: PathBuf,
    /// Shared by all of a repository's work trees: `refs`, `packed-refs`, `config`, `info/exclude`.
    /// The same as `git_dir` except for a linked worktree.
    pub(super) common_dir: PathBuf,
}

impl Layout {
    /// From the three folders, which must be in the form the system reports paths in (see
    /// [`resolve`](Self::resolve)).
    pub fn new(work_tree: PathBuf, git_dir: PathBuf, common_dir: PathBuf) -> Self {
        Self { work_tree, git_dir, common_dir }
    }

    /// The layout of the repository whose top is `root`, by reading its `.git` (a folder, or the file
    /// a worktree or submodule has). `None` when `root` has none git would accept. Paths are made real
    /// (links resolved), because that is how the file events name them.
    pub fn resolve(root: &Path) -> Option<Self> {
        let git_dir = git_dir_of(root)?;
        // A linked worktree's git dir is `<common>/worktrees/<name>` (what `git_dir_of` checked).
        let common_dir = match git_dir.parent() {
            Some(parent) if parent.file_name().is_some_and(|name| name == "worktrees") => {
                parent.parent().map(normalize)
            }
            _ => None,
        }
        .unwrap_or_else(|| git_dir.clone());
        let real = |path: PathBuf| std::fs::canonicalize(&path).unwrap_or(path);
        Some(Self::new(real(root.to_path_buf()), real(git_dir), real(common_dir)))
    }

    /// The folders to watch: the work tree, and each git dir that is not inside it.
    pub fn roots(&self) -> Vec<&Path> {
        let mut roots = vec![self.work_tree.as_path()];
        // The common dir first: it holds a worktree's own, and then one watch covers both.
        for dir in [&self.common_dir, &self.git_dir] {
            if !dir.starts_with(&self.work_tree) && !roots.iter().any(|root| dir.starts_with(root)) {
                roots.push(dir);
            }
        }
        roots
    }

    pub fn work_tree(&self) -> &Path {
        &self.work_tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_repository_is_watched_as_one_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("mkdir");
        let layout = Layout::resolve(dir.path()).expect("a layout");
        assert_eq!(layout.roots().len(), 1);
        assert_eq!(layout.git_dir, layout.common_dir);
        assert!(layout.git_dir.starts_with(&layout.work_tree));
    }

    #[test]
    fn a_linked_worktree_also_watches_both_git_dirs_outside_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let main_git = dir.path().join("app/.git");
        let own = main_git.join("worktrees/badge");
        std::fs::create_dir_all(&own).expect("mkdir");
        let tree = dir.path().join("badge");
        std::fs::create_dir(&tree).expect("mkdir");
        std::fs::write(tree.join(".git"), format!("gitdir: {}\n", own.display())).expect("writes");
        let layout = Layout::resolve(&tree).expect("a layout");
        assert_eq!(layout.common_dir, std::fs::canonicalize(&main_git).expect("real"));
        assert_eq!(layout.git_dir, std::fs::canonicalize(&own).expect("real"));
        // The common dir contains the worktree's own, so one watch covers both.
        assert_eq!(layout.roots(), vec![layout.work_tree.as_path(), layout.common_dir.as_path()]);
    }

    #[test]
    fn a_folder_without_a_git_has_no_layout() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(Layout::resolve(dir.path()), None);
    }
}
