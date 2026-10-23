//! What git last said of a terminal's folder: the status (or why there is none), which folder it is
//! about, the refresh in flight, and what the polling needs to decide what to do next.
//!
//! The answer is the window's own fact about a terminal: the sidebar's rows, the title row, the status
//! bar and the palette read it, so it is not a feature's. Features are lent it through the facts.

use std::path::PathBuf;

use wormhole_core::git::discover::{Absent, Sighting};
use wormhole_core::git::{GitError, RepoStatus};

use crate::window::repo::job::{Job, Ticket, Wanted};

/// What a status refresh is for: the folder, and the branch compared with.
pub(crate) type StatusKey = (PathBuf, Option<String>);

/// Git state kept per session.
#[derive(Default)]
pub(crate) struct GitState {
    pub status: Option<RepoStatus>,
    /// The cwd the status was computed for.
    pub cwd: Option<PathBuf>,
    /// Why the last refresh failed. When git failed, the status stays the last
    /// good one (while the folder is the same), so a hiccup does not blank the
    /// list; a missing git clears it.
    error: Option<GitError>,
    /// The refresh in flight.
    job: Job<StatusKey>,
    /// Set while git's last answer was "not a repository": git is then not polled, a cheap look
    /// for a `.git` is (see `wormhole_core::git::discover`).
    absent: Option<Absent>,
    /// A look for a `.git` is running off the UI thread.
    probing: bool,
    /// The branch the status is read against (`None`: HEAD): the Changes tab's "compare with". The
    /// status is a different list for each, so it is part of what is asked.
    pub compare: Option<String>,
}

/// What became of a refresh's answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refreshed {
    /// Not the answer wanted any more (a newer request superseded it, or the
    /// session moved to another folder or branch while git ran): dropped.
    Stale,
    Applied {
        changed: bool,
    },
}

/// What the tick does about one session's git status.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GitPoll {
    /// Ask git.
    Refresh,
    /// Look for a `.git` (a `stat` walk), not at git.
    Probe,
    Idle,
}

impl GitState {
    /// What to do on tick number `tick` (already offset by the session's place, so sessions
    /// don't all poll on the same tick) for a terminal in `cwd`. A folder git has not answered
    /// for is asked at once. A repository is polled every 5th tick (2 s) when in front, every
    /// 20th (8 s) behind. A folder in no repository is not polled at all: it is looked at for a
    /// `.git` every tick in front (0.4 s), every 5th behind.
    pub(crate) fn poll(&self, cwd: Option<&std::path::Path>, tick: u64, in_front: bool) -> GitPoll {
        if self.cwd.as_deref() != cwd {
            return GitPoll::Refresh;
        }
        if cwd.is_some_and(|cwd| self.is_absent_for(cwd)) {
            return if in_front || tick.is_multiple_of(5) { GitPoll::Probe } else { GitPoll::Idle };
        }
        if tick.is_multiple_of(if in_front { 5 } else { 20 }) { GitPoll::Refresh } else { GitPoll::Idle }
    }

    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn loading(&self) -> bool {
        self.job.is_pending()
    }

    /// Why the last refresh failed, if it did.
    pub(crate) fn error(&self) -> Option<&GitError> {
        self.error.as_ref()
    }

    /// git is missing: nothing polls it again until Retry (or the window coming to the
    /// front), since on a Mac without the Command Line Tools every run of the stub asks
    /// to install them.
    pub(crate) fn waits_for_retry(&self) -> bool {
        self.error == Some(GitError::GitMissing)
    }

    /// Git said "not a repository" for `cwd` and nothing has changed since: no need to poll it.
    pub(crate) fn is_absent_for(&self, cwd: &std::path::Path) -> bool {
        self.error.is_none() && self.absent.as_ref().is_some_and(|absent| absent.is_for(cwd))
    }

    /// The answer shown is for `cwd`, whatever it said: the panel need not say "Reading".
    pub(crate) fn answered_for(&self, cwd: &std::path::Path) -> bool {
        self.cwd.as_deref() == Some(cwd)
    }

    /// Start a look for a `.git` near `cwd`. False when one already runs or nothing is absent.
    pub(crate) fn begin_probe(&mut self, cwd: &std::path::Path) -> bool {
        if self.probing || !self.is_absent_for(cwd) {
            return false;
        }
        self.probing = true;
        true
    }

    /// A look came back with `found`: what to do about it.
    pub(crate) fn finish_probe(&mut self, cwd: &std::path::Path, found: Option<PathBuf>) -> Sighting {
        self.probing = false;
        match &mut self.absent {
            Some(absent) if absent.is_for(cwd) && self.error.is_none() => absent.observe(found),
            _ => Sighting::Unchanged,
        }
    }

    /// Start a refresh for `key`. `None` when that same refresh is already
    /// running; a different one (another branch compared, another folder)
    /// supersedes it, and the older answer is dropped when it lands.
    pub(crate) fn begin_refresh(&mut self, key: StatusKey) -> Option<Wanted> {
        self.job.begin(key)
    }

    /// Take a refresh's answer. `now` is what the session shows at this moment.
    pub(crate) fn finish_refresh(
        &mut self,
        ticket: Ticket,
        now: Option<&StatusKey>,
        result: anyhow::Result<Option<RepoStatus>>,
    ) -> Refreshed {
        let Some(key) = self.job.finish(ticket) else { return Refreshed::Stale };
        if now != Some(&key) {
            // The next poll asks again for what is wanted now.
            return Refreshed::Stale;
        }
        let same_folder = self.cwd.as_ref() == Some(&key.0);
        self.cwd = Some(key.0);
        let changed = match result.map_err(|err| (GitError::of(&err), err)) {
            Ok(status) => {
                // The first answer for a folder also replaces "Reading git status…".
                let changed = self.status != status || self.error.is_some() || !same_folder;
                self.error = None;
                self.note_absent(status.is_none());
                self.status = status;
                changed
            }
            // The folder stopped being a repository after git found one: as if it never was.
            Err((GitError::NotARepo, _)) => {
                let changed = self.status.is_some() || self.error.is_some() || !same_folder;
                (self.status, self.error) = (None, None);
                self.note_absent(true);
                changed
            }
            Err((error, err)) => {
                self.absent = None;
                if self.error.as_ref() != Some(&error) {
                    // In full, once: the panel shows git's first line.
                    tracing::warn!(cwd = ?self.cwd, "git status failed: {err:#}");
                }
                // Another folder's list under this folder's name would mislead, and without git
                // the list is not coming back by itself.
                let keep = same_folder && error != GitError::GitMissing;
                let changed = self.error.as_ref() != Some(&error) || (!keep && self.status.is_some());
                if !keep {
                    self.status = None;
                }
                self.error = Some(error);
                changed
            }
        };
        Refreshed::Applied { changed }
    }
}

impl GitState {
    /// Remember (or forget) that git said the folder is not in a repository. An answer that
    /// repeats keeps what the looks learned, so a `.git` git rejects is not asked about again.
    fn note_absent(&mut self, absent: bool) {
        let Some(cwd) = self.cwd.clone().filter(|_| absent) else {
            self.absent = None;
            return;
        };
        if !self.absent.as_ref().is_some_and(|known| known.is_for(&cwd)) {
            self.absent = Some(Absent::new(cwd));
        }
    }
}
