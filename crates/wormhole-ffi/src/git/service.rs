//! Reading a repository for the Changes and History tabs. Every call is blocking (a few git processes, each
//! with core's timeout and output cap), so Swift calls them off the main thread; nothing here keeps a process
//! or a lock between calls except the small lists that let a diff be asked for by path alone.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::config::SizeUnits;
use wormhole_core::git::branch::head_of;
use wormhole_core::git::{
    self, Commit, DIFF_ASK_BYTES, DiffLimits, DiffSpec, FileChange, LOG_LIMIT, Range, STASH_REV, find_root,
};

use super::error::GitBackendError;
use super::records::{GitChange, GitDiff, GitDiffFile, GitLog, GitLogCommit, GitRev, GitRevKind, GitStatus};
use super::shown::{GitCommitDetails, GitCompact, Message};

/// Recent commits the revision picker offers to compare with.
const RECENT_COMMITS: usize = 10;

/// What the last status of a (repository, compared revision) listed, which is what a diff is made against.
struct Listing {
    base_rev: String,
    files: Vec<FileChange>,
}

#[derive(Default)]
struct Kept {
    listings: HashMap<(PathBuf, Option<String>), Listing>,
    commits: HashMap<(PathBuf, String), Opened>,
}

/// A commit that was opened: what a diff of one of its files is made against.
#[derive(Clone)]
struct Opened {
    sha: String,
    parent: Option<String>,
    files: Vec<FileChange>,
    message: Message,
}

/// Repositories, read-only, through core's `GitCmd`. One per app; it holds no repository open.
#[derive(uniffi::Object)]
pub struct Git {
    kept: Mutex<Kept>,
    /// Sizes read as git writes them (KiB) rather than as Finder does (KB).
    binary_units: AtomicBool,
}

#[uniffi::export]
impl Git {
    /// Refuses to start outside the experiment's own data folder (`isolation`).
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, GitBackendError> {
        crate::isolation::require()?;
        Ok(Arc::new(Self { kept: Mutex::default(), binary_units: AtomicBool::new(false) }))
    }

    /// `size_units` of the settings: sizes in a diff's words are KiB when true.
    pub fn set_binary_units(&self, binary: bool) {
        self.binary_units.store(binary, Ordering::Relaxed);
    }

    /// Whether `folder` is in a repository: a look for `.git`, no process, so it costs nothing to ask first.
    pub fn is_repository(&self, folder: String) -> bool {
        find_root(Path::new(&folder)).is_some()
    }

    /// The repository root of `folder` (the band's folder: what the Files tree and the project's colour are of),
    /// `None` outside a repository. A look for `.git`, no process.
    pub fn repository_root(&self, folder: String) -> Option<String> {
        find_root(Path::new(&folder)).map(|root| root.to_string_lossy().into_owned())
    }

    /// The status of the working tree, or what changed since the merge-base with `compare` (any branch, tag
    /// or commit) plus the untracked files. Line counts, LFS marks and submodule words included.
    pub fn status(&self, folder: String, compare: Option<String>) -> Result<GitStatus, GitBackendError> {
        // Kept under the root `diff` finds the same way (`find_root`), not the real path git prints.
        let root = root_of(&folder)?;
        let mut status = git::status(Path::new(&folder), compare.as_deref())
            .map_err(|err| GitBackendError::of(&err))?
            .ok_or(GitBackendError::NotARepository)?;
        git::annotate(&mut status);
        let out = GitStatus {
            files: status.files.iter().map(GitChange::of).collect(),
            branch: status.branch.clone(),
            dirty: status.dirty,
            upstream: status.upstream.as_ref().map(|up| up.name.clone()),
            ahead: status.upstream.as_ref().map_or(0, |up| up.ahead),
            behind: status.upstream.as_ref().map_or(0, |up| up.behind),
            compare_missing: status.compare_missing.clone(),
        };
        self.kept()
            .listings
            .insert((root, compare), Listing { base_rev: status.base_rev, files: status.files });
        Ok(out)
    }

    /// One file's diff against the working tree (or `compare`, as `status` listed it). `load_anyway` asks
    /// again for a diff that came back `TooLarge`.
    pub fn diff(
        &self,
        folder: String,
        path: String,
        compare: Option<String>,
        load_anyway: bool,
    ) -> Result<GitDiff, GitBackendError> {
        let root = root_of(&folder)?;
        let key = (root.clone(), compare.clone());
        let listed = |kept: &Kept| {
            kept.listings.get(&key).and_then(|l| {
                l.files.iter().find(|f| f.path == path).map(|f| (f.clone(), l.base_rev.clone()))
            })
        };
        let found = listed(&self.kept());
        // A path the last status did not list (or none yet): read it again, as the panel just did.
        let (file, base_rev) = match found {
            Some(found) => found,
            None => {
                self.status(folder, compare)?;
                listed(&self.kept())
                    .ok_or_else(|| GitBackendError::failed(format!("{path} has no changes to show")))?
            }
        };
        self.file_diff(&root, &file, DiffSpec { old: base_rev, new: None }, load_anyway)
    }

    /// The log of `rev` (HEAD when none, the stashes for the stashes), at most `LOG_LIMIT` commits, limited to
    /// the commits that touched `paths` when given.
    pub fn log(
        &self,
        folder: String,
        rev: Option<GitRev>,
        paths: Vec<String>,
    ) -> Result<GitLog, GitBackendError> {
        let root = root_of(&folder)?;
        let target = match &rev {
            None => "HEAD",
            Some(rev) if rev.kind == GitRevKind::Stashes => STASH_REV,
            Some(rev) => rev.name.as_str(),
        };
        let found = if paths.is_empty() {
            git::log(&root, target, LOG_LIMIT + 1)
        } else {
            git::log_paths(&root, target, LOG_LIMIT + 1, &paths)
        };
        let mut commits = found.map_err(|err| match GitBackendError::of(&err) {
            GitBackendError::Failed { .. } => GitBackendError::failed(git::log_failure(&root, target, &err)),
            other => other,
        })?;
        let limited = commits.len() > LOG_LIMIT;
        commits.truncate(LOG_LIMIT);
        let branch = match &rev {
            Some(rev) => Some(rev.name.clone()),
            None => head_of(&root).map(|head| head.label().to_string()),
        };
        Ok(GitLog { branch, commits: commits.iter().map(GitLogCommit::from).collect(), limited })
    }

    /// An opened commit in full: message, signatures, refs and its files against its first parent.
    pub fn commit_details(&self, folder: String, sha: String) -> Result<GitCommitDetails, GitBackendError> {
        let root = root_of(&folder)?;
        let details = git::commit_details(&root, &sha).map_err(|err| GitBackendError::of(&err))?;
        let opened = self.open(&root, &sha, &details)?;
        Ok(GitCommitDetails::of(&details, &opened.files, now()))
    }

    /// The opened commit's message for lines of about `line_chars` characters (core's estimate of how the
    /// view wraps), and whether it leaves anything out. Nothing for a commit that was not opened.
    pub fn compact_message(&self, folder: String, sha: String, line_chars: u32) -> Option<GitCompact> {
        let root = root_of(&folder).ok()?;
        let opened = self.kept().commits.get(&(root, sha)).cloned()?;
        Some(opened.message.compact(line_chars as usize))
    }

    /// One file's diff in a commit (against its first parent).
    pub fn commit_diff(
        &self,
        folder: String,
        sha: String,
        path: String,
        load_anyway: bool,
    ) -> Result<GitDiff, GitBackendError> {
        let root = root_of(&folder)?;
        let cached = self.kept().commits.get(&(root.clone(), sha.clone())).cloned();
        let opened = match cached {
            Some(opened) => opened,
            None => {
                let details = git::commit_details(&root, &sha).map_err(|err| GitBackendError::of(&err))?;
                self.open(&root, &sha, &details)?
            }
        };
        let file = opened
            .files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| GitBackendError::failed(format!("{path} is not in that commit")))?;
        let spec = git::commit_spec(&commit_of(&opened.sha, opened.parent.as_ref()));
        self.file_diff(&root, file, spec, load_anyway)
    }

    /// What the revision picker lists: branches (current first, then remote), tags, the latest commits and,
    /// for History, the stashes when there are some.
    pub fn revs(&self, folder: String, for_history: bool) -> Result<Vec<GitRev>, GitBackendError> {
        let root = root_of(&folder)?;
        let failed = |err: anyhow::Error| GitBackendError::of(&err);
        let rev = |name: String, kind, detail: String| GitRev { name, kind, detail };
        // `history_revs` is the branches and, last, `refs/stash` when there is one.
        let names =
            if for_history { git::history_revs(&root) } else { git::branches(&root) }.map_err(failed)?;
        let stashes = names.last().is_some_and(|name| name == STASH_REV);
        let branches = names.into_iter().filter(|name| name != STASH_REV);
        let mut revs: Vec<GitRev> =
            branches.map(|name| rev(name, GitRevKind::Branch, String::new())).collect();
        revs.extend(
            git::tags(&root).map_err(failed)?.into_iter().map(|t| rev(t, GitRevKind::Tag, String::new())),
        );
        let recent = git::log(&root, "HEAD", RECENT_COMMITS).map_err(failed)?;
        revs.extend(recent.into_iter().map(|c| rev(c.sha, GitRevKind::Commit, c.subject)));
        if stashes {
            revs.push(rev("stash".into(), GitRevKind::Stashes, String::new()));
        }
        Ok(revs)
    }
}

impl Git {
    /// The files of the commit `details` describes, kept under `asked` (what the log gave) and its full id.
    fn open(
        &self,
        root: &Path,
        asked: &str,
        details: &git::CommitDetails,
    ) -> Result<Opened, GitBackendError> {
        let parent = details.parents.first().cloned();
        let files = git::commit_files(root, &commit_of(&details.sha, parent.as_ref()))
            .map_err(|err| GitBackendError::of(&err))?;
        let opened = Opened { sha: details.sha.clone(), parent, files, message: Message::of(details) };
        let mut kept = self.kept();
        if kept.commits.len() >= 64 {
            kept.commits.clear();
        }
        kept.commits.insert((root.to_path_buf(), asked.to_string()), opened.clone());
        kept.commits.insert((root.to_path_buf(), details.sha.clone()), opened.clone());
        Ok(opened)
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn units(&self) -> SizeUnits {
        if self.binary_units.load(Ordering::Relaxed) { SizeUnits::Binary } else { SizeUnits::Decimal }
    }

    fn file_diff(
        &self,
        root: &Path,
        file: &FileChange,
        spec: DiffSpec,
        load_anyway: bool,
    ) -> Result<GitDiff, GitBackendError> {
        if let Some(sub) = &file.submodule {
            return submodule_diff(root, file, sub);
        }
        // Rows are not coloured here (Swift's lexer does that from the text), so core is told to leave the
        // whole files unread: no highlighting source, one `git show` less per side.
        let limits = DiffLimits {
            max_patch_bytes: if load_anyway { u64::MAX } else { DIFF_ASK_BYTES },
            max_highlight_lines: 0,
            max_text_bytes: 0,
        };
        let diff =
            git::diff_with_limits(root, file, &spec, &limits).map_err(|err| GitBackendError::of(&err))?;
        let units = self.units();
        Ok(if diff.too_large {
            let lines = file.added.zip(file.removed).map(|(a, r)| (a + r) as usize).filter(|n| *n > 0);
            GitDiff::TooLarge {
                bytes: diff.patch_bytes,
                title: git::too_large_title(diff.patch_bytes, units),
                detail: git::too_large_detail(lines),
            }
        } else if let Some(change) = &diff.lfs {
            GitDiff::Lfs { file: GitDiffFile::of(file, &diff, units), summary: git::lfs_summary(change, units) }
        } else if diff.binary {
            GitDiff::Binary {
                file: GitDiffFile::of(file, &diff, units),
                sizes: git::sizes_label(diff.sizes.0, diff.sizes.1, units),
            }
        } else {
            GitDiff::Text { file: GitDiffFile::of(file, &diff, units) }
        })
    }
}

fn submodule_diff(root: &Path, file: &FileChange, sub: &git::Submodule) -> Result<GitDiff, GitBackendError> {
    let log = git::submodule_log(root, &file.path, sub).map_err(|err| GitBackendError::of(&err))?;
    let (ahead, behind) = match &log.range {
        Range::Commits { ahead, behind, .. } => {
            (ahead.iter().map(GitLogCommit::from).collect(), behind.iter().map(GitLogCommit::from).collect())
        }
        _ => (Vec::new(), Vec::new()),
    };
    let note = [log.nothing_to_list(), sub.inside_note()].into_iter().flatten().collect::<Vec<_>>().join(" ");
    Ok(GitDiff::Submodule {
        label: sub.label(file.kind),
        note: (!note.is_empty()).then_some(note),
        ahead,
        behind,
    })
}

/// A commit as core's diff and file functions want it: they read only the id and the first parent.
fn commit_of(sha: &str, parent: Option<&String>) -> Commit {
    Commit {
        sha: sha.to_string(),
        short: git::short(sha).to_string(),
        parent: parent.cloned(),
        merge: false,
        author: String::new(),
        time: 0,
        subject: String::new(),
        refs: Vec::new(),
    }
}

/// Unix seconds, for the ages a block writes.
fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn root_of(folder: &str) -> Result<PathBuf, GitBackendError> {
    find_root(Path::new(folder)).ok_or(GitBackendError::NotARepository)
}
