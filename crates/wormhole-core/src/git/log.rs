//! Branches and commit history.

use std::path::Path;

use anyhow::Result;

use super::cmd::classify;
use super::lfs::mark_lfs;
use super::status::{changed_files, resolves};
use super::submodule::annotate_files;
use super::{Commit, DiffSpec, EMPTY_TREE, FileChange, GitCmd, GitError, has_head};

/// Local branches (current first), then remote-tracking branches.
pub fn branches(root: &Path) -> Result<Vec<String>> {
    let raw = GitCmd::new(root)
        .args([
            "for-each-ref",
            "--format=%(HEAD)%(refname)",
            "--sort=-committerdate",
            "refs/heads",
            "refs/remotes",
        ])
        .run()?;
    Ok(parse_branches(&String::from_utf8_lossy(&raw)))
}

pub(crate) fn parse_branches(text: &str) -> Vec<String> {
    let mut current = Vec::new();
    let mut rest = Vec::new();
    for line in text.lines() {
        let (is_current, refname) = match line.strip_prefix('*') {
            Some(name) => (true, name),
            None => (false, line.trim_start()),
        };
        let name = if let Some(name) = refname.strip_prefix("refs/heads/") {
            name
        } else if let Some(name) = refname.strip_prefix("refs/remotes/") {
            if name.ends_with("/HEAD") {
                continue;
            }
            name
        } else {
            continue;
        };
        if is_current { current.push(name.to_string()) } else { rest.push(name.to_string()) }
    }
    current.extend(rest);
    current
}

/// How many commits the History tab lists ("Latest 300 commits" when there are more).
pub const LOG_LIMIT: usize = 300;

/// Recent commits reachable from `rev`, at most `limit` of them.
pub fn log(root: &Path, rev: &str, limit: usize) -> Result<Vec<Commit>> {
    if !has_head(root) {
        return Ok(Vec::new());
    }
    // A branch may be called `--output=<file>`: the rev is never an option.
    let raw = GitCmd::new(root)
        .arg("log")
        .arg(format!("-n{limit}"))
        .arg("--format=%H%x1f%h%x1f%P%x1f%an%x1f%at%x1f%s%x1e")
        .revs([rev])
        .paths::<_, &str>([])
        .run()?;
    Ok(parse_log(&String::from_utf8_lossy(&raw)))
}

/// Why [`log`] failed, in the History tab's words: git's own line, and after it that the
/// branch was deleted when `rev` no longer names a commit (a branch picked earlier and deleted
/// since; git only says "bad revision"). Asked only after a failure, so a refresh that works
/// costs no extra process.
pub fn log_failure(root: &Path, rev: &str, err: &anyhow::Error) -> String {
    let message = format!("{err:#}");
    let failed = matches!(GitError::of(err), GitError::Failed(_));
    if failed && !resolves(root, rev) { branch_gone(&message) } else { message }
}

/// Git's line with why: "fatal: bad revision 'old-spike': the branch was deleted."
fn branch_gone(message: &str) -> String {
    format!("{}: the branch was deleted.", message.trim_end())
}

pub(crate) fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\x1e').filter_map(|record| parse_record(record.trim_start_matches('\n'))).collect()
}

/// One commit of `--format=%H%x1f%h%x1f%P%x1f%an%x1f%at%x1f%s`.
pub(crate) fn parse_record(record: &str) -> Option<Commit> {
    let fields: Vec<&str> = record.split('\x1f').collect();
    if fields.len() < 6 {
        return None;
    }
    Some(Commit {
        sha: fields[0].to_string(),
        short: fields[1].to_string(),
        parent: fields[2].split(' ').next().filter(|p| !p.is_empty()).map(str::to_string),
        author: fields[3].to_string(),
        time: fields[4].parse().unwrap_or(0),
        subject: fields[5].to_string(),
    })
}

/// Files touched by a commit (vs its first parent).
pub fn commit_files(root: &Path, commit: &Commit) -> Result<Vec<FileChange>> {
    let parent = commit.parent.as_deref().unwrap_or(EMPTY_TREE);
    let mut files = changed_files(root, parent, Some(&commit.sha))?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    mark_lfs(root, &mut files);
    annotate_files(root, &mut files);
    Ok(files)
}

pub fn commit_spec(commit: &Commit) -> DiffSpec {
    DiffSpec {
        old: commit.parent.clone().unwrap_or_else(|| EMPTY_TREE.to_string()),
        new: Some(commit.sha.clone()),
    }
}

/// Whether `commit` is reachable from `rev` (on that branch): `git merge-base --is-ancestor`,
/// which answers with its exit status alone. The command palette's commit keeps the History
/// tab's branch when this says so.
pub fn is_ancestor(root: &Path, commit: &str, rev: &str) -> Result<bool> {
    let output = GitCmd::new(root).args(["merge-base", "--is-ancestor"]).revs([commit, rev]).output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(classify(output.status, &String::from_utf8_lossy(&output.stderr)).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branches_current_first() {
        let text =
            " refs/heads/feature\n*refs/heads/main\n refs/remotes/origin/HEAD\n refs/remotes/origin/main\n";
        assert_eq!(parse_branches(text), vec!["main", "feature", "origin/main"]);
    }

    #[test]
    fn parses_log() {
        let text = "aaa\x1fa1\x1fppp qqq\x1fAda\x1f1700000000\x1fFix: thing\x1e\nbbb\x1fb1\x1f\x1fBob\x1f1600000000\x1fInitial\x1e\n";
        let commits = parse_log(text);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].parent.as_deref(), Some("ppp"));
        assert_eq!(commits[0].subject, "Fix: thing");
        assert_eq!(commits[1].parent, None);
        assert_eq!(commits[1].time, 1600000000);
    }

    #[test]
    fn the_log_of_a_deleted_branch_says_so_after_gits_line() {
        use crate::git::testing::{git, repo_with};
        let dir = repo_with(&[("a.txt", "1\n")]);
        let root = dir.path();
        git(root, &["branch", "gone"]);
        assert_eq!(log(root, "gone", 10).expect("the branch exists").len(), 1);
        git(root, &["branch", "-D", "-q", "gone"]);
        let err = log(root, "gone", 10).expect_err("the branch is gone");
        let said = log_failure(root, "gone", &err);
        // "A branch that's gone": git's words, then why.
        assert!(said.starts_with("fatal: bad revision 'gone'"), "{said}");
        assert!(said.ends_with("'gone': the branch was deleted."), "{said}");
        assert!(!said.contains('\n'), "{said}");
        // A failure that isn't about a missing branch is git's words alone.
        let other = anyhow::anyhow!("fatal: not a git repository");
        assert_eq!(log_failure(root, "HEAD", &other), "fatal: not a git repository");
        assert_eq!(
            branch_gone("fatal: bad revision 'x'\n"),
            "fatal: bad revision 'x': the branch was deleted."
        );
    }

    /// A commit on the branch History shows keeps that branch; one it hasn't got switches to
    /// HEAD. A name git doesn't know is an error, not "no".
    #[test]
    fn is_ancestor_follows_the_branches() {
        use super::super::testing::{git, repo_with, write};
        let repo = repo_with(&[("a.txt", "a")]);
        let root = repo.path();
        git(root, &["checkout", "-q", "-b", "feature"]);
        write(root, "b.txt", "b");
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "on feature"]);
        let feature = log(root, "feature", 1).expect("log").remove(0);
        let initial = log(root, "main", 1).expect("log").remove(0);
        assert!(is_ancestor(root, &initial.sha, "feature").expect("asked"));
        assert!(is_ancestor(root, &initial.sha, "main").expect("asked"));
        assert!(!is_ancestor(root, &feature.sha, "main").expect("asked"));
        assert!(is_ancestor(root, &feature.sha, "HEAD").expect("asked"));
        assert!(is_ancestor(root, &feature.sha, "no-such-branch").is_err());
    }
}
