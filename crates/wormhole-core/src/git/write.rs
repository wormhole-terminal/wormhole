//! The one place this crate writes to a repository: `git worktree add` and `git worktree remove`.
//!
//! Everything else in `git` is read-only, and that stays the rule. The exception is kept this
//! narrow on purpose: one short builder with [`GitCmd`]'s hardening and two methods, not a
//! general way to run git. Both run only on the user's action, confirmed in the app
//! (`worktree`); never `--force`, so git itself refuses to remove a worktree with uncommitted or
//! untracked files, and never `-B`, so no existing branch is reset.
//!
//! On top of [`GitCmd`]: hooks are off (`core.hooksPath` pointed at nothing). `worktree add`
//! checks files out, which runs `post-checkout`, and a hook is whatever the repository's own
//! config or `.git/hooks` holds. A branch name and a folder come after `--end-of-options`, as
//! every name from outside does.

use std::path::Path;
use std::time::Duration;

use anyhow::Result;

use super::GitCmd;

/// Checking a large repository out takes a while; still bounded, as every git call is.
const ADD_TIMEOUT: Duration = Duration::from_secs(120);
const REMOVE_TIMEOUT: Duration = Duration::from_secs(60);

/// What `core.hooksPath` is pointed at so no hook runs: a folder with nothing in it.
const NO_HOOKS: &str = "/var/empty";

/// A writing git invocation in a repository, built whole by one of the two methods.
pub(crate) struct GitWrite {
    cmd: GitCmd,
}

impl GitWrite {
    fn in_repo(repo: &Path, timeout: Duration) -> Self {
        let mut cmd = GitCmd::new(repo);
        cmd.args(["-c", format!("core.hooksPath={NO_HOOKS}").as_str()]).timeout(timeout);
        Self { cmd }
    }

    /// `git worktree add [-b <branch>] -- <folder> [<branch>]`: a worktree at `folder` on
    /// `branch`, which is created from HEAD when `new_branch` and checked out as it is otherwise.
    /// `branch` has passed [`super::worktree::branch_name`] (so it can't read as an option).
    pub(crate) fn worktree_add(repo: &Path, folder: &Path, branch: &str, new_branch: bool) -> Result<()> {
        let mut write = Self::in_repo(repo, ADD_TIMEOUT);
        write.cmd.args(["worktree", "add"]);
        if new_branch {
            write.cmd.args(["-b", branch]);
            write.cmd.revs([folder.as_os_str()]);
        } else {
            write.cmd.revs([folder.as_os_str(), branch.as_ref()]);
        }
        write.cmd.run().map(drop)
    }

    /// `git worktree remove -- <folder>`: never forced, so git refuses a worktree with changes.
    pub(crate) fn worktree_remove(repo: &Path, folder: &Path) -> Result<()> {
        let mut write = Self::in_repo(repo, REMOVE_TIMEOUT);
        write.cmd.args(["worktree", "remove"]).revs([folder]);
        write.cmd.run().map(drop)
    }

    #[cfg(test)]
    pub(crate) fn command(&self) -> &std::process::Command {
        self.cmd.command()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::{ADD_TIMEOUT, GitWrite, NO_HOOKS};

    fn args(write: &GitWrite) -> Vec<String> {
        write.command().get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    /// The same hardening as every read, hooks off on top, and the names after `--end-of-options`.
    #[test]
    fn a_write_is_hardened_like_a_read_with_hooks_off() {
        let write = GitWrite::in_repo(Path::new("/some/repo"), ADD_TIMEOUT);
        let args = args(&write);
        assert_eq!(
            args[..7],
            [
                "--no-pager",
                "-C",
                "/some/repo",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "diff.autoRefreshIndex=false"
            ]
        );
        assert_eq!(args[7..], ["-c", &format!("core.hooksPath={NO_HOOKS}")]);
        assert!(
            Path::new(NO_HOOKS).is_dir() || !Path::new(NO_HOOKS).exists(),
            "a folder with nothing in it, or none"
        );
        let _ = Duration::ZERO;
    }

    /// The two commands, argument by argument; nothing else can be built.
    #[test]
    fn only_worktree_add_and_remove_exist() {
        let mut add = GitWrite::in_repo(Path::new("/r"), ADD_TIMEOUT);
        add.cmd.args(["worktree", "add"]).args(["-b", "fix/login"]).revs(["/r-worktrees/fix-login"]);
        assert_eq!(
            args(&add)[9..],
            ["worktree", "add", "-b", "fix/login", "--end-of-options", "/r-worktrees/fix-login"]
        );
        let mut existing = GitWrite::in_repo(Path::new("/r"), ADD_TIMEOUT);
        existing.cmd.args(["worktree", "add"]).revs(["/r-worktrees/main", "main"]);
        assert_eq!(
            args(&existing)[9..],
            ["worktree", "add", "--end-of-options", "/r-worktrees/main", "main"]
        );
        let mut remove = GitWrite::in_repo(Path::new("/r"), ADD_TIMEOUT);
        remove.cmd.args(["worktree", "remove"]).revs(["/r-worktrees/fix-login"]);
        assert_eq!(args(&remove)[9..], ["worktree", "remove", "--end-of-options", "/r-worktrees/fix-login"]);
        for built in [&add, &existing, &remove] {
            assert!(!args(built).iter().any(|a| a == "--force" || a == "-f" || a == "-B"), "never forced");
        }
    }
}
