//! What the History tab keeps of one terminal: the log of the branch shown, the commit opened with its
//! files and details, and the rows the list draws from them. No GPUI beyond the list's state: the reads are
//! `refresh`'s, the drawing `view`'s.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::git::{Commit, CommitDetails, FileChange, GitError};

use crate::style::panel_list::{CardPart, Rows};
use crate::window::repo::job::{Job, Ticket, Wanted};

/// What a history refresh is for: the folder, and the branch whose log is shown.
pub(super) type HistoryKey = (PathBuf, Option<String>);

/// What a refresh found: the repository root, and its log (or why git could not
/// list it); `Ok(None)` when the folder is not in a repository, `Err` when git
/// could not tell (it is missing, or failed).
pub(super) type Fetched = Result<Option<(PathBuf, Result<Vec<Commit>, String>)>, GitError>;

/// The files of one commit, as far as they are known.
pub(super) enum CommitFiles {
    /// Being fetched.
    Loading,
    Loaded(Rc<Vec<FileChange>>),
    /// Git failed. Shown while the commit stays open; not kept, so opening it again tries again.
    Failed(SharedString),
}

/// How many commits' files are kept; the open one and those in flight always are.
pub(super) const FILES_KEPT: usize = 64;

/// One commit's details (the whole message, the committer, the refs), as far as they are known.
#[derive(Clone)]
pub(super) enum CommitInfo {
    /// Being fetched.
    Loading,
    Loaded(Rc<CommitDetails>),
    /// Git failed. Shown while the commit stays open; not kept, so opening it again tries again.
    Failed(SharedString),
}

/// How many commits' details are kept (a copy from the menu loads them without opening the commit);
/// the open one and those in flight always are.
pub(super) const DETAILS_KEPT: usize = 64;

/// Per-session commit history.
#[derive(Default)]
pub(super) struct HistoryState {
    /// Branch whose log is shown; `None` = HEAD.
    pub(super) rev: Option<String>,
    pub(super) commits: Rc<Vec<Commit>>,
    /// The refresh in flight.
    pub(super) job: Job<HistoryKey>,
    /// What the shown answer is for. Until git has answered for the folder and branch on
    /// screen, an empty history is still being read, not "not a repository".
    pub(super) answered: Option<HistoryKey>,
    /// Why the log of the branch could not be read (it was deleted, say).
    pub(super) error: Option<SharedString>,
    /// Why git could not read the folder at all: drawn instead of the list.
    pub(super) problem: Option<GitError>,
    /// Commit expanded to show its files.
    pub(super) expanded: Option<String>,
    pub(super) files: HashMap<String, CommitFiles>,
    /// Of the commits opened (or copied from) since the log changed: read once each, never per frame.
    pub(super) details: HashMap<String, CommitInfo>,
    /// Counts what the open commit's details row shows, so the list measures that row again when
    /// they arrive or another commit opens: its description (`Details`) would be the same.
    pub(super) details_shown: u32,
    /// A commit to scroll to once the list has been laid out again: the parent the person followed.
    pub(super) reveal: Option<String>,
    /// Of a commit's files, those that are in the working tree now: the ones the icon that
    /// opens a file in the editor shows on. Read when the files arrive, so a frame never touches
    /// the disk.
    pub(super) present: HashMap<String, Rc<HashSet<String>>>,
    pub(super) root: Option<PathBuf>,
    pub(super) list: Rows<HistoryRow>,
}

/// One line of the History list: a commit, or what sits under the expanded one.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum HistoryRow {
    /// The card's head: "History", how many commits, and the branch picker.
    Head,
    Error,
    Commit(usize),
    /// Under the open commit: its message, who wrote it, its refs. The number is
    /// [`HistoryState::details_shown`].
    Details(u32),
    /// "Loading files…" / "No file changes." under the expanded commit.
    Note(&'static str),
    /// Why the expanded commit's files could not be listed.
    FilesError,
    /// Index into the expanded commit's files.
    File(usize),
    /// Bottom padding, so the last row can scroll clear of the panel edge.
    Spacer,
}

impl HistoryRow {
    /// The commits are one card; the spacer under it is outside.
    pub(super) fn part(&self) -> CardPart {
        match self {
            Self::Head => CardPart::Head,
            Self::Spacer => CardPart::Outside,
            _ => CardPart::Body,
        }
    }
}

impl HistoryState {
    /// An empty history showing branch `rev` (`None` = HEAD).
    pub(super) fn for_rev(rev: Option<String>) -> Self {
        Self { rev, ..Self::default() }
    }

    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn loading(&self) -> bool {
        self.job.is_pending()
    }

    /// Whether git has answered for `cwd` and the branch shown (whatever it said).
    pub(super) fn answered_for(&self, cwd: &std::path::Path) -> bool {
        self.answered.as_ref().is_some_and(|(dir, rev)| dir == cwd && *rev == self.rev)
    }

    /// git is missing: like the Changes tab, nothing polls it until Retry.
    pub(super) fn waits_for_retry(&self) -> bool {
        self.problem == Some(GitError::GitMissing)
    }

    /// Start a refresh for `key`. `None` when that same refresh is already
    /// running; another branch or folder supersedes it and the older answer is
    /// dropped when it lands.
    pub(super) fn begin_refresh(&mut self, key: HistoryKey) -> Option<Wanted> {
        self.job.begin(key)
    }

    /// Take a refresh's answer; `now` is what the session shows at this moment.
    /// Returns whether the list changed.
    pub(super) fn finish_refresh(
        &mut self,
        ticket: Ticket,
        now: Option<&HistoryKey>,
        result: Fetched,
    ) -> bool {
        let Some(key) = self.job.finish(ticket) else { return false };
        if now != Some(&key) {
            // The next poll asks again for what is wanted now.
            return false;
        }
        let had_problem = self.problem.take().is_some();
        let first_answer = self.answered.as_ref() != Some(&key);
        self.answered = Some(key);
        match result {
            Ok(Some((root, Ok(commits)))) => {
                self.error = None;
                self.root = Some(root);
                if *self.commits == commits && !had_problem && !first_answer {
                    return false;
                }
                self.commits = Rc::new(commits);
                true
            }
            Ok(Some((root, Err(err)))) => {
                self.root = Some(root);
                self.error = Some(err.into());
                true
            }
            Ok(None) => {
                // Still no repository: nothing to redraw (the note is the same).
                let changed = self.root.is_some() || !self.commits.is_empty() || had_problem || first_answer;
                self.root = None;
                self.commits = Rc::default();
                changed
            }
            Err(problem) => {
                (self.root, self.commits, self.error) = (None, Rc::default(), None);
                self.problem = Some(problem);
                true
            }
        }
    }

    /// Expand `sha`, or collapse it when it is the open one. Returns the commit
    /// whose files have to be fetched (once: one already on its way is awaited).
    pub(super) fn toggle(&mut self, sha: &str) -> Option<(PathBuf, Commit)> {
        let open = self.expanded.as_deref() == Some(sha);
        // A failure is not kept: closing or opening again is a retry.
        if matches!(self.files.get(sha), Some(CommitFiles::Failed(_))) {
            self.files.remove(sha);
        }
        self.details_shown = self.details_shown.wrapping_add(1);
        if open {
            self.expanded = None;
            return None;
        }
        self.expanded = Some(sha.to_string());
        if self.files.contains_key(sha) {
            return None;
        }
        let root = self.root.clone()?;
        let commit = self.commits.iter().find(|c| c.sha == sha)?.clone();
        self.files.insert(sha.to_string(), CommitFiles::Loading);
        Some((root, commit))
    }

    /// Open `commit` of the repository at `root` (the command palette's commit, which may not be
    /// listed yet): on the branch shown when `on_branch` says the commit is on it, else on HEAD.
    /// Returns the commit whose files have to be fetched.
    pub(super) fn open(
        &mut self,
        root: PathBuf,
        commit: Commit,
        on_branch: bool,
    ) -> Option<(PathBuf, Commit)> {
        if !on_branch {
            self.rev = None;
        }
        self.expanded = Some(commit.sha.clone());
        self.details_shown = self.details_shown.wrapping_add(1);
        if matches!(self.files.get(&commit.sha), Some(CommitFiles::Failed(_))) {
            self.files.remove(&commit.sha);
        }
        if self.files.contains_key(&commit.sha) {
            return None;
        }
        self.files.insert(commit.sha.clone(), CommitFiles::Loading);
        Some((root, commit))
    }

    /// The files of `sha` arrived (or git failed to list them).
    pub(super) fn files_loaded(&mut self, sha: &str, result: Result<Vec<FileChange>, String>) {
        match result {
            Ok(files) => {
                self.files.insert(sha.to_string(), CommitFiles::Loaded(Rc::new(files)));
                if self.files.len() > FILES_KEPT {
                    let expanded = self.expanded.clone();
                    self.files.retain(|key, files| {
                        Some(key.as_str()) == expanded.as_deref()
                            || key == sha
                            || matches!(files, CommitFiles::Loading)
                    });
                    self.present.retain(|key, _| self.files.contains_key(key));
                }
            }
            // Only for a commit that is still waiting for it, and still open.
            Err(err) if matches!(self.files.get(sha), Some(CommitFiles::Loading)) => {
                if self.expanded.as_deref() == Some(sha) {
                    self.files.insert(sha.to_string(), CommitFiles::Failed(err.into()));
                } else {
                    self.files.remove(sha);
                }
            }
            Err(_) => {}
        }
    }

    /// Which of the files of `sha` are in the working tree now.
    pub(super) fn files_present(&mut self, sha: &str, present: HashSet<String>) {
        if self.files.contains_key(sha) {
            self.present.insert(sha.to_string(), Rc::new(present));
        }
    }

    /// The files of `sha`, once loaded.
    pub(super) fn loaded(&self, sha: &str) -> Option<&Rc<Vec<FileChange>>> {
        match self.files.get(sha) {
            Some(CommitFiles::Loaded(files)) => Some(files),
            _ => None,
        }
    }

    /// `sha`'s details are wanted: whether they have to be fetched (they are not here, or the last try
    /// failed; one already on its way is awaited).
    pub(super) fn want_details(&mut self, sha: &str) -> bool {
        if matches!(self.details.get(sha), Some(CommitInfo::Loading | CommitInfo::Loaded(_))) {
            return false;
        }
        self.details.insert(sha.to_string(), CommitInfo::Loading);
        self.shown_changed(sha);
        true
    }

    /// The details of `sha` arrived (or git failed to read them).
    pub(super) fn details_loaded(&mut self, sha: &str, result: Result<CommitDetails, String>) {
        match result {
            Ok(details) => {
                self.details.insert(sha.to_string(), CommitInfo::Loaded(Rc::new(details)));
                if self.details.len() > DETAILS_KEPT {
                    let expanded = self.expanded.clone();
                    self.details.retain(|key, info| {
                        Some(key.as_str()) == expanded.as_deref()
                            || key == sha
                            || matches!(info, CommitInfo::Loading)
                    });
                }
            }
            // Only for a commit that is still waiting for it, and still open.
            Err(err) if matches!(self.details.get(sha), Some(CommitInfo::Loading)) => {
                if self.expanded.as_deref() == Some(sha) {
                    self.details.insert(sha.to_string(), CommitInfo::Failed(err.into()));
                } else {
                    self.details.remove(sha);
                }
            }
            Err(_) => {}
        }
        self.shown_changed(sha);
    }

    /// The details of `sha`, once loaded.
    pub(super) fn details_of(&self, sha: &str) -> Option<&Rc<CommitDetails>> {
        match self.details.get(sha) {
            Some(CommitInfo::Loaded(details)) => Some(details),
            _ => None,
        }
    }

    /// The log changed (a commit landed, HEAD moved): the refs of what was read are out of date. All
    /// but the open commit's are dropped; its are kept on screen while they are read again, and its
    /// sha is returned for that.
    pub(super) fn details_stale(&mut self) -> Option<String> {
        let expanded = self.expanded.clone();
        self.details.retain(|key, _| Some(key) == expanded.as_ref());
        expanded.filter(|sha| matches!(self.details.get(sha), Some(CommitInfo::Loaded(_))))
    }

    /// Row `Details` changes with the open commit's details (and only with those).
    fn shown_changed(&mut self, sha: &str) {
        if self.expanded.as_deref() == Some(sha) {
            self.details_shown = self.details_shown.wrapping_add(1);
        }
    }

    pub(super) fn rows(&self) -> Vec<HistoryRow> {
        let mut rows = Vec::with_capacity(self.commits.len() + 3);
        rows.push(HistoryRow::Head);
        if self.error.is_some() {
            rows.push(HistoryRow::Error);
        }
        for (ix, commit) in self.commits.iter().enumerate() {
            rows.push(HistoryRow::Commit(ix));
            if self.expanded.as_deref() != Some(commit.sha.as_str()) {
                continue;
            }
            rows.push(HistoryRow::Details(self.details_shown));
            match self.files.get(&commit.sha) {
                None | Some(CommitFiles::Loading) => rows.push(HistoryRow::Note("Loading files…")),
                Some(CommitFiles::Failed(_)) => rows.push(HistoryRow::FilesError),
                Some(CommitFiles::Loaded(files)) if files.is_empty() => {
                    rows.push(HistoryRow::Note("No file changes."))
                }
                Some(CommitFiles::Loaded(files)) => rows.extend((0..files.len()).map(HistoryRow::File)),
            }
        }
        rows.push(HistoryRow::Spacer);
        rows
    }
}
