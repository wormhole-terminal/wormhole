//! What lies between the two commits a parent records for a submodule, asked of the submodule's own
//! repository.
//!
//! The parent's `git status` knows only that the id moved. The commits live in the submodule, so
//! this runs `git log old...new` in its folder (through [`GitCmd`], with the hardening every other
//! call has: the folder is hostile input like any repository). The answer is kept per
//! (folder, old, new): a refresh every two seconds costs file reads to learn the submodule's
//! commit, and a process only when that pair is one it has not seen.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::Result;

use super::{Submodule, commit_id};
use crate::git::head::{POINTER_CAP, normalize, pointed_dir};
use crate::git::{Commit, FileChange, GitCmd, GitError, RepoStatus};
use crate::storage::fsutil::{LinkPolicy, read_capped};

/// The most commits listed on each side; more are counted ("and 340 more").
pub const COMMITS_SHOWN: usize = 200;

/// Submodules a refresh looks into; a repository with more is not a case worth a slower refresh.
const MAX_ANNOTATED: usize = 24;

/// Answers kept. A pair is small (a list of at most two times [`COMMITS_SHOWN`] commits).
const MAX_KEPT: usize = 128;

/// How long "those commits aren't in the clone" is believed: whoever fetches in the submodule
/// changes nothing the key can see.
const NOT_FETCHED_FOR: Duration = Duration::from_secs(60);

/// A `.git` pointer, a `HEAD` and a loose ref are one line.
const REF_CAP: u64 = 512;
/// `packed-refs` of a big repository.
const PACKED_CAP: u64 = 8 << 20;

/// Whether the submodule's folder can be asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkout {
    /// A repository of its own is there.
    Ready,
    /// The folder exists without a repository in it: the submodule was never initialised (or was
    /// deinitialised). Asking git there would answer for the parent.
    NotCheckedOut,
    /// There is no folder.
    FolderMissing,
}

/// What lies between the commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Range {
    /// Both sides record the same commit.
    Same,
    /// The commits only the new side has (`ahead`, newest first) and only the old side has
    /// (`behind`: the pointer went back, or to another line of history). The lists hold at most
    /// [`COMMITS_SHOWN`], the totals say how many there are.
    Commits { ahead: Vec<Commit>, behind: Vec<Commit>, ahead_total: usize, behind_total: usize },
    /// One of the commits is not in the submodule's clone (the parent was pulled, the submodule
    /// not fetched): there is nothing to list until it is.
    NotFetched { old: bool, new: bool },
    /// The submodule is new, so there is no old commit.
    NoBase,
    /// Not asked: the folder can't be (see [`Checkout`]), or it has no commit.
    Unavailable,
}

/// A submodule's commits between the base and the other side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmoduleLog {
    pub checkout: Checkout,
    pub old: Option<String>,
    /// The commit of the other side: the revision it records, or the folder's HEAD.
    pub new: Option<String>,
    pub range: Range,
}

/// What a kept answer is for.
type Key = (PathBuf, Option<String>, Option<String>);

static KEPT: Mutex<Vec<(Key, Instant, Arc<SubmoduleLog>)>> = Mutex::new(Vec::new());

/// Fill in [`Submodule::log`] of the submodules in `status`, without starting a process for a
/// pair already known (and none for a submodule whose commit can be read from its files).
pub fn annotate(status: &mut RepoStatus) {
    let root = status.root.clone();
    annotate_files(&root, &mut status.files);
}

/// [`annotate`] for the files of a commit, or any list of changes in the repository at `root`.
pub fn annotate_files(root: &Path, files: &mut [FileChange]) {
    let submodules = files.iter_mut().filter(|file| file.submodule.is_some()).take(MAX_ANNOTATED);
    for file in submodules {
        let Some(sub) = file.submodule.as_mut() else { continue };
        if file.kind == crate::git::ChangeKind::Deleted {
            continue;
        }
        match log_for(root, &file.path, sub, false) {
            Ok(log) => sub.log = log,
            Err(err) => tracing::debug!(path = %file.path, "submodule log failed: {err:#}"),
        }
    }
}

/// The commits between what the parent records for the submodule at `path` and the other side.
/// Asks git for the folder's HEAD if its files don't say; the answer is kept like [`annotate`]'s.
pub fn submodule_log(root: &Path, path: &str, sub: &Submodule) -> Result<Arc<SubmoduleLog>> {
    let log = log_for(root, path, sub, true)?;
    Ok(log.unwrap_or_else(|| {
        Arc::new(SubmoduleLog {
            checkout: Checkout::Ready,
            old: sub.old.clone(),
            new: None,
            range: Range::Unavailable,
        })
    }))
}

pub(super) fn log_for(
    root: &Path,
    path: &str,
    sub: &Submodule,
    ask_git: bool,
) -> Result<Option<Arc<SubmoduleLog>>> {
    let folder = root.join(path);
    let checkout = checkout(&folder);
    if checkout != Checkout::Ready {
        // Nothing to ask, and nothing worth keeping: a look at the folder is as cheap as a lookup.
        let range = Range::Unavailable;
        return Ok(Some(Arc::new(SubmoduleLog { checkout, old: sub.old.clone(), new: None, range })));
    }
    let new = match sub.new.clone().or_else(|| head_of(&folder)) {
        Some(new) => Some(new),
        None if ask_git => head_by_git(&folder),
        None => return Ok(None),
    };
    let key = (folder.clone(), sub.old.clone(), new.clone());
    if let Some(kept) = kept(&key) {
        return Ok(Some(kept));
    }
    let range = match (&sub.old, &new) {
        (None, _) => Range::NoBase,
        (Some(_), None) => Range::Unavailable,
        (Some(old), Some(new)) if old == new => Range::Same,
        (Some(old), Some(new)) => commits_between(&folder, old, new)?,
    };
    let log = Arc::new(SubmoduleLog { checkout, old: sub.old.clone(), new, range });
    keep(key, &log);
    Ok(Some(log))
}

fn kept(key: &Key) -> Option<Arc<SubmoduleLog>> {
    let kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    let (_, at, log) = kept.iter().find(|(k, ..)| k == key)?;
    let stale = matches!(log.range, Range::NotFetched { .. }) && at.elapsed() > NOT_FETCHED_FOR;
    (!stale).then(|| log.clone())
}

fn keep(key: Key, log: &Arc<SubmoduleLog>) {
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    kept.retain(|(k, ..)| *k != key);
    if kept.len() >= MAX_KEPT {
        kept.remove(0);
    }
    kept.push((key, Instant::now(), log.clone()));
}

/// Whether `folder` is a repository of its own. A link is not followed: a submodule that is a
/// symlink to somewhere else is not the parent's.
fn checkout(folder: &Path) -> Checkout {
    let Ok(meta) = std::fs::symlink_metadata(folder) else { return Checkout::FolderMissing };
    if !meta.is_dir() {
        return Checkout::NotCheckedOut;
    }
    match std::fs::symlink_metadata(folder.join(".git")) {
        Ok(dot_git) if !dot_git.file_type().is_symlink() => Checkout::Ready,
        _ => Checkout::NotCheckedOut,
    }
}

/// `git log --left-right old...new` in the submodule: `>` marks the commits of the new side.
fn commits_between(folder: &Path, old: &str, new: &str) -> Result<Range> {
    let asked = GitCmd::new(folder)
        .arg("log")
        .args(["--left-right", "--no-show-signature", "--no-color", "--encoding=UTF-8"])
        .arg(format!("-n{}", COMMITS_SHOWN + 1))
        .arg("--format=%m%x1f%H%x1f%h%x1f%P%x1f%an%x1f%at%x1f%s%x1e")
        .revs([format!("{old}...{new}")])
        .paths::<_, &str>([])
        .run();
    let raw = match asked {
        Ok(raw) => raw,
        Err(err) if matches!(GitError::of(&err), GitError::Failed(_)) => {
            // A commit the clone doesn't have is the usual reason; anything else is git's to say.
            let (old_missing, new_missing) = (!has_commit(folder, old), !has_commit(folder, new));
            if old_missing || new_missing {
                return Ok(Range::NotFetched { old: old_missing, new: new_missing });
            }
            return Err(err);
        }
        Err(err) => return Err(err),
    };
    let (mut ahead, mut behind) = (Vec::new(), Vec::new());
    for record in String::from_utf8_lossy(&raw).split('\x1e') {
        let record = record.trim_start_matches('\n');
        let mut chars = record.chars();
        let Some(side) = chars.next() else { continue };
        let Some(commit) = Commit::from_record(chars.as_str().trim_start_matches('\x1f')) else { continue };
        if side == '>' { ahead.push(commit) } else { behind.push(commit) }
    }
    let (mut ahead_total, mut behind_total) = (ahead.len(), behind.len());
    if ahead.len() + behind.len() > COMMITS_SHOWN {
        // Cut off: the lists are what fits, the totals what there are.
        if let Some((left, right)) = count_between(folder, old, new) {
            (behind_total, ahead_total) = (left.max(behind.len()), right.max(ahead.len()));
        }
        ahead.truncate(COMMITS_SHOWN);
        behind.truncate(COMMITS_SHOWN);
    }
    Ok(Range::Commits { ahead, behind, ahead_total, behind_total })
}

/// `git rev-list --left-right --count old...new`: (only in old, only in new).
fn count_between(folder: &Path, old: &str, new: &str) -> Option<(usize, usize)> {
    let raw = GitCmd::new(folder)
        .args(["rev-list", "--left-right", "--count"])
        .revs([format!("{old}...{new}")])
        .paths::<_, &str>([])
        .run()
        .ok()?;
    let text = String::from_utf8_lossy(&raw);
    let mut counts = text.split_whitespace().map(|n| n.parse().ok());
    Some((counts.next()??, counts.next()??))
}

fn has_commit(folder: &Path, id: &str) -> bool {
    GitCmd::new(folder)
        .args(["cat-file", "-e"])
        .revs([format!("{id}^{{commit}}")])
        .output()
        .is_ok_and(|out| out.status.success())
}

/// The commit the folder's HEAD is at, from its files alone: no process, so the panel can look at
/// every submodule on every refresh. `None` when they don't say (a layout this doesn't read).
pub(super) fn head_of(folder: &Path) -> Option<String> {
    let gitdir = git_dir(folder)?;
    let head = read_capped(&gitdir.join("HEAD"), REF_CAP, LinkPolicy::Within(&gitdir)).ok()?;
    let head = String::from_utf8_lossy(&head);
    match head.trim().strip_prefix("ref:") {
        Some(name) => resolve_ref(&gitdir, name.trim()),
        None => commit_id(head.trim()),
    }
}

/// The folder's git dir: `.git` itself, or what a submodule's `.git` file points to.
fn git_dir(folder: &Path) -> Option<PathBuf> {
    let dot_git = folder.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    if !meta.is_file() {
        return None;
    }
    let pointer = read_capped(&dot_git, POINTER_CAP, LinkPolicy::Within(folder)).ok()?;
    Some(normalize(&folder.join(pointed_dir(&String::from_utf8_lossy(&pointer))?)))
}

/// `refs/heads/main` as a loose ref, else from `packed-refs`.
fn resolve_ref(gitdir: &Path, name: &str) -> Option<String> {
    if !name.starts_with("refs/") {
        return None;
    }
    if let Ok(loose) = read_capped(&gitdir.join(name), REF_CAP, LinkPolicy::Within(gitdir)) {
        return commit_id(String::from_utf8_lossy(&loose).trim());
    }
    let packed = read_capped(&gitdir.join("packed-refs"), PACKED_CAP, LinkPolicy::Within(gitdir)).ok()?;
    String::from_utf8_lossy(&packed).lines().find_map(|line| {
        let (id, refname) = line.split_once(' ')?;
        (refname == name).then(|| commit_id(id)).flatten()
    })
}

/// `git rev-parse HEAD` in the folder, for layouts [`head_of`] does not read.
fn head_by_git(folder: &Path) -> Option<String> {
    let raw =
        GitCmd::new(folder).args(["rev-parse", "--verify", "--quiet"]).revs(["HEAD^{commit}"]).run().ok()?;
    commit_id(String::from_utf8_lossy(&raw).trim())
}
