//! What git last said of a terminal's folder: the repository (or why there is none), which folder it
//! is about, the read in flight, and what the polling needs to decide what to do next. The app's
//! `window/repo/state.rs`, with times as `Millis` instead of ticks (a terminal's own last read is
//! what spaces the reads, not the window's tick), and with no "compare with" branch: that is the
//! Changes panel's question, not a row's.

use std::path::{Path, PathBuf};

use crate::git::attribution::Place;
use crate::git::discover::{Absent, Sighting};
use crate::git::{GitError, RepoStatus};

use super::super::facts::Millis;
use super::job::{Job, Ticket, Wanted};

/// What a row needs of a repository's status: the files themselves stay with the read that found
/// them (`attribution` keeps a picture of them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    /// The repository's root, as git names it.
    pub root: PathBuf,
    pub branch: Option<String>,
    /// The working tree has uncommitted changes.
    pub dirty: bool,
    /// Where the branch stands against its upstream; 0 and 0 without one.
    pub ahead: u32,
    pub behind: u32,
    /// How many files are changed in the repository.
    pub files: usize,
}

impl From<&RepoStatus> for Repo {
    fn from(status: &RepoStatus) -> Self {
        let upstream = status.upstream.as_ref();
        Self {
            root: status.root.clone(),
            branch: status.branch.clone(),
            dirty: status.dirty,
            ahead: upstream.map_or(0, |upstream| upstream.ahead),
            behind: upstream.map_or(0, |upstream| upstream.behind),
            files: status.files.len(),
        }
    }
}

/// Git state kept per terminal.
#[derive(Debug, Default)]
pub(super) struct GitState {
    pub(super) repo: Option<Repo>,
    /// The folder the repository was read for.
    cwd: Option<PathBuf>,
    /// The repository git last named for this terminal. Unlike `repo` it is not blanked while a read
    /// is on its way, so what is kept by repository (`attribution`, the watches) does not let go of the
    /// terminal for a moment.
    root: Option<PathBuf>,
    /// Why the last read failed. When git failed, the repository stays the last good one (while the
    /// folder is the same), so a hiccup does not blank the list; a missing git clears it.
    error: Option<GitError>,
    job: Job<PathBuf>,
    /// Set while git's last answer was "not a repository": git is then not polled, a cheap look for a
    /// `.git` is (see `git::discover`).
    absent: Option<Absent>,
    /// A look for a `.git` is running.
    probing: bool,
    sight: Sight,
    /// A read is owed as soon as one can start: the terminal was seen another way, or its repository
    /// changed, since the last one. Cleared when a read starts.
    owed: bool,
    /// A look confirmed that a `.git` appeared where git said there was none: ask git, whatever the
    /// absence says.
    ask: bool,
    read_at: Option<Millis>,
    look_at: Option<Millis>,
    /// A look to confirm a `.git` that showed up, due at this moment.
    look_again: Option<Millis>,
}

/// How a terminal is seen, which sets how often its repository is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sight {
    /// On screen in the window in front: what the person looks at.
    #[default]
    Front,
    /// Seen, but not looked at closely: behind another terminal, or in a window that is not in front.
    Behind,
    /// Its window cannot be seen and nothing in its repository is working: what git would say changes
    /// nothing anyone sees, so it is read only now and then, and at once when seen again.
    Unseen,
}

/// The window's tick the app's rates were set in: 0.4 s.
const TICK: Millis = 400;

/// How a repository is read when no watch follows it: in front every 3 ticks (1.2 s), behind every
/// 20th (8 s), unseen every 25th (10 s); and the look for a `.git` where there is no repository every
/// tick in front, every 5th behind, every 25th unseen.
fn rates(sight: Sight) -> (Millis, Millis) {
    let (look, read) = match sight {
        Sight::Front => (1, 3),
        Sight::Behind => (5, 20),
        Sight::Unseen => (25, 25),
    };
    (look * TICK, read * TICK)
}

/// A repository followed through its files is read every 75th tick (30 s) in any sight, as the
/// backstop for a missed event (a volume that sends none, a dropped batch that went unflagged).
pub const WATCHED_READ: Millis = 75 * TICK;

/// How long after a `.git` first shows up the second look is taken (see `discover::Absent`).
pub const PROBE_CONFIRM: Millis = 150;

/// What is due for one terminal.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Poll {
    /// Ask git.
    Refresh,
    /// Look for a `.git` (a `stat` walk), not at git.
    Probe,
    Idle,
}

/// What became of a read's answer.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refreshed {
    /// Not the answer wanted any more (a newer request superseded it, or the terminal moved to another
    /// folder while git ran): dropped.
    Stale,
    Applied {
        changed: bool,
    },
}

impl GitState {
    /// What is due now for a terminal in `cwd` whose repository is followed through its files
    /// (`watched`). A folder git has not answered for is asked at once, and so is one owed a read. A
    /// folder in no repository is not polled at all: it is looked at for a `.git`. Only one read runs
    /// at a time (`begin_refresh`), so a slow repository is read back to back at most, never piled up.
    pub(super) fn poll(&self, cwd: Option<&Path>, now: Millis, watched: bool) -> Poll {
        let Some(cwd) = cwd else { return Poll::Idle };
        if self.waits_for_retry() {
            return Poll::Idle;
        }
        let due = |since: Option<Millis>, every: Millis| since.is_none_or(|at| now >= at + every);
        let (look, read) = rates(self.sight);
        if self.cwd.as_deref() != Some(cwd) || self.ask {
            return if self.job.is_pending() { Poll::Idle } else { Poll::Refresh };
        }
        if self.is_absent_for(cwd) {
            let confirming = self.look_again.is_some_and(|at| now >= at);
            return if !self.probing && (confirming || (self.look_again.is_none() && due(self.look_at, look)))
            {
                Poll::Probe
            } else {
                Poll::Idle
            };
        }
        let read = if watched { WATCHED_READ } else { read };
        if !self.job.is_pending() && (self.owed || due(self.read_at, read)) {
            Poll::Refresh
        } else {
            Poll::Idle
        }
    }

    /// When [`poll`](Self::poll) will have something to do, `None` when only an event can make it.
    pub(super) fn next_due(&self, cwd: Option<&Path>, now: Millis, watched: bool) -> Option<Millis> {
        let cwd = cwd?;
        if self.waits_for_retry() {
            return None;
        }
        let (look, read) = rates(self.sight);
        if self.cwd.as_deref() != Some(cwd) || self.ask {
            return (!self.job.is_pending()).then_some(now);
        }
        if self.is_absent_for(cwd) {
            return match (self.probing, self.look_again) {
                (true, _) => None,
                (false, Some(at)) => Some(at),
                (false, None) => Some(self.look_at.map_or(now, |at| at + look)),
            };
        }
        let read = if watched { WATCHED_READ } else { read };
        let at = if self.owed { now } else { self.read_at.map_or(now, |at| at + read) };
        (!self.job.is_pending()).then_some(at)
    }

    /// How the terminal is seen now. A change owes one read, taken at once: coming into sight, what
    /// changed meanwhile shows without waiting for the period; going out of sight, the repository is
    /// pictured once more, so edits made by whoever just stopped working are not merged with the next
    /// worker's (`git::attribution`).
    pub(super) fn note_sight(&mut self, sight: Sight) {
        if std::mem::replace(&mut self.sight, sight) != sight {
            self.owed = true;
        }
    }

    /// Something in the terminal's repository changed (a watch said so). Owes a read at once unless
    /// the terminal is unseen: nobody sees what git would say, and coming into sight owes a read
    /// anyway ([`note_sight`](Self::note_sight)). True when the read is wanted now.
    pub(super) fn watch_hit(&mut self) -> bool {
        if self.sight == Sight::Unseen {
            return false;
        }
        self.owed = true;
        true
    }

    /// Where the terminal is, for what is kept per repository (`attribution`): the repository git
    /// named, or none, once git has answered for `cwd`; until then not known, so a terminal that has
    /// changed folder keeps what it had.
    pub(super) fn place(&self, cwd: Option<&Path>) -> Place<'_> {
        if !cwd.is_some_and(|cwd| self.answered_for(cwd)) {
            return Place::Unknown;
        }
        self.root.as_deref().map_or(Place::Outside, Place::In)
    }

    /// The repository git last named, kept while a new folder is being asked about: what the watches
    /// are claimed by, so a `cd` does not let go of a repository and start it again.
    pub(super) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub(super) fn error(&self) -> Option<&GitError> {
        self.error.as_ref()
    }

    /// git is missing: nothing polls it again until Retry, since on a Mac without the Command Line
    /// Tools every run of the stub asks to install them.
    pub(super) fn waits_for_retry(&self) -> bool {
        self.error == Some(GitError::GitMissing)
    }

    /// Git said "not a repository" for `cwd` and nothing has changed since: no need to poll it.
    pub(super) fn is_absent_for(&self, cwd: &Path) -> bool {
        self.error.is_none() && self.absent.as_ref().is_some_and(|absent| absent.is_for(cwd))
    }

    /// The answer kept is for `cwd`, whatever it said.
    pub(super) fn answered_for(&self, cwd: &Path) -> bool {
        self.cwd.as_deref() == Some(cwd)
    }

    pub(super) fn begin_probe(&mut self, cwd: &Path, now: Millis) -> bool {
        if self.probing || !self.is_absent_for(cwd) {
            return false;
        }
        self.probing = true;
        self.look_at = Some(now);
        self.look_again = None;
        true
    }

    /// A look came back with `found`: what to do about it.
    pub(super) fn finish_probe(&mut self, cwd: &Path, found: Option<PathBuf>, now: Millis) -> Sighting {
        self.probing = false;
        let sighting = match &mut self.absent {
            Some(absent) if absent.is_for(cwd) && self.error.is_none() => absent.observe(found),
            _ => Sighting::Unchanged,
        };
        match sighting {
            Sighting::LookAgain => self.look_again = Some(now + PROBE_CONFIRM),
            Sighting::AskGit => self.ask = true,
            Sighting::Unchanged => {}
        }
        sighting
    }

    /// Start a read for `key`. `None` when that same read is already running; a different one (another
    /// folder) supersedes it, and the older answer is dropped when it lands.
    pub(super) fn begin_refresh(&mut self, key: PathBuf, now: Millis) -> Option<Wanted> {
        let wanted = self.job.begin(key)?;
        (self.owed, self.ask) = (false, false);
        self.read_at = Some(now);
        Some(wanted)
    }

    /// Take a read's answer. `now` is the folder the terminal is in at this moment.
    pub(super) fn finish_refresh(
        &mut self,
        ticket: Ticket,
        now: Option<&Path>,
        result: anyhow::Result<Option<RepoStatus>>,
    ) -> Refreshed {
        let Some(key) = self.job.finish(ticket) else { return Refreshed::Stale };
        if now != Some(key.as_path()) {
            // The next poll asks again for what is wanted now.
            return Refreshed::Stale;
        }
        let same_folder = self.cwd.as_ref() == Some(&key);
        self.cwd = Some(key);
        let changed = match result.map_err(|err| (GitError::of(&err), err)) {
            Ok(status) => {
                let repo = status.as_ref().map(Repo::from);
                // The first answer for a folder also replaces "nothing known".
                let changed = self.repo != repo || self.error.is_some() || !same_folder;
                self.error = None;
                self.note_absent(repo.is_none());
                self.note_root(repo.as_ref().map(|repo| repo.root.as_path()));
                self.repo = repo;
                changed
            }
            // The folder stopped being a repository after git found one: as if it never was.
            Err((GitError::NotARepo, _)) => {
                let changed = self.repo.is_some() || self.error.is_some() || !same_folder;
                (self.repo, self.error) = (None, None);
                self.note_absent(true);
                self.note_root(None);
                changed
            }
            Err((error, err)) => {
                self.absent = None;
                if self.error.as_ref() != Some(&error) {
                    // In full, once.
                    tracing::warn!(cwd = ?self.cwd, "git status failed: {err:#}");
                }
                // Another folder's answer under this folder's name would mislead, and without git the
                // answer is not coming back by itself.
                let keep = same_folder && error != GitError::GitMissing;
                let changed = self.error.as_ref() != Some(&error) || (!keep && self.repo.is_some());
                if !keep {
                    self.repo = None;
                    self.note_root(None);
                }
                self.error = Some(error);
                changed
            }
        };
        Refreshed::Applied { changed }
    }

    /// Git is tried again (Retry, or the window coming to the front).
    pub(super) fn retry(&mut self) {
        if self.waits_for_retry() {
            self.error = None;
            self.owed = true;
        }
    }

    fn note_root(&mut self, root: Option<&Path>) {
        if self.root.as_deref() != root {
            self.root = root.map(Path::to_path_buf);
        }
    }

    /// Remember (or forget) that git said the folder is not in a repository. An answer that repeats
    /// keeps what the looks learned, so a `.git` git rejects is not asked about again.
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
