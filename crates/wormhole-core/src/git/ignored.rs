//! Which ignored folders of a checkout a new worktree may start with: `node_modules` and `target`, and
//! only those two names, on purpose. Their contents are mostly independent of where the checkout is,
//! and the tools that own them rebuild what isn't: cargo rebuilds the workspace's own crates, whose
//! dep-info records absolute paths, and npm and pnpm shims and `node_modules/.cache` may hold absolute
//! paths too. A Python `.venv` is different: it hard-codes its own path in `bin/activate` and in the
//! shebang of every script, so a clone would run against the original's interpreter and is left out.
//! An unknown ignored folder may be anything (secrets, a database, a huge cache); a fixed list is the
//! safe side.
//!
//! This file only reads, through [`GitCmd`]: the decision (which of git's ignored paths qualify) is
//! pure ([`candidates`]). The cloning and the removal, which write, are in `worktree::build_folders`.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::GitCmd;

/// The folder names that are cloned, and nothing else.
pub(crate) const BUILD_FOLDERS: [&str; 2] = ["node_modules", "target"];

/// At most this many folders per worktree: a monorepo has one `node_modules` per package, but a
/// repository with thousands of them would make a worktree take long to start, and a hostile
/// repository must not be able to make Wormhole clone without end.
pub(crate) const MAX_FOLDERS: usize = 256;

/// The most of git's listing that is read (about 16,000 folders): a repository with more ignored
/// folders than that has none worth cloning, and a hostile one must not make this read without end.
const MAX_LISTING: u64 = 1 << 20;

/// The folders to clone, from the NUL-separated output of
/// `git ls-files --others --ignored --exclude-standard --directory -z`: ignored directories end in `/`.
/// Absolute paths, paths with `..` and anything not named in [`BUILD_FOLDERS`] are dropped, and so is
/// everything past [`MAX_FOLDERS`].
pub(crate) fn candidates(raw: &[u8]) -> Vec<PathBuf> {
    raw.split(|&b| b == 0)
        .filter_map(|entry| {
            let entry = std::str::from_utf8(entry).ok()?.strip_suffix('/')?;
            let path = Path::new(entry);
            let safe =
                !path.has_root() && path.components().all(|c| matches!(c, std::path::Component::Normal(_)));
            let named = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| BUILD_FOLDERS.contains(&n));
            (safe && named).then(|| path.to_path_buf())
        })
        .take(MAX_FOLDERS)
        .collect()
}

/// Ask git (read-only) which of `checkout`'s ignored folders are build folders. `--directory` lists an
/// ignored folder as one entry without descending into it, so a big `target` costs one line.
pub(crate) fn build_folders(checkout: &Path) -> Result<Vec<PathBuf>> {
    let raw = GitCmd::new(checkout)
        .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
        .max_output(MAX_LISTING)
        .run()?;
    Ok(candidates(&raw))
}

/// Whether `rel` (a folder, relative to `checkout`) is ignored by `checkout`'s own rules. The folder
/// need not exist yet: the trailing slash makes git take the path as a directory. A path that is
/// tracked there is not ignored (git says so too).
pub(crate) fn is_ignored(checkout: &Path, rel: &Path) -> Result<bool> {
    // `./` first: check-ignore rejects literal pathspecs, so the path is read as a pathspec, and a
    // folder named like magic (`:(top)x`) must not be.
    let mut dir = std::ffi::OsString::from("./");
    dir.push(rel);
    dir.push("/");
    let output =
        GitCmd::new(checkout).args(["check-ignore", "-q"]).pathspecs_not_literal().paths([dir]).output()?;
    // 0: ignored, 1: not ignored, anything else (128) is an error.
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => anyhow::bail!(
            "git check-ignore failed in {}: {}",
            checkout.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{MAX_FOLDERS, build_folders, candidates, is_ignored};
    use crate::git::testing::{git, repo_with, write};

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn only_ignored_build_folders_qualify() {
        let raw = b"node_modules/\0web/node_modules/\0target/\0.venv/\0dist/\0notes.txt\0node_modules\0src/target/debug/\0my_target/\0";
        assert_eq!(candidates(raw), paths(&["node_modules", "web/node_modules", "target"]));
    }

    #[test]
    fn paths_that_leave_the_checkout_are_dropped() {
        let raw = b"/abs/node_modules/\0../node_modules/\0a/../target/\0./target/\0ok/target/\0";
        assert_eq!(candidates(raw), paths(&["ok/target"]));
    }

    #[test]
    fn the_number_is_capped_and_empty_input_is_none() {
        assert!(candidates(b"").is_empty());
        let raw: Vec<u8> =
            (0..MAX_FOLDERS + 10).flat_map(|i| format!("p{i}/node_modules/\0").into_bytes()).collect();
        assert_eq!(candidates(&raw).len(), MAX_FOLDERS);
    }

    /// What git itself lists, not a hand-written listing: an ignored `target/` is one entry (git does not
    /// descend into it), a nested `node_modules/` is found, and other ignored or untracked paths are not
    /// build folders.
    #[test]
    fn git_lists_ignored_folders_without_descending_into_them() {
        let repo = repo_with(&[
            (".gitignore", "target/\nnode_modules/\n.venv/\n*.log\n"),
            ("src/main.rs", "fn main() {}\n"),
        ]);
        write(repo.path(), "target/debug/deep/out.o", "x");
        write(repo.path(), "pkg/node_modules/dep/index.js", "x");
        write(repo.path(), ".venv/bin/python", "x");
        write(repo.path(), "build.log", "x");
        write(repo.path(), "untracked/node_modules/x", "x"); // ignored by the `node_modules/` rule too
        write(repo.path(), "notes.txt", "x");
        let listing = crate::git::GitCmd::new(repo.path())
            .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
            .run()
            .expect("lists");
        let listed: Vec<&str> = listing
            .split(|&b| b == 0)
            .filter(|e| !e.is_empty())
            .map(|e| std::str::from_utf8(e).expect("utf8"))
            .collect();
        assert!(listed.contains(&"target/"), "{listed:?}");
        assert!(
            !listed.iter().any(|e| e.starts_with("target/") && *e != "target/"),
            "not descended: {listed:?}"
        );
        assert!(listed.contains(&"pkg/node_modules/"), "{listed:?}");
        let mut found = build_folders(repo.path()).expect("lists");
        found.sort();
        assert_eq!(
            found,
            [PathBuf::from("pkg/node_modules"), "target".into(), "untracked/node_modules".into()]
        );
    }

    /// The new worktree's own rules decide: a branch whose `.gitignore` lacks `node_modules` has it as an
    /// untracked folder there, which must not be cloned in; a folder with a tracked file is not ignored.
    #[test]
    fn a_folder_is_ignored_by_the_checkouts_own_rules() {
        let repo = repo_with(&[(".gitignore", "target/\n")]);
        write(repo.path(), "tracked/target/keep", "k");
        git(repo.path(), &["add", "-f", "tracked/target/keep"]);
        git(repo.path(), &["commit", "-q", "-m", "a tracked file in an ignored folder"]);
        let existing = |rel: &str| is_ignored(repo.path(), Path::new(rel)).expect("asks");
        assert!(existing("target"), "ignored, though it does not exist");
        assert!(!existing("node_modules"), "no rule for it here");
        assert!(!existing("tracked/target"), "a tracked file keeps it from being ignored");
        git(repo.path(), &["checkout", "-q", "-b", "other"]);
        write(repo.path(), ".gitignore", "node_modules/\n");
        assert!(existing("node_modules") && !existing("target"));
        assert!(is_ignored(&repo.path().join("missing"), Path::new("target")).is_err());
    }
}
