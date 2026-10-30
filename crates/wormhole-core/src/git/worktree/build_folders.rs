//! The build folders (`node_modules`, `target`) of a worktree: cloned in from the main checkout when
//! the worktree is made, removed again with it. Both write to disk, which is why this is part of
//! `worktree`, the one place that is allowed to write in a repository's folders; which folders qualify
//! is decided by `git::ignored`, which only reads.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as _, Result};

use super::Worktree;
use crate::git::ignored;
use crate::platform::{backup, paths};
use crate::storage::files;

/// Clone `repo`'s build folders into the worktree at `folder`, each left out of backups (they are
/// rebuildable, and a clone is not free to back up). A folder is cloned only if `folder`'s own ignore
/// rules ignore it as well (the branch may not): otherwise it would show up there as untracked files.
/// A folder that can't be cloned is skipped with a warning; nothing here fails the worktree, which is
/// made already. Blocking: run it off the UI thread. Returns the folders cloned, relative to `folder`,
/// for [`Worktree::cloned`].
pub fn clone_build_folders(repo: &Path, folder: &Path) -> Vec<PathBuf> {
    let started = Instant::now();
    let found = match ignored::build_folders(repo) {
        Ok(found) => found,
        Err(err) => {
            tracing::warn!(repo = %repo.display(), "listing ignored folders: {err:#}");
            return Vec::new();
        }
    };
    let scratch = paths::temp();
    let mut cloned = Vec::new();
    for rel in found {
        match ignored::is_ignored(folder, &rel) {
            Ok(true) => {}
            Ok(false) => {
                tracing::debug!(folder = %rel.display(), "not cloned: the worktree does not ignore it");
                continue;
            }
            Err(err) => {
                tracing::warn!(folder = %rel.display(), "not cloned: {err:#}");
                continue;
            }
        }
        match files::clone_dir(repo, folder, &rel, &scratch) {
            Ok(()) => {
                if let Err(err) = backup::exclude(&folder.join(&rel)) {
                    tracing::warn!(folder = %rel.display(), "leaving a clone out of backups: {err}");
                }
                cloned.push(rel);
            }
            // The worktree has it already (a tracked file there), or its parent isn't there: nothing to do.
            Err(err)
                if matches!(err.kind(), std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotFound) =>
            {
                tracing::debug!(folder = %rel.display(), "not cloned: {err}");
            }
            // Not APFS, or another volume: the same for every folder, so say it once and stop.
            Err(err) if err.kind() == std::io::ErrorKind::Unsupported => {
                tracing::warn!("build folders are not cloned: {err}");
                break;
            }
            Err(err) => tracing::warn!(folder = %rel.display(), "cloning a build folder: {err}"),
        }
    }
    tracing::info!(count = cloned.len(), took = ?started.elapsed(), "build folders cloned into a worktree");
    cloned
}

/// Delete the clones `worktree` was given, so the worktree's removal doesn't have to: a big `target` or
/// `node_modules` takes long to delete, and `git worktree remove` is killed after a minute. Each is deleted only if it is still what was
/// made: a path inside the worktree (no `..`, no link on the way), a real folder (not a link), and
/// still ignored there. Anything else is left alone, with a warning. The first failure is returned and
/// the worktree stays as it is. Blocking, and as long as the tree is big: run it off the UI thread.
pub(super) fn remove_clones(worktree: &Worktree) -> Result<()> {
    let root = fs::canonicalize(&worktree.folder)
        .with_context(|| format!("resolving {}", worktree.folder.display()))?;
    for rel in &worktree.cloned {
        let plain =
            !rel.as_os_str().is_empty() && rel.components().all(|c| matches!(c, Component::Normal(_)));
        let path = root.join(rel);
        let inside = plain
            && path
                .parent()
                .and_then(|parent| fs::canonicalize(parent).ok())
                .is_some_and(|parent| parent.starts_with(&root));
        let is_folder = fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_dir());
        if !(inside && is_folder && ignored::is_ignored(&root, rel).unwrap_or(false)) {
            tracing::warn!(folder = %rel.display(), "a cloned build folder is left in place: it is not what was made");
            continue;
        }
        fs::remove_dir_all(&path).with_context(|| format!("removing the clone {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::clone_build_folders;
    use crate::git::testing::{git, repo_with, write};
    use crate::git::worktree::{Worktree, add, plan, remove};
    use crate::platform::backup;

    fn worktree_of(repo: &Path, base: &Path, branch: &str) -> Worktree {
        let made = plan(repo, branch, Some(base)).expect("plans");
        add(&made).expect("adds");
        made
    }

    fn fill(dir: &Path) {
        write(dir, "node_modules/dep/index.js", "module.exports = 1;\n");
        write(dir, "target/debug/app", "binary");
    }

    #[test]
    fn ignored_build_folders_are_cloned_and_left_out_of_backups() {
        let repo = repo_with(&[(".gitignore", "node_modules/\ntarget/\n"), ("a.txt", "a\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        fill(repo.path());
        let made = worktree_of(repo.path(), base.path(), "feature");
        let cloned = clone_build_folders(repo.path(), &made.folder);
        if cloned.is_empty() {
            // A temp dir that is not on APFS can't clone, and then nothing is copied at all.
            assert!(!made.folder.join("node_modules").exists() && !made.folder.join("target").exists());
            return;
        }
        let mut sorted = cloned.clone();
        sorted.sort();
        assert_eq!(sorted, [PathBuf::from("node_modules"), PathBuf::from("target")]);
        assert_eq!(fs::read_to_string(made.folder.join("target/debug/app")).expect("cloned"), "binary");
        assert!(backup::is_excluded(&made.folder.join("target")));
        assert!(!backup::is_excluded(&repo.path().join("target")), "the original stays in backups");
    }

    /// The branch decides what is ignored in its worktree: one made before `node_modules` was ignored
    /// would show a cloned `node_modules` as untracked files, so only `target` is cloned there.
    #[test]
    fn a_folder_the_worktree_does_not_ignore_is_not_cloned() {
        let repo = repo_with(&[(".gitignore", "target/\n")]);
        git(repo.path(), &["branch", "old"]);
        write(repo.path(), ".gitignore", "target/\nnode_modules/\n");
        git(repo.path(), &["commit", "-q", "-am", "ignore node_modules too"]);
        fill(repo.path());
        let base = tempfile::tempdir().expect("tempdir");
        let made = worktree_of(repo.path(), base.path(), "old");
        let cloned = clone_build_folders(repo.path(), &made.folder);
        assert!(!made.folder.join("node_modules").exists(), "{cloned:?}");
        assert!(cloned.is_empty() || cloned == [PathBuf::from("target")], "{cloned:?}");
    }

    #[test]
    fn removing_a_worktree_deletes_the_clones_it_recorded_and_nothing_else() {
        let repo = repo_with(&[(".gitignore", "node_modules/\ntarget/\n"), ("a.txt", "a\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        let mut made = worktree_of(repo.path(), base.path(), "feature");
        // As if cloned: ignored folders in the worktree, one of which was never recorded.
        fill(&made.folder);
        made.cloned = vec!["node_modules".into(), "a.txt".into(), "../escape".into(), "missing".into()];
        let outside = base.path().join("escape");
        fs::create_dir_all(&outside).expect("mkdir");
        remove(&made).expect("removes");
        assert!(!made.folder.exists());
        assert!(outside.exists(), "a recorded path that leaves the worktree is never deleted");
    }

    #[test]
    fn a_clone_that_cannot_be_deleted_leaves_the_worktree_in_place() {
        let repo = repo_with(&[(".gitignore", "node_modules/\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        let mut made = worktree_of(repo.path(), base.path(), "feature");
        write(&made.folder, "node_modules/locked/file", "x");
        made.cloned = vec!["node_modules".into()];
        let locked = made.folder.join("node_modules/locked");
        fs::set_permissions(&locked, crate::platform::fs::permissions(0o500)).expect("chmod");
        let outcome = remove(&made);
        fs::set_permissions(&locked, crate::platform::fs::permissions(0o700)).expect("chmod back");
        assert!(outcome.is_err(), "deleting a file in a read-only folder fails: {outcome:?}");
        assert!(made.folder.exists(), "the worktree stays");
        remove(&made).unwrap_or_else(|e| panic!("removes once it can: {e:#}"));
    }

    #[test]
    fn a_recorded_folder_that_is_a_link_or_no_longer_ignored_is_kept() {
        // No trailing slash, so the rule ignores a link as well as a folder.
        let repo = repo_with(&[(".gitignore", "target\n"), ("a.txt", "a\n")]);
        let base = tempfile::tempdir().expect("tempdir");
        let mut made = worktree_of(repo.path(), base.path(), "feature");
        // A link in place of the clone, to a folder that must survive.
        let keep = base.path().join("keep");
        write(&keep, "file", "x");
        crate::platform::fs::symlink(&keep, made.folder.join("target")).expect("link");
        made.cloned = vec!["target".into()];
        // The link is ignored, so git's dirty check passes; the link is not followed.
        remove(&made).expect("removes");
        assert!(keep.join("file").exists());
    }
}
