//! The reads of a terminal's repository: asking git for the status in the background, the cheap look
//! for a `.git` where git said there is no repository, and what the tick decides for each terminal.

use std::path::PathBuf;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::git::attribution::{Place, Snapshot};
use wormhole_core::git::discover::{self, Sighting};
use wormhole_core::git::{self, RepoStatus};
use wormhole_core::view::sidebar::RowState;

use super::state::{GitPoll, Refreshed, Sight};
use super::watches::RepoWatches;
use crate::terminal_view::TerminalSummary;
use crate::window::Workspace;
use crate::window::repo::Observed;
use crate::window::repo::job::Ticket;

/// How long after a `.git` first shows up the second look is taken (see [`discover::Absent`]).
const PROBE_CONFIRM: std::time::Duration = std::time::Duration::from_millis(150);

impl Workspace {
    /// Refresh git status for a session in the background.
    pub(crate) fn refresh_git(&mut self, ix: usize, cx: &mut Context<Self>) {
        // A file a feature watches (an open diff's) is looked at in the same background hop, so a
        // diff an agent made stale is noticed within one poll.
        let watched = if ix == self.active { self.repo_watched(cx) } else { None };
        let Some(session) = self.sessions.get_mut(ix) else { return };
        let Some(cwd) = session.terminal().read(cx).cwd.clone() else { return };
        let compare = session.git.compare.clone();
        let Some(wanted) = session.git.begin_refresh((cwd.clone(), compare.clone())) else { return };
        let (id, ticket) = (session.id, wanted.ticket());
        let task = cx.background_executor().spawn(async move {
            let observed = watched.map(Observed::of);
            // A compare branch that was deleted falls back to HEAD there, and says so. Submodules get
            // their commits looked up here too: a process only for a pair not seen before.
            let status = git::status(&cwd, compare.as_deref()).map(|status| {
                status.map(|mut status| {
                    git::annotate(&mut status);
                    status
                })
            });
            // What each changed file looks like, in the same hop, dated by when the disk was read:
            // the sidebar rows count the files a terminal changed, which is what differs between two
            // of these. Not for a status against a branch (Compare with): its files are not the
            // working tree's, so while a repository's only terminal compares, no pictures come and
            // that reads as one long gap between two.
            let snapshot = status
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .and_then(|status| Snapshot::take(status, Instant::now()));
            (status, observed, snapshot)
        });
        cx.spawn(async move |this, cx| {
            let (status, observed, snapshot) = task.await;
            let _ = this
                .update(cx, |this, cx| this.finish_git_refresh(id, ticket, status, observed, snapshot, cx));
        })
        .detach();
    }

    /// Read the status of terminal `ix` against `branch` from now on (`None`: HEAD): the answer for
    /// the old one is dropped, so the list never shows one branch's files under another's name.
    pub(crate) fn compare_with(&mut self, ix: usize, branch: Option<String>, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get_mut(ix) else { return };
        session.git.compare = branch;
        session.git.status = None;
        self.refresh_git(ix, cx);
        cx.notify();
    }

    /// Look for a `.git` near the terminal's folder, which git said is in no repository: the
    /// replacement for polling git about it. The `stat` walk runs off the UI thread; a change is
    /// confirmed by a second look shortly after (`git init` writes `.git` a piece at a time), and
    /// only then is git asked, through the normal refresh.
    pub(crate) fn probe_repo(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get_mut(ix) else { return };
        let Some(cwd) = session.terminal().read(cx).cwd.clone() else { return };
        if !session.git.begin_probe(&cwd) {
            return;
        }
        let id = session.id;
        let looked = cwd.clone();
        let task = cx.background_executor().spawn(async move { discover::probe(&looked) });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| this.finish_probe(id, cwd, found, cx));
        })
        .detach();
    }

    fn finish_probe(&mut self, id: i64, cwd: PathBuf, found: Option<PathBuf>, cx: &mut Context<Self>) {
        let Some(ix) = self.sessions.iter().position(|s| s.id == id) else { return };
        match self.sessions[ix].git.finish_probe(&cwd, found) {
            Sighting::Unchanged => {}
            Sighting::LookAgain => {
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(PROBE_CONFIRM).await;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(ix) = this.sessions.iter().position(|s| s.id == id) {
                            this.probe_repo(ix, cx);
                        }
                    });
                })
                .detach();
            }
            Sighting::AskGit => self.refresh_git(ix, cx),
        }
    }

    fn finish_git_refresh(
        &mut self,
        id: i64,
        ticket: Ticket,
        status: anyhow::Result<Option<RepoStatus>>,
        observed: Option<Observed>,
        snapshot: Option<Snapshot>,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.sessions.iter().position(|s| s.id == id) else { return };
        let before = self.git_before(ix, cx);
        let session = &mut self.sessions[ix];
        let now = session.terminal().read(cx).cwd.clone().map(|cwd| (cwd, session.git.compare.clone()));
        match session.git.finish_refresh(ticket, now.as_ref(), status) {
            Refreshed::Stale => {}
            Refreshed::Applied { changed } => {
                if changed {
                    cx.notify();
                    self.publish_git(ix, before, cx);
                }
                // Only an answer that is still wanted: a stale one is a picture of a read that was
                // superseded, and dropped like its status.
                if let Some(snapshot) = snapshot {
                    self.observe_changes(ix, snapshot, cx);
                }
                // The features that follow the answer (an open diff) look at it, for the terminal on
                // screen.
                if ix == self.active {
                    self.repo_was_read(id, observed, cx);
                }
            }
        }
    }

    /// Something changed in the repository at `root` (its watch said so, after the debounce): read it for
    /// every terminal in it that is not unseen, without waiting for the tick.
    pub(crate) fn repo_changed(&mut self, root: &std::path::Path, cx: &mut Context<Self>) {
        for ix in 0..self.sessions.len() {
            let cwd = self.sessions[ix].terminal().read(cx).cwd.clone();
            let git = &mut self.sessions[ix].git;
            if git.waits_for_retry() || git.place(cwd.as_deref()) != Place::In(root) {
                continue;
            }
            if git.watch_hit() {
                self.refresh_git(ix, cx);
            }
        }
    }

    /// What the tick does about each terminal's repository: ask git, look for a `.git`, or nothing.
    /// A terminal whose git is missing waits for a Retry. `seen` is whether any of the window can be
    /// seen, `active` whether it is the window in front.
    ///
    /// A window nobody can see reads the repositories something works in as a window behind does: their
    /// pictures say who changed what (`git::attribution`), which is not known afterwards. Git's answers
    /// for the others change nothing anyone sees, so they are read only every 10 s, and at once when the
    /// window is seen again. In a large repository one read is most of a second of git's time.
    pub(crate) fn poll_repos(&mut self, seen: bool, active: bool, cx: &mut Context<Self>) {
        let working: Vec<PathBuf> = if seen {
            Vec::new()
        } else {
            self.sessions
                .iter()
                .filter(|session| works(&session.pane.summary))
                .filter_map(|session| {
                    // The terminal's folder, as the loop below and the reads use it.
                    let cwd = session.terminal().read(cx).cwd.clone();
                    match session.git.place(cwd.as_deref()) {
                        Place::In(root) => Some(root.to_path_buf()),
                        Place::Unknown | Place::Outside => None,
                    }
                })
                .collect()
        };
        // The repositories the terminals are in, for the watches. By the last one git named, not by where
        // the terminal is asked about now: a `cd` must not let go of a repository and start it again.
        let mut roots: Vec<PathBuf> = Vec::new();
        for session in &self.sessions {
            if let Some(root) = session.git.root()
                && !roots.iter().any(|known| known == root)
            {
                roots.push(root.to_path_buf());
            }
        }
        RepoWatches::claim(&roots, cx);
        for ix in 0..self.sessions.len() {
            if self.sessions[ix].git.waits_for_retry() {
                continue;
            }
            let cwd = self.sessions[ix].terminal().read(cx).cwd.clone();
            let git = &mut self.sessions[ix].git;
            let sight = if seen {
                if active && ix == self.active { Sight::Front } else { Sight::Behind }
            } else {
                match git.place(cwd.as_deref()) {
                    Place::In(root) if working.iter().any(|busy| busy == root) => Sight::Behind,
                    _ => Sight::Unseen,
                }
            };
            git.note_sight(sight);
            let watched = matches!(git.place(cwd.as_deref()), Place::In(root) if RepoWatches::live(root, cx));
            match git.poll(cwd.as_deref(), self.ticks + ix as u64, sight, watched) {
                GitPoll::Refresh => self.refresh_git(ix, cx),
                GitPoll::Probe => self.probe_repo(ix, cx),
                GitPoll::Idle => {}
            }
        }
    }
}

/// Whether a terminal may be changing files, for the polling of an unseen window: one that tells its
/// status while it says it works, a plain one while any program holds it. Generous on purpose (the
/// attribution's own rule depends on a setting): a wrong yes costs a read, a wrong no a misattribution.
fn works(summary: &TerminalSummary) -> bool {
    if summary.source.told() {
        summary.row_state() == RowState::Working
    } else {
        summary.program_in_foreground
    }
}
