//! Working-tree status, and the files changed between two revisions.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use super::counts::count_untracked_lines;
use super::lfs::mark_lfs;
use super::status_parse::{parse_numstat, parse_porcelain, parse_raw};
use super::{
    ChangeKind, EMPTY_TREE, FileChange, GitCmd, GitError, RepoStatus, Upstream, has_head, repo_root,
    superproject,
};
use crate::view::title_extras::BranchStatus;

/// How much of the picture a status call fills in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Detail {
    /// Every file with its line counts: `diff --numstat`, and the lines of each untracked file.
    Full,
    /// Which files changed and how, nothing that has to look inside them.
    Summary,
}

/// Status of the repository containing `cwd`, or `None` if it isn't in one.
///
/// With `compare = Some(branch)`, tracked changes are everything since the
/// merge-base with that branch (committed and uncommitted), like a PR. A branch
/// that no longer exists falls back to HEAD and is named in
/// [`RepoStatus::compare_missing`].
pub fn status(cwd: &Path, compare: Option<&str>) -> Result<Option<RepoStatus>> {
    status_with(cwd, compare, Detail::Full)
}

/// [`status`] without the line counts: every file's `added` and `removed` are `None`.
///
/// What it saves is what grows with the repository's content rather than its file list: no
/// `git diff --numstat` (a process and a pass over every changed file) and no reading of
/// untracked files. For a session nobody is looking at, which only needs "N files changed", this
/// costs two git processes (three with `compare`) and does not depend on how big the untracked
/// files are; `status` costs one more process and a read of each untracked file, every time.
pub fn status_summary(cwd: &Path, compare: Option<&str>) -> Result<Option<RepoStatus>> {
    status_with(cwd, compare, Detail::Summary)
}

fn status_with(cwd: &Path, compare: Option<&str>, detail: Detail) -> Result<Option<RepoStatus>> {
    let Some(root) = repo_root(cwd)? else {
        return Ok(None);
    };

    let porcelain = GitCmd::new(&root)
        .args(["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"])
        .run()?;
    let parsed = parse_porcelain(&porcelain);
    let (branch, mut files) = (parsed.branch, parsed.files);
    // The tree's own state, before "Compare with" swaps the tracked files for the diff.
    let dirty = !files.is_empty();

    // The header says whether HEAD exists, so no process is needed to ask.
    let head = parsed.has_head.unwrap_or_else(|| has_head(&root));
    let (mut compare, mut compare_missing) = (compare, None);
    let base_rev = match compare.filter(|_| head) {
        Some(branch) => match merge_base(&root, branch) {
            Ok(base) => base,
            // The branch was deleted since it was picked (or shares no history with HEAD): show
            // the changes against HEAD rather than nothing, and name a deleted branch so the
            // panel can say so. Asked only after a failure, so a refresh costs no extra process.
            Err(err) if matches!(GitError::of(&err), GitError::Failed(_)) => {
                if !resolves(&root, branch) {
                    compare_missing = Some(branch.to_string());
                }
                compare = None;
                "HEAD".to_string()
            }
            Err(err) => return Err(err),
        },
        None if head => "HEAD".to_string(),
        None => EMPTY_TREE.to_string(),
    };

    if compare.is_some() && head {
        // Tracked files come from the diff against the merge-base; keep only
        // untracked (and conflicted) entries from `status`.
        let in_status: Vec<FileChange> = files.iter().filter(|f| f.submodule.is_some()).cloned().collect();
        files.retain(|f| matches!(f.kind, ChangeKind::Untracked | ChangeKind::Conflicted));
        files.extend(match detail {
            Detail::Full => changed_files(&root, &base_rev, None)?,
            Detail::Summary => changed_paths(&root, &base_rev, None)?,
        });
        carry_submodule_state(&mut files, &in_status);
    } else if detail == Detail::Full {
        let numstat = GitCmd::diff(&root)
            .args(["--numstat", "-z", "-M"])
            .revs([&base_rev])
            .paths::<_, &str>([])
            .run()?;
        apply_counts(&mut files, &parse_numstat(&numstat));
    }
    if detail == Detail::Full {
        count_untracked_lines(&root, &mut files);
        mark_lfs(&root, &mut files);
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let superproject = superproject(&root);
    Ok(Some(RepoStatus {
        root,
        branch,
        files,
        base_rev,
        compare: compare.map(str::to_string),
        compare_missing,
        dirty,
        upstream: parsed.upstream,
        superproject,
    }))
}

/// Where `branch` split off from HEAD.
fn merge_base(root: &Path, branch: &str) -> Result<String> {
    let base = GitCmd::new(root).arg("merge-base").revs([branch, "HEAD"]).run()?;
    Ok(String::from_utf8_lossy(&base).trim().to_string())
}

/// Whether `rev` names a commit.
pub(super) fn resolves(root: &Path, rev: &str) -> bool {
    GitCmd::new(root)
        .args(["rev-parse", "--verify", "--quiet"])
        .revs([format!("{rev}^{{commit}}")])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Fill in `added`/`removed` by path. One map instead of a scan per file: with 10 000 changed
/// files the scan was most of the time after git's own (430 ms per refresh, 300 ms of it git).
fn apply_counts(files: &mut [FileChange], counts: &[(String, Option<u32>, Option<u32>)]) {
    let mut by_path: HashMap<&str, (Option<u32>, Option<u32>)> = HashMap::with_capacity(counts.len());
    for (path, added, removed) in counts {
        // The first entry for a path wins, as the linear search it replaces did.
        by_path.entry(path.as_str()).or_insert((*added, *removed));
    }
    for file in files {
        if let Some(&(added, removed)) = by_path.get(file.path.as_str()) {
            file.added = added;
            file.removed = removed;
        }
    }
}

impl RepoStatus {
    /// What this status (the Changes tab's, so no git is run for it) says about the branch for the
    /// title row, if there is anything to say.
    pub fn branch_status(&self) -> Option<BranchStatus> {
        let (ahead, behind, upstream) = match &self.upstream {
            Some(Upstream { name, ahead, behind }) => (*ahead, *behind, Some(name.clone())),
            None => (0, 0, None),
        };
        let this = BranchStatus { dirty: self.dirty, ahead, behind, upstream };
        (this.dirty || ahead > 0 || behind > 0).then_some(this)
    }
}

/// Files that differ between `old` and `new` (`None` = working tree).
pub fn changed_files(root: &Path, old: &str, new: Option<&str>) -> Result<Vec<FileChange>> {
    let mut files = changed_paths(root, old, new)?;
    let range: Vec<&str> = std::iter::once(old).chain(new).collect();
    let numstat =
        GitCmd::diff(root).args(["--numstat", "-z", "-M"]).revs(&range).paths::<_, &str>([]).run()?;
    apply_counts(&mut files, &parse_numstat(&numstat));
    Ok(files)
}

/// [`changed_files`] without the line counts.
fn changed_paths(root: &Path, old: &str, new: Option<&str>) -> Result<Vec<FileChange>> {
    let range: Vec<&str> = std::iter::once(old).chain(new).collect();
    // `--raw` rather than `--name-status`: the modes and ids say which entries are submodules.
    let names = GitCmd::diff(root)
        .args(["--raw", "-z", "-M", "--no-abbrev"])
        .revs(&range)
        .paths::<_, &str>([])
        .run()?;
    Ok(parse_raw(&names))
}

/// With "Compare with", the tracked files come from a diff against the merge-base, which knows a
/// submodule's commits but not what `git status` found inside its folder: take that from the
/// status's own entry for it.
fn carry_submodule_state(files: &mut [FileChange], from_status: &[FileChange]) {
    for file in files.iter_mut() {
        let (Some(sub), Some(seen)) =
            (file.submodule.as_mut(), from_status.iter().find(|f| f.path == file.path))
        else {
            continue;
        };
        if let Some(seen) = &seen.submodule {
            (sub.tracked, sub.untracked, sub.removed) = (seen.tracked, seen.untracked, seen.removed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Upstream;

    /// A repository with a bare upstream, one commit ahead and one behind, and a file edited since.
    #[test]
    fn status_says_how_far_a_branch_is_from_its_upstream_and_whether_the_tree_is_dirty() {
        use crate::git::testing::{git, repo_with, spawns, write};
        let upstream = tempfile::tempdir().expect("tempdir");
        git(upstream.path(), &["init", "-q", "--bare", "-b", "main"]);
        let dir = repo_with(&[("a.txt", "1\n")]);
        let root = dir.path();
        let remote = upstream.path().to_str().expect("utf-8 path");
        git(root, &["remote", "add", "origin", remote]);
        git(root, &["push", "-q", "-u", "origin", "main"]);

        let clean = status(root, None).expect("status").expect("repo");
        assert!(!clean.dirty);
        assert_eq!(clean.upstream, Some(Upstream { name: "origin/main".into(), ahead: 0, behind: 0 }));

        // One commit here, one made elsewhere and fetched.
        write(root, "b.txt", "2\n");
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "local"]);
        let other = tempfile::tempdir().expect("tempdir");
        git(other.path(), &["clone", "-q", remote, "."]);
        write(other.path(), "c.txt", "3\n");
        git(other.path(), &["add", "-A"]);
        git(other.path(), &["commit", "-q", "-m", "elsewhere"]);
        git(other.path(), &["push", "-q", "origin", "main"]);
        git(root, &["fetch", "-q"]);

        write(root, "a.txt", "edited\n");
        let both = status(root, None).expect("status").expect("repo");
        assert!(both.dirty);
        assert_eq!(both.upstream, Some(Upstream { name: "origin/main".into(), ahead: 1, behind: 1 }));
        // The same call: no process more than the status itself costs.
        assert_eq!(spawns(|| drop(status(root, None))), 3);
        // The tree's state and the branch's don't depend on what "Compare with" lists.
        git(root, &["branch", "side"]);
        let compared = status(root, Some("side")).expect("status").expect("repo");
        assert_eq!((compared.dirty, compared.upstream), (both.dirty, both.upstream));
    }

    #[test]
    fn a_branch_without_an_upstream_has_none_and_a_clean_tree_is_not_dirty() {
        use crate::git::testing::repo_with;
        let dir = repo_with(&[("a.txt", "1\n")]);
        let status = status(dir.path(), None).expect("status").expect("repo");
        assert_eq!((status.dirty, status.upstream), (false, None));
    }

    #[test]
    fn counts_are_matched_by_path_first_entry_wins() {
        let file = |path: &str| FileChange::new(path, ChangeKind::Modified);
        let mut files = vec![file("a"), file("b"), file("c")];
        let counts = vec![
            ("b".to_string(), Some(2), Some(1)),
            ("a".to_string(), None, None),
            ("b".to_string(), Some(9), Some(9)),
        ];
        apply_counts(&mut files, &counts);
        let got: Vec<_> = files.iter().map(|f| (f.added, f.removed)).collect();
        assert_eq!(got, [(None, None), (Some(2), Some(1)), (None, None)]);
    }

    /// 10 000 changed files: the per-file scan this replaced against the map.
    /// `cargo test -p wormhole-core counting_ -- --ignored --nocapture`
    #[test]
    #[ignore = "benchmark"]
    fn counting_10000_changed_files() {
        let n = 10_000;
        let counts: Vec<(String, Option<u32>, Option<u32>)> =
            (0..n).map(|i| (format!("src/module_{i}/file_{i}.rs"), Some(i as u32), Some(1))).collect();
        let fresh = || -> Vec<FileChange> {
            counts
                .iter()
                .rev()
                .map(|(path, ..)| FileChange::new(path.clone(), ChangeKind::Modified))
                .collect()
        };
        let mut files = fresh();
        let t = std::time::Instant::now();
        for file in &mut files {
            if let Some((_, added, removed)) = counts.iter().find(|(path, ..)| *path == file.path) {
                file.added = *added;
                file.removed = *removed;
            }
        }
        eprintln!("linear find: {:?}", t.elapsed());
        let mut files = fresh();
        let t = std::time::Instant::now();
        apply_counts(&mut files, &counts);
        eprintln!("map: {:?}", t.elapsed());
        assert!(files.iter().all(|f| f.added.is_some()));
    }

    #[test]
    fn status_costs_three_git_processes_and_a_summary_two() {
        use crate::git::testing::{git, repo_with, spawns, write};
        let dir = repo_with(&[("a.txt", "1\n")]);
        let root = dir.path();
        write(root, "a.txt", "2\n");
        write(root, "new.txt", "x\ny\n");
        git(root, &["branch", "side"]);
        // root, status, numstat (it used to ask `rev-parse HEAD` as well).
        assert_eq!(spawns(|| drop(status(root, None))), 3);
        assert_eq!(spawns(|| drop(status_summary(root, None))), 2);
        // + merge-base, and the name-status/numstat pair instead of the numstat.
        assert_eq!(spawns(|| drop(status(root, Some("side")))), 5);
        assert_eq!(spawns(|| drop(status_summary(root, Some("side")))), 4);
    }

    #[test]
    #[cfg(unix)]
    fn status_passes_on_why_git_failed() {
        use crate::git::GitError;
        use crate::git::testing::{DUBIOUS, fake_git, repo_with, with_git};
        let dir = repo_with(&[("a.txt", "1\n")]);
        let tools = tempfile::tempdir().expect("tempdir");
        let dubious = fake_git(tools.path(), "dubious", DUBIOUS, 128);
        for summary in [false, true] {
            let run = |program: &std::path::Path| {
                with_git(program, || {
                    if summary { status_summary(dir.path(), None) } else { status(dir.path(), None) }
                })
            };
            let err = run(&dubious).expect_err("git failed");
            assert_eq!(GitError::of(&err), GitError::Failed(DUBIOUS.into()));
            let err = run(&tools.path().join("absent")).expect_err("no git");
            assert_eq!(GitError::of(&err), GitError::GitMissing);
        }
    }

    #[test]
    fn a_deleted_compare_branch_falls_back_to_head_and_says_so() {
        use crate::git::testing::{git, repo_with, spawns, write};
        let dir = repo_with(&[("a.txt", "1\n")]);
        let root = dir.path();
        git(root, &["branch", "gone"]);
        write(root, "a.txt", "2\n");
        assert_eq!(status(root, Some("gone")).expect("status").expect("repo").compare_missing, None);
        git(root, &["branch", "-D", "-q", "gone"]);

        let head = status(root, None).expect("status").expect("repo");
        for full in [true, false] {
            let fell_back =
                if full { status(root, Some("gone")) } else { status_summary(root, Some("gone")) };
            let fell_back = fell_back.expect("not an error").expect("repo");
            assert_eq!(fell_back.compare_missing.as_deref(), Some("gone"));
            assert_eq!((fell_back.compare.as_deref(), fell_back.base_rev.as_str()), (None, "HEAD"));
            let paths = |s: &RepoStatus| s.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>();
            assert_eq!(paths(&fell_back), paths(&head));
        }
        // root, status, merge-base, the check that the branch is gone, numstat.
        assert_eq!(spawns(|| drop(status(root, Some("gone")))), 5);
    }

    #[test]
    fn a_branch_with_no_history_in_common_compares_with_head_as_before() {
        use crate::git::testing::{git, repo_with};
        let dir = repo_with(&[("a.txt", "1\n")]);
        let root = dir.path();
        git(root, &["checkout", "-q", "--orphan", "other"]);
        git(root, &["commit", "-q", "-m", "unrelated"]);
        git(root, &["checkout", "-q", "main"]);
        let status = status(root, Some("other")).expect("status").expect("repo");
        // It exists, so it is not "deleted"; there is just no merge-base to compare with.
        assert_eq!((status.compare, status.compare_missing, status.base_rev), (None, None, "HEAD".into()));
    }

    #[test]
    fn a_summary_lists_what_status_lists_without_reading_files() {
        use crate::git::testing::{repo_with, write};
        let dir = repo_with(&[("a.txt", "1\n"), ("gone.txt", "bye\n")]);
        let root = dir.path();
        write(root, "a.txt", "1\n2\n");
        write(root, "new.txt", "x\ny\n");
        std::fs::remove_file(root.join("gone.txt")).expect("rm");
        for compare in [None, Some("main")] {
            let full = status(root, compare).expect("status").expect("repo");
            let summary = status_summary(root, compare).expect("summary").expect("repo");
            assert_eq!(
                (&summary.root, &summary.branch, &summary.base_rev, &summary.compare),
                (&full.root, &full.branch, &full.base_rev, &full.compare)
            );
            let listed = |s: &RepoStatus| -> Vec<(String, ChangeKind)> {
                s.files.iter().map(|f| (f.path.clone(), f.kind)).collect()
            };
            assert_eq!(listed(&summary), listed(&full), "{compare:?}");
            assert!(summary.files.iter().all(|f| f.added.is_none() && f.removed.is_none()));
            assert!(full.files.iter().any(|f| f.added.is_some()));
        }
    }

    fn summary(dirty: bool, upstream: Option<(u32, u32)>) -> RepoStatus {
        RepoStatus {
            root: "/repo".into(),
            branch: Some("main".into()),
            files: Vec::new(),
            base_rev: "HEAD".into(),
            compare: None,
            compare_missing: None,
            dirty,
            upstream: upstream.map(|(ahead, behind)| Upstream { name: "origin/main".into(), ahead, behind }),
            superproject: None,
        }
    }

    #[test]
    fn only_departures_have_a_branch_status() {
        assert_eq!(summary(false, None).branch_status(), None);
        assert_eq!(summary(false, Some((0, 0))).branch_status(), None, "level with its upstream");
        let dirty = summary(true, None).branch_status().expect("dirty");
        assert_eq!((dirty.dirty, dirty.ahead, dirty.behind), (true, 0, 0));
        let both = summary(false, Some((2, 1))).branch_status().expect("ahead and behind");
        assert_eq!((both.dirty, both.ahead, both.behind), (false, 2, 1));
        assert_eq!(both.upstream.as_deref(), Some("origin/main"));
    }
}
