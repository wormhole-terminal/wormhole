//! What a folder band says about its working tree beyond the branch: which repository a
//! worktree belongs to, and the commit a detached HEAD is at.

use std::path::{Component, Path, PathBuf};

use anyhow::Result;

use super::{GitCmd, GitError};
use crate::fsutil::{LinkPolicy, read_capped};

/// A worktree's `.git` file is one line.
pub(super) const POINTER_CAP: u64 = 4096;

/// The main repository of the worktree whose top is `root`, or `None` for an ordinary
/// repository (whose `.git` is a folder) or a submodule. Read from the `.git` file alone, without a
/// git process (the sidebar asks for every band) and without following the path it names: a
/// worktree's git dir is always `<common git dir>/worktrees/<name>`, and anything else isn't one.
pub fn worktree_of(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if !dot_git.is_file() {
        return None;
    }
    let pointer = read_capped(&dot_git, POINTER_CAP, LinkPolicy::Within(root)).ok()?;
    let gitdir = normalize(&root.join(pointed_dir(&String::from_utf8_lossy(&pointer))?));
    let worktrees = gitdir.parent()?;
    (worktrees.file_name()? == "worktrees").then(|| main_tree(worktrees.parent().unwrap_or(worktrees)))
}

/// The path a `.git` file points to: `gitdir: /repo/.git/worktrees/name`.
pub(super) fn pointed_dir(text: &str) -> Option<&str> {
    text.lines().find_map(|line| line.strip_prefix("gitdir:")).map(str::trim).filter(|dir| !dir.is_empty())
}

/// The working tree of a repository's common git dir: its parent when it is a `.git` folder,
/// the dir itself for a bare repository.
pub(super) fn main_tree(common: &Path) -> PathBuf {
    match common.file_name() {
        Some(name) if name == ".git" => common.parent().unwrap_or(common).to_path_buf(),
        _ => common.to_path_buf(),
    }
}

/// `a/b/../c` as `a/c`, without touching the disk (the repository may be elsewhere).
pub(super) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// The commit HEAD is at, as a detached HEAD's tooltip names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadCommit {
    pub short: String,
    pub subject: String,
}

/// The commit HEAD is at in the repository at `root`; `None` before the first commit.
pub fn head_commit(root: &Path) -> Result<Option<HeadCommit>> {
    match GitCmd::new(root).args(["log", "-1", "--format=%h%x00%s"]).revs(["HEAD"]).paths::<_, &str>([]).run()
    {
        Ok(out) => Ok(parse_head(&String::from_utf8_lossy(&out))),
        // No commit yet: there is no HEAD to name.
        Err(err) if matches!(GitError::of(&err), GitError::Failed(_)) => Ok(None),
        Err(err) => Err(err),
    }
}

fn parse_head(text: &str) -> Option<HeadCommit> {
    let (short, subject) = text.trim_end().split_once('\0')?;
    (!short.is_empty()).then(|| HeadCommit { short: short.to_string(), subject: subject.to_string() })
}

#[cfg(test)]
mod tests {
    use super::{HeadCommit, main_tree, normalize, parse_head, pointed_dir, worktree_of};
    use std::path::{Path, PathBuf};

    #[test]
    fn a_git_file_points_at_its_git_dir() {
        assert_eq!(pointed_dir("gitdir: /w/app/.git/worktrees/badge\n"), Some("/w/app/.git/worktrees/badge"));
        assert_eq!(pointed_dir("gitdir: ../.git/modules/sub"), Some("../.git/modules/sub"));
        assert_eq!(pointed_dir("nonsense"), None);
        assert_eq!(pointed_dir("gitdir:   "), None);
    }

    #[test]
    fn only_a_git_dir_under_worktrees_is_a_worktree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tree = dir.path().join("badge");
        std::fs::create_dir_all(&tree).expect("mkdir");
        let pointing = |to: &str| {
            std::fs::write(tree.join(".git"), format!("gitdir: {to}\n")).expect("writes .git");
            worktree_of(&tree)
        };
        assert_eq!(pointing("/w/app/.git/worktrees/badge"), Some(PathBuf::from("/w/app")));
        assert_eq!(pointing("../app/.git/worktrees/badge"), Some(dir.path().join("app")), "relative");
        assert_eq!(pointing("/w/app/.git/modules/sub"), None, "a submodule");
        assert_eq!(pointing("/etc"), None, "anything else is read no further");
    }

    #[test]
    fn the_common_dir_leads_to_the_main_working_tree() {
        let common = normalize(Path::new("/w/app/.git/worktrees/badge/../.."));
        assert_eq!(common, PathBuf::from("/w/app/.git"));
        assert_eq!(main_tree(&common), PathBuf::from("/w/app"));
        assert_eq!(main_tree(Path::new("/srv/app.git")), PathBuf::from("/srv/app.git"), "a bare repository");
    }

    #[test]
    fn heads_commit_is_its_short_hash_and_subject() {
        let head = parse_head("4104e74\0fix(terminal): erase scrollback when running clear\n");
        assert_eq!(
            head,
            Some(HeadCommit {
                short: "4104e74".into(),
                subject: "fix(terminal): erase scrollback when running clear".into()
            })
        );
        assert_eq!(parse_head(""), None);
    }
}
