//! Working-tree status, and the files changed between two revisions.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use super::counts::count_untracked_lines;
use super::lfs::mark_lfs;
use super::{
    ChangeKind, EMPTY_TREE, FileChange, GitCmd, GitError, RepoStatus, Upstream, has_head, repo_root,
};

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
        files.retain(|f| matches!(f.kind, ChangeKind::Untracked | ChangeKind::Conflicted));
        files.extend(match detail {
            Detail::Full => changed_files(&root, &base_rev, None)?,
            Detail::Summary => changed_paths(&root, &base_rev, None)?,
        });
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
    Ok(Some(RepoStatus {
        root,
        branch,
        files,
        base_rev,
        compare: compare.map(str::to_string),
        compare_missing,
        dirty,
        upstream: parsed.upstream,
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
    let names =
        GitCmd::diff(root).args(["--name-status", "-z", "-M"]).revs(&range).paths::<_, &str>([]).run()?;
    Ok(parse_name_status(&names))
}

/// Parse `git diff --name-status -z -M`: `M\0path\0`, `R100\0old\0new\0`.
pub(crate) fn parse_name_status(raw: &[u8]) -> Vec<FileChange> {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0').filter(|f| !f.is_empty());
    let mut files = Vec::new();
    while let Some(status) = fields.next() {
        let (kind, original) = match status.chars().next() {
            Some('A') => (ChangeKind::Added, None),
            Some('D') => (ChangeKind::Deleted, None),
            Some('R') | Some('C') => (ChangeKind::Renamed, fields.next().map(str::to_string)),
            Some('U') => (ChangeKind::Conflicted, None),
            _ => (ChangeKind::Modified, None),
        };
        let Some(path) = fields.next() else { break };
        files.push(FileChange {
            path: path.to_string(),
            original_path: original,
            kind,
            added: None,
            removed: None,
            lfs: false,
        });
    }
    files
}

/// What `git status --porcelain=v2 --branch -z` says.
struct Porcelain {
    branch: Option<String>,
    /// From `# branch.oid`: `(initial)` means no commit yet. `None` if the header is missing.
    has_head: Option<bool>,
    /// From `# branch.upstream` and `# branch.ab`, which git prints only for a branch whose
    /// upstream exists.
    upstream: Option<Upstream>,
    files: Vec<FileChange>,
}

/// Parse `git status --porcelain=v2 --branch -z`.
fn parse_porcelain(raw: &[u8]) -> Porcelain {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0').peekable();
    let mut branch = None;
    let mut has_head = None;
    let mut upstream_name = None;
    let mut counts = None;
    let mut files = Vec::new();

    let kind_from_xy = |xy: &str| -> ChangeKind {
        let mut chars = xy.chars();
        let (x, y) = (chars.next().unwrap_or('.'), chars.next().unwrap_or('.'));
        if x == 'A' {
            ChangeKind::Added
        } else if x == 'D' || y == 'D' {
            ChangeKind::Deleted
        } else {
            ChangeKind::Modified
        }
    };

    while let Some(entry) = fields.next() {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.splitn(2, ' ');
        let tag = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default();
        match tag {
            "#" => {
                if let Some(head) = rest.strip_prefix("branch.head ")
                    && head != "(detached)"
                {
                    branch = Some(head.to_string());
                } else if let Some(oid) = rest.strip_prefix("branch.oid ") {
                    has_head = Some(oid != "(initial)");
                } else if let Some(name) = rest.strip_prefix("branch.upstream ") {
                    upstream_name = Some(name.to_string());
                } else if let Some(ab) = rest.strip_prefix("branch.ab ") {
                    counts = parse_ahead_behind(ab);
                }
            }
            // 1 XY sub mH mI mW hH hI path
            "1" => {
                let cols: Vec<&str> = rest.splitn(8, ' ').collect();
                if cols.len() == 8 {
                    files.push(FileChange {
                        path: cols[7].to_string(),
                        original_path: None,
                        kind: kind_from_xy(cols[0]),
                        added: None,
                        removed: None,
                        lfs: false,
                    });
                }
            }
            // 2 XY sub mH mI mW hH hI Xscore path \0 origPath
            "2" => {
                let cols: Vec<&str> = rest.splitn(9, ' ').collect();
                let original = fields.next().map(str::to_string);
                if cols.len() == 9 {
                    files.push(FileChange {
                        path: cols[8].to_string(),
                        original_path: original,
                        kind: ChangeKind::Renamed,
                        added: None,
                        removed: None,
                        lfs: false,
                    });
                }
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 path
            "u" => {
                let cols: Vec<&str> = rest.splitn(10, ' ').collect();
                if cols.len() == 10 {
                    files.push(FileChange {
                        path: cols[9].to_string(),
                        original_path: None,
                        kind: ChangeKind::Conflicted,
                        added: None,
                        removed: None,
                        lfs: false,
                    });
                }
            }
            "?" => files.push(FileChange {
                path: rest.to_string(),
                original_path: None,
                kind: ChangeKind::Untracked,
                added: None,
                removed: None,
                lfs: false,
            }),
            _ => {}
        }
    }
    // Both lines or nothing: an upstream that is gone has a name but no counts.
    let upstream = upstream_name.zip(counts).map(|(name, (ahead, behind))| Upstream { name, ahead, behind });
    Porcelain { branch, has_head, upstream, files }
}

/// `+2 -1`, what `# branch.ab` says: commits ahead of and behind the upstream.
fn parse_ahead_behind(text: &str) -> Option<(u32, u32)> {
    let (ahead, behind) = text.split_once(' ')?;
    Some((ahead.strip_prefix('+')?.parse().ok()?, behind.strip_prefix('-')?.parse().ok()?))
}

/// Parse `git diff --numstat -z [-M]`: `added\tremoved\tpath\0`, or for
/// renames `added\tremoved\t\0old\0new\0` (keyed by the new path).
pub(crate) fn parse_numstat(raw: &[u8]) -> Vec<(String, Option<u32>, Option<u32>)> {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0');
    let mut out = Vec::new();
    while let Some(entry) = fields.next() {
        let mut cols = entry.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (cols.next(), cols.next(), cols.next()) else {
            continue;
        };
        let path = if path.is_empty() {
            let _old = fields.next();
            fields.next().unwrap_or_default()
        } else {
            path
        };
        out.push((path.to_string(), added.parse().ok(), removed.parse().ok()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2() {
        let raw = b"# branch.oid abc\0# branch.head main\0\
1 .M N... 100644 100644 100644 aaa bbb src/main.rs\0\
1 A. N... 000000 100644 100644 000 ccc new file.txt\0\
1 .D N... 100644 100644 000000 ddd ddd gone.rs\0\
2 R. N... 100644 100644 100644 eee eee R100 renamed.rs\0old.rs\0\
u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
? notes/todo.md\0";
        let Porcelain { branch, files, .. } = parse_porcelain(raw);
        assert_eq!(branch.as_deref(), Some("main"));
        let kinds: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("src/main.rs", ChangeKind::Modified),
                ("new file.txt", ChangeKind::Added),
                ("gone.rs", ChangeKind::Deleted),
                ("renamed.rs", ChangeKind::Renamed),
                ("conflict.rs", ChangeKind::Conflicted),
                ("notes/todo.md", ChangeKind::Untracked),
            ]
        );
        assert_eq!(files[3].original_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn detached_head_has_no_branch() {
        let branch = parse_porcelain(b"# branch.head (detached)\0").branch;
        assert_eq!(branch, None);
    }

    #[test]
    fn parses_numstat() {
        let counts = parse_numstat(b"3\t1\tsrc/main.rs\0-\t-\timage.png\0");
        assert_eq!(counts[0], ("src/main.rs".into(), Some(3), Some(1)));
        assert_eq!(counts[1], ("image.png".into(), None, None));
    }

    #[test]
    fn parses_numstat_with_renames() {
        let counts = parse_numstat(b"2\t0\t\0old.rs\0new.rs\0");
        assert_eq!(counts, vec![("new.rs".into(), Some(2), Some(0))]);
    }

    #[test]
    fn parses_name_status() {
        let files = parse_name_status(b"M\0a.rs\0R087\0old.rs\0new.rs\0A\0b.rs\0D\0c.rs\0");
        let kinds: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("a.rs", ChangeKind::Modified),
                ("new.rs", ChangeKind::Renamed),
                ("b.rs", ChangeKind::Added),
                ("c.rs", ChangeKind::Deleted),
            ]
        );
        assert_eq!(files[1].original_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn the_branch_header_says_whether_there_is_a_commit() {
        let initial = parse_porcelain(b"# branch.oid (initial)\0# branch.head main\0");
        assert_eq!((initial.has_head, initial.branch.as_deref()), (Some(false), Some("main")));
        let born =
            parse_porcelain(b"# branch.oid 4b825dc642cb6eb9a060e54bf8d69288fbee4904\0# branch.head main\0");
        assert_eq!(born.has_head, Some(true));
        assert_eq!(parse_porcelain(b"? a\0").has_head, None, "no header: ask git");
    }

    #[test]
    fn the_branch_header_carries_the_upstream_and_how_far_apart_they_are() {
        let tracked =
            parse_porcelain(b"# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0");
        assert_eq!(tracked.upstream, Some(Upstream { name: "origin/main".into(), ahead: 2, behind: 1 }));
        // No upstream: git prints neither line. An upstream that is gone: only the name.
        assert_eq!(parse_porcelain(b"# branch.head main\0").upstream, None);
        let gone = parse_porcelain(b"# branch.head main\0# branch.upstream origin/old\0");
        assert_eq!(gone.upstream, None, "no counts to show");
        // Not numbers: nothing rather than a wrong count.
        let odd = parse_porcelain(b"# branch.upstream origin/main\0# branch.ab 2 1\0");
        assert_eq!(odd.upstream, None);
    }

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
        let file = |path: &str| FileChange {
            path: path.into(),
            original_path: None,
            kind: ChangeKind::Modified,
            added: None,
            removed: None,
            lfs: false,
        };
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
                .map(|(path, ..)| FileChange {
                    path: path.clone(),
                    original_path: None,
                    kind: ChangeKind::Modified,
                    added: None,
                    removed: None,
                    lfs: false,
                })
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
}
