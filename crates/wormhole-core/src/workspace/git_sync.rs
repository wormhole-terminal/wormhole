//! The model's side of the git feed: it reads where each terminal is and what it is doing from the
//! terminals' facts, so the shell only does the I/O it is asked for and hands the answers back.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::git::RepoStatus;
use crate::git::attribution::Snapshot;

use super::event::Event;
use super::facts::Millis;
use super::git_feed::{ChangeRules, GitJob, Wanted};
use super::model::WorkspaceModel;

impl WorkspaceModel {
    /// Bring the feed up to date with the terminals (their folders, how they are seen, what they are
    /// doing) and say what is due: the reads and looks to start now. Call it when a terminal's folder
    /// or status changed, when the window's state did, and at [`git_next_due`](Self::git_next_due).
    pub fn git_poll(&mut self, now: Millis, at: Instant) -> Vec<GitJob> {
        self.git_sync(at);
        self.git.poll(now)
    }

    /// When the soonest read or look is due; `None` when only an event can make one due.
    pub fn git_next_due(&self, now: Millis) -> Option<Millis> {
        self.git.next_due(now)
    }

    /// A read came back (see [`GitJob::Read`]): what it said shows on the rows it concerns.
    pub fn git_read(
        &mut self,
        id: i64,
        wanted: &Wanted,
        result: anyhow::Result<Option<RepoStatus>>,
        snapshot: Option<Snapshot>,
    ) -> Vec<Event> {
        if self.git.finish_read(id, wanted, result, snapshot) {
            self.git_facts();
        }
        self.out.take()
    }

    /// A look came back (see [`GitJob::Probe`]).
    pub fn git_probed(&mut self, id: i64, cwd: &Path, found: Option<PathBuf>, now: Millis) {
        self.git.finish_probe(id, cwd, found, now);
    }

    /// Something in the repository at `root` changed.
    pub fn git_repo_changed(&mut self, root: &Path) {
        self.git.repo_changed(root);
    }

    /// The repositories the terminals are in, for the shell to follow through their files.
    pub fn git_roots(&self) -> Vec<PathBuf> {
        self.git.roots()
    }

    /// A repository is (or is no longer) followed by a running watch.
    pub fn git_watched(&mut self, root: &Path, live: bool) {
        self.git.set_watched(root, live);
    }

    /// The window can be seen at all (not minimised, hidden or covered).
    pub fn set_window_seen(&mut self, seen: bool) {
        self.git.set_seen(seen);
    }

    /// git was missing and may be there now.
    pub fn git_retry(&mut self) {
        self.git.retry();
    }

    /// The feed's reading of each terminal, from the facts: folder, sight, and whether it works.
    fn git_sync(&mut self, at: Instant) {
        let shown = self.shown();
        let active = self.status.window_active;
        let mut moved = false;
        for session in &self.sessions {
            self.git.folder(session.id, session.facts.cwd.as_deref());
            moved |= self.git.track(session.id, &session.facts, at);
        }
        let terminals = self
            .sessions
            .iter()
            .enumerate()
            .map(|(ix, session)| (session.id, shown.contains(&ix), works(&session.facts)));
        self.git.set_sights(active, terminals);
        let closed: Vec<i64> =
            self.git.known().filter(|id| !self.sessions.iter().any(|session| session.id == *id)).collect();
        for id in closed {
            moved |= self.git.forget(id);
        }
        if moved {
            self.git_facts();
        }
    }

    /// The rules of Settings → Changed files moved: the counts are made again under them.
    pub(super) fn set_change_rules(&mut self, rules: ChangeRules) {
        if self.cfg.changes == rules {
            return;
        }
        self.cfg.changes = rules;
        self.git.set_rules(rules);
        self.git_facts();
    }

    /// Put what the feed says into every terminal's facts; a change that shows is a `Listing`.
    fn git_facts(&mut self) {
        let mut changed = false;
        for session in &mut self.sessions {
            let before = session.facts.clone();
            self.git.facts_of(session.id).apply(&mut session.facts);
            changed |= session.facts != before;
        }
        if changed {
            self.emit(Event::Listing);
        }
    }
}

/// Whether a terminal may be changing files, for the polling of a window nobody can see: one that
/// tells its status while it says it works, a plain one while any program holds it. Generous on
/// purpose: a wrong yes costs a read, a wrong no a misattribution.
fn works(facts: &super::facts::TerminalFacts) -> bool {
    if facts.told() {
        facts.row_state() == crate::view::sidebar::RowState::Working
    } else {
        facts.command.is_some()
    }
}
