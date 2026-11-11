//! What git says of each terminal's folder, as the rows show it: the branch, whether the tree is
//! dirty, how far it is from its upstream, the repository it is in and how many files the terminal
//! itself changed. The rules are the app's `window/repo/` (state, jobs, polling) and
//! `git::attribution`, as a sans-IO model: no git process, no file event, no thread. The shell
//! (`apple/ffi`'s `GitJobs`) does the I/O the model asks for as [`GitJob`]s and hands the answers back,
//! and follows the repositories' files, calling [`GitFeed::repo_changed`] when something in one
//! changed: git is read when something happened, and the periods here are only the backstop.
//!
//! `job` (stale answers are dropped by ticket), `state` (one terminal's answer and polling) and this
//! file (all of them, and the attribution that tells whose changes are whose).

mod job;
mod sights;
mod state;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::config::{PlainChanges, SharedChanges};
use crate::git::attribution::{Attribution, Kind, Snapshot};
use crate::storage::store::ClaimRecord;
use crate::view::sidebar::RowState;
use crate::view::sidebar::command::ends_by_itself;

pub use job::{Ticket, Wanted};
pub use state::{PROBE_CONFIRM, Repo, Sight, WATCHED_READ};

use super::facts::{Millis, TerminalFacts};
use state::{GitState, Poll, Refreshed};

/// The setting that decides what a row counts (Settings → Terminals & sidebar → Changed files).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeRules {
    /// Which programs in front make a plain terminal count as working.
    pub plain: PlainChanges,
    /// What becomes of a file more than one terminal changed.
    pub shared: SharedChanges,
}

/// What the model asks the shell to do. The answer comes back through `finish_read` or
/// `finish_probe`, with the ticket or the folder it was asked for.
#[derive(Clone, Debug)]
pub enum GitJob {
    /// Read the repository `cwd` is in (`git::status_summary`), and take a picture of its files
    /// (`attribution::Snapshot::take`) in the same hop, dated by when the disk was read.
    Read { id: i64, cwd: PathBuf, wanted: Wanted },
    /// Look for a `.git` near `cwd` (`git::discover::probe`): a `stat` walk, not git.
    Probe { id: i64, cwd: PathBuf },
}

/// What git says of a terminal, in the terms of its facts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitFacts {
    pub repo: Option<PathBuf>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    /// Files changed in the repository, and the ones this terminal changed.
    pub files: Option<usize>,
    pub changes: Option<usize>,
}

impl GitFacts {
    /// Put them in the terminal's facts. Whether anything changed.
    pub fn apply(&self, facts: &mut TerminalFacts) {
        facts.repo.clone_from(&self.repo);
        facts.branch.clone_from(&self.branch);
        facts.dirty = self.dirty;
        facts.ahead = self.ahead;
        facts.behind = self.behind;
        facts.repo_files = self.files;
        facts.changes = self.changes;
    }
}

/// Every terminal's git state in a window.
#[derive(Debug)]
pub struct GitFeed {
    terminals: HashMap<i64, State>,
    attribution: Attribution,
    rules: ChangeRules,
    /// Repositories a running watch follows: their reads are only the backstop.
    watched: HashSet<PathBuf>,
    /// The window can be seen at all.
    seen: bool,
}

#[derive(Debug, Default)]
struct State {
    git: GitState,
    /// Where the terminal is now, as the shell last said.
    cwd: Option<PathBuf>,
}

impl Default for GitFeed {
    fn default() -> Self {
        Self::new(ChangeRules::default())
    }
}

impl GitFeed {
    pub fn new(rules: ChangeRules) -> Self {
        Self {
            terminals: HashMap::new(),
            attribution: Attribution::default(),
            rules,
            watched: HashSet::new(),
            seen: true,
        }
    }

    /// The rules changed in Settings: what a plain terminal's work is, and what a shared file counts as.
    pub fn set_rules(&mut self, rules: ChangeRules) {
        self.rules = rules;
    }

    /// Terminal `id` is in `cwd` (or nowhere known).
    pub fn folder(&mut self, id: i64, cwd: Option<&Path>) {
        let state = self.terminals.entry(id).or_default();
        if state.cwd.as_deref() != cwd {
            state.cwd = cwd.map(Path::to_path_buf);
        }
    }

    /// How terminal `id` is seen.
    pub fn sight(&mut self, id: i64, sight: Sight) {
        self.terminals.entry(id).or_default().git.note_sight(sight);
    }

    /// Terminal `id` closed: its files are not counted any more. Whether a count changed.
    pub fn forget(&mut self, id: i64) -> bool {
        self.terminals.remove(&id);
        self.attribution.forget(id)
    }

    /// The files each of `ids` changed, one row per file, for keeping them with the window (`Store::save_claims`).
    pub fn claim_rows(&self, ids: &[i64]) -> Vec<ClaimRecord> {
        ids.iter()
            .filter_map(|&id| self.attribution.claims_of(id).map(|(root, paths)| (id, root, paths)))
            .flat_map(|(session_id, root, paths)| {
                let root = root.to_string_lossy().into_owned();
                let mut paths: Vec<_> = paths.iter().cloned().collect();
                paths.sort();
                paths.into_iter().map(move |path| ClaimRecord { session_id, root: root.clone(), path })
            })
            .collect()
    }

    /// Put back what was kept (a window restored): each terminal is taken to be in the repository its files are
    /// in until its folder says otherwise. Whether any terminal has files again.
    pub fn restore_claims(&mut self, claims: Vec<ClaimRecord>) -> bool {
        let mut grouped: std::collections::BTreeMap<(i64, String), Vec<String>> = Default::default();
        for claim in claims {
            grouped.entry((claim.session_id, claim.root)).or_default().push(claim.path);
        }
        let mut restored = false;
        for ((id, root), paths) in grouped {
            if self.attribution.restore(id, Path::new(&root), paths) {
                // Known to the feed from now on, so its facts carry the count before its first read.
                self.terminals.entry(id).or_default();
                restored = true;
            }
        }
        restored
    }

    /// git was missing and may be there now (Retry, the window coming to the front).
    pub fn retry(&mut self) {
        self.terminals.values_mut().for_each(|state| state.git.retry());
    }

    /// Tell the model, from the terminal's facts, whether it is working: the files that change
    /// meanwhile are its. Returns whether a count changed (a terminal moving to another repository).
    pub fn track(&mut self, id: i64, facts: &TerminalFacts, now: Instant) -> bool {
        let (kind, working) = self.working(facts);
        let Some(state) = self.terminals.get(&id) else { return false };
        let place = state.git.place(state.cwd.as_deref());
        self.attribution.track(id, place, kind, working, now)
    }

    /// A terminal that tells its status is working while the row says so. A plain one is working by
    /// the setting: by default while a command that ends by itself runs in front, since a pager, an
    /// editor, `ssh` or a dev server stays open and would take every edit made outside the app.
    #[cfg(test)]
    pub(super) fn working_of(&self, facts: &TerminalFacts) -> bool {
        self.working(facts).1
    }

    fn working(&self, facts: &TerminalFacts) -> (Kind, bool) {
        if facts.told() {
            return (Kind::Told, facts.row_state() == RowState::Working);
        }
        let in_front = facts.command.is_some();
        let ends = facts.command.as_deref().is_some_and(ends_by_itself);
        let working = match self.rules.plain {
            PlainChanges::None => false,
            PlainChanges::Any => in_front,
            PlainChanges::Finishing => ends && in_front,
        };
        (Kind::Plain, working)
    }

    /// Something in the repository at `root` changed (its watch said so, after the debounce): every
    /// terminal in it that is not unseen is owed a read.
    pub fn repo_changed(&mut self, root: &Path) {
        for state in self.terminals.values_mut() {
            if !state.git.waits_for_retry()
                && state.git.place(state.cwd.as_deref()) == crate::git::attribution::Place::In(root)
            {
                state.git.watch_hit();
            }
        }
    }

    /// The repositories now followed by a running watch (`true`) or not (`false`).
    pub fn set_watched(&mut self, root: &Path, live: bool) {
        if live {
            self.watched.insert(root.to_path_buf());
        } else {
            self.watched.remove(root);
        }
    }

    /// The repositories the terminals are in, for the watches. By the last one git named, not by
    /// where the terminal is asked about now: a `cd` must not let go of a repository and start it again.
    pub fn roots(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = Vec::new();
        for root in self.terminals.values().filter_map(|state| state.git.root()) {
            if !roots.iter().any(|known| known == root) {
                roots.push(root.to_path_buf());
            }
        }
        roots.sort();
        roots
    }

    /// What is due at `now`: the reads and looks to start. Each starts once; its answer ends it.
    pub fn poll(&mut self, now: Millis) -> Vec<GitJob> {
        let mut jobs = Vec::new();
        for (&id, state) in &mut self.terminals {
            let watched = matches!(state.git.place(state.cwd.as_deref()), crate::git::attribution::Place::In(root) if self.watched.contains(root));
            let Some(cwd) = state.cwd.clone() else { continue };
            match state.git.poll(Some(&cwd), now, watched) {
                Poll::Refresh => {
                    if let Some(wanted) = state.git.begin_refresh(cwd.clone(), now) {
                        jobs.push(GitJob::Read { id, cwd, wanted });
                    }
                }
                Poll::Probe => {
                    if state.git.begin_probe(&cwd, now) {
                        jobs.push(GitJob::Probe { id, cwd });
                    }
                }
                Poll::Idle => {}
            }
        }
        jobs
    }

    /// When [`poll`](Self::poll) next has something to do; `None` when only an event can say.
    pub fn next_due(&self, now: Millis) -> Option<Millis> {
        self.terminals
            .values()
            .filter_map(|state| {
                let place = state.git.place(state.cwd.as_deref());
                let watched =
                    matches!(place, crate::git::attribution::Place::In(root) if self.watched.contains(root));
                state.git.next_due(state.cwd.as_deref(), now, watched)
            })
            .min()
    }

    /// A read came back. `snapshot` is its picture of the disk. Returns whether what the terminal
    /// shows changed (and so may be that of others: the picture moves counts across the repository).
    pub fn finish_read(
        &mut self,
        id: i64,
        wanted: &Wanted,
        result: anyhow::Result<Option<crate::git::RepoStatus>>,
        snapshot: Option<Snapshot>,
    ) -> bool {
        let Some(state) = self.terminals.get_mut(&id) else { return false };
        let now = state.cwd.clone();
        match state.git.finish_refresh(wanted.ticket(), now.as_deref(), result) {
            Refreshed::Stale => false,
            Refreshed::Applied { changed } => {
                // Only an answer that is still wanted: a stale one is a picture of a read that was
                // superseded, and dropped like its status.
                let moved = snapshot.is_some_and(|snapshot| self.attribution.observe(snapshot));
                changed | moved
            }
        }
    }

    /// A look came back with `found`: the confirming look is due `PROBE_CONFIRM` after, or git is
    /// asked (the next `poll` says so).
    pub fn finish_probe(&mut self, id: i64, cwd: &Path, found: Option<PathBuf>, now: Millis) {
        if let Some(state) = self.terminals.get_mut(&id) {
            state.git.finish_probe(cwd, found, now);
        }
    }

    /// What git says of terminal `id`, in the terms of its facts.
    pub fn facts_of(&self, id: i64) -> GitFacts {
        let Some(state) = self.terminals.get(&id) else { return GitFacts::default() };
        let repo = state.git.repo.as_ref();
        GitFacts {
            repo: repo.map(|repo| repo.root.clone()),
            branch: repo.and_then(|repo| repo.branch.clone()),
            dirty: repo.is_some_and(|repo| repo.dirty),
            ahead: repo.map_or(0, |repo| repo.ahead),
            behind: repo.map_or(0, |repo| repo.behind),
            files: repo.map(|repo| repo.files),
            changes: self.attribution.changes_of(id, self.rules.shared),
        }
    }

    /// Why git failed for the terminal, if it did.
    pub fn error_of(&self, id: i64) -> Option<&crate::git::GitError> {
        self.terminals.get(&id)?.git.error()
    }
}
