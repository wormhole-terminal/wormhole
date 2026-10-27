//! The reads of a terminal's repository: asking git for the status in the background, the cheap look
//! for a `.git` where git said there is no repository, and what the tick decides for each terminal.

use std::path::PathBuf;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::git::attribution::Snapshot;
use wormhole_core::git::discover::{self, Sighting};
use wormhole_core::git::{self, RepoStatus};

use super::state::{GitPoll, Refreshed};
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
        // When the disk is looked at, which is what the snapshot's picture of it is dated by.
        let asked = Instant::now();
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
            // What each changed file looks like, in the same hop: the sidebar rows count the files a
            // terminal changed, which is what differs between two of these.
            let snapshot = status.as_ref().ok().and_then(Option::as_ref).and_then(Snapshot::take);
            (status, observed, snapshot)
        });
        cx.spawn(async move |this, cx| {
            let (status, observed, snapshot) = task.await;
            let _ = this.update(cx, |this, cx| {
                this.finish_git_refresh(
                    id,
                    ticket,
                    status,
                    observed,
                    snapshot.map(|snapshot| (snapshot, asked)),
                    cx,
                )
            });
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
        snapshot: Option<(Snapshot, Instant)>,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.sessions.iter().position(|s| s.id == id) else { return };
        let session = &mut self.sessions[ix];
        let now = session.terminal().read(cx).cwd.clone().map(|cwd| (cwd, session.git.compare.clone()));
        match session.git.finish_refresh(ticket, now.as_ref(), status) {
            Refreshed::Stale => {}
            Refreshed::Applied { changed } => {
                if changed {
                    cx.notify();
                }
                if let Some((snapshot, asked)) = snapshot {
                    self.observe_changes(snapshot, asked, cx);
                }
                // The features that follow the answer (an open diff) look at it, for the terminal on
                // screen.
                if ix == self.active {
                    self.repo_was_read(id, observed, cx);
                }
            }
        }
    }

    /// What the tick does about each terminal's repository: ask git, look for a `.git`, or nothing.
    /// A terminal whose git is missing waits for a Retry.
    pub(crate) fn poll_repos(&mut self, cx: &mut Context<Self>) {
        for ix in 0..self.sessions.len() {
            if self.sessions[ix].git.waits_for_retry() {
                continue;
            }
            let cwd = self.sessions[ix].terminal().read(cx).cwd.clone();
            match self.sessions[ix].git.poll(cwd.as_deref(), self.ticks + ix as u64, ix == self.active) {
                GitPoll::Refresh => self.refresh_git(ix, cx),
                GitPoll::Probe => self.probe_repo(ix, cx),
                GitPoll::Idle => {}
            }
        }
    }
}
