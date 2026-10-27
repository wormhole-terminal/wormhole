//! The History tab's reads: the log of the branch shown, a commit's files and details, and which commit to open.
//! All off the UI thread; a result is applied only if it answers the request the tab still waits for.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::config::TabId;
use wormhole_core::git::{self, Commit, FileChange, GitError, LOG_LIMIT};

use super::History;
use super::shown::Shown;
use crate::features::git::changes::Changes;
use crate::window::feature::FeatureEvent;

/// How many of a commit's files are looked for in the working tree; a commit that touches more
/// shows no icon on the rest.
const PRESENT_CHECKED: usize = 2000;

/// The paths of `files` that are regular files inside the repository at `root` now (a file the
/// commit deleted, or one removed since, is not).
fn present_files(root: &Path, files: &[FileChange]) -> HashSet<String> {
    (files.iter().take(PRESENT_CHECKED))
        .filter(|file| wormhole_core::storage::files::resolve_within(root, &file.path).is_ok())
        .map(|file| file.path.clone())
        .collect()
}

impl History {
    /// Reads the log of the branch shown for terminal `session`, in the folder its terminal is in.
    pub(super) fn refresh(&mut self, session: i64, cx: &mut Context<Self>) {
        let Some(terminal) = self.terminals.get(&session).cloned() else { return };
        let Some(cwd) = terminal.read(cx).cwd.clone() else { return };
        let Some(state) = self.of.get_mut(&session) else { return };
        let rev = state.rev.clone();
        let Some(wanted) = state.begin_refresh((cwd.clone(), rev.clone())) else { return };
        let ticket = wanted.ticket();
        let task = cx.background_executor().spawn(async move {
            let root = match git::repo_root(&cwd) {
                Ok(Some(root)) => root,
                Ok(None) => return Ok(None),
                Err(err) => {
                    tracing::debug!(?cwd, "git could not read the history: {err:#}");
                    return Err(GitError::of(&err));
                }
            };
            // A branch that was deleted since it was picked gets its own words.
            let rev = rev.as_deref().unwrap_or("HEAD");
            let commits = git::log(&root, rev, LOG_LIMIT).map_err(|err| git::log_failure(&root, rev, &err));
            Ok(Some((root, commits)))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let now = terminal.read(cx).cwd.clone();
                let Some(state) = this.of.get_mut(&session) else { return };
                let now = now.map(|cwd| (cwd, state.rev.clone()));
                if state.finish_refresh(ticket, now.as_ref(), result) {
                    // What points at the open commit may have moved with the log.
                    let stale = state.details_stale();
                    let root = state.root.clone();
                    cx.notify();
                    if let (Some((sha, request)), Some(root)) = (stale, root) {
                        this.fetch_details(session, root, sha, request, cx);
                    }
                }
            });
        })
        .detach();
    }

    /// A click, or → ←, on a commit: show or fold its files.
    pub(super) fn toggle_commit(&mut self, session: i64, sha: String, cx: &mut Context<Self>) {
        let Some(state) = self.of.get_mut(&session) else { return };
        let fetch = state.toggle(&sha);
        // Read at every opening, so what points at the commit is what is there now.
        let details =
            (state.expanded.as_deref() == Some(sha.as_str())).then(|| state.want_details(&sha)).flatten();
        let root = state.root.clone();
        cx.notify();
        if let Some((root, commit)) = fetch {
            self.fetch_commit_files(session, root, commit, cx);
        }
        if let (Some(request), Some(root)) = (details, root) {
            self.fetch_details(session, root, sha, request, cx);
        }
    }

    /// "Show more" / "Show less" under the open commit's message.
    pub(super) fn toggle_body(&mut self, session: i64, cx: &mut Context<Self>) {
        if let Some(state) = self.of.get_mut(&session) {
            state.toggle_body();
            cx.notify();
        }
    }

    /// A parent in the details: open it, and bring its row into view.
    pub(super) fn reveal_parent(&mut self, session: i64, sha: String, cx: &mut Context<Self>) {
        let Some(state) = self.of.get_mut(&session) else { return };
        if state.expanded.as_deref() != Some(sha.as_str()) {
            state.reveal = Some(sha.clone());
            self.toggle_commit(session, sha, cx);
        }
    }

    /// Show `commit` of the repository at `root` open in the tab: the palette found it. When it is not
    /// on the branch the log shows, the log goes back to HEAD.
    pub(super) fn reveal_commit(
        &mut self,
        session: i64,
        root: PathBuf,
        commit: Commit,
        cx: &mut Context<Self>,
    ) {
        let Some(rev) = self.of.get(&session).and_then(|state| state.rev.clone()) else {
            return self.open_commit(session, root, commit, true, cx);
        };
        let asked = cx.background_executor().spawn({
            let (root, sha) = (root.clone(), commit.sha.clone());
            async move { git::is_ancestor(&root, &sha, &rev).unwrap_or(false) }
        });
        cx.spawn(async move |this, cx| {
            let on_branch = asked.await;
            let _ = this.update(cx, |this, cx| this.open_commit(session, root, commit, on_branch, cx));
        })
        .detach();
    }

    fn open_commit(
        &mut self,
        session: i64,
        root: PathBuf,
        commit: Commit,
        on_branch: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get_mut(&session) else { return };
        let (sha, details_root) = (commit.sha.clone(), root.clone());
        let fetch = state.open(root, commit, on_branch);
        let details = state.want_details(&sha);
        cx.emit(FeatureEvent::ShowTab(TabId::History));
        cx.notify();
        if let Some((root, commit)) = fetch {
            self.fetch_commit_files(session, root, commit, cx);
        }
        if let Some(request) = details {
            self.fetch_details(session, details_root, sha, request, cx);
        }
    }

    /// Reads `sha`'s details off the UI thread (and builds what the block draws from them there), as read
    /// number `request`: an answer for an older one is dropped. A copy that waited for them follows.
    pub(super) fn fetch_details(
        &mut self,
        session: i64,
        root: PathBuf,
        sha: String,
        request: u64,
        cx: &mut Context<Self>,
    ) {
        let task = cx.background_executor().spawn({
            let sha = sha.clone();
            async move {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                git::commit_details(&root, &sha)
                    .map(|details| Shown::new(details, now))
                    .map_err(|e| format!("{e:#}"))
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let Some(state) = this.of.get_mut(&session) else { return };
                let taken = state.details_loaded(&sha, request, result);
                cx.notify();
                if taken {
                    this.finish_pending_copy(session, &sha, cx);
                }
            });
        })
        .detach();
    }

    fn fetch_commit_files(&mut self, session: i64, root: PathBuf, commit: Commit, cx: &mut Context<Self>) {
        let sha = commit.sha.clone();
        let task = cx.background_executor().spawn(async move {
            let files = git::commit_files(&root, &commit).map_err(|e| format!("{e:#}"))?;
            let present = present_files(&root, &files);
            Ok::<_, String>((files, present))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(state) = this.of.get_mut(&session) {
                    match result {
                        Ok((files, present)) => {
                            state.files_loaded(&sha, Ok(files));
                            state.files_present(&sha, present);
                        }
                        Err(err) => state.files_loaded(&sha, Err(err)),
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// A commit's file opens its diff, in Changes' view over the terminal.
    pub(super) fn open_commit_file(
        &mut self,
        session: i64,
        commit: Commit,
        file: FileChange,
        changes: &Entity<Changes>,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get(&session) else { return };
        let Some(root) = state.root.clone() else { return };
        let siblings = state.loaded(&commit.sha).cloned().unwrap_or_default();
        let spec = git::commit_spec(&commit);
        let context = Some(commit.short.clone().into());
        changes.update(cx, |changes, cx| {
            changes.open_diff(session, root, file, spec, siblings, context, cx);
        });
    }
}
