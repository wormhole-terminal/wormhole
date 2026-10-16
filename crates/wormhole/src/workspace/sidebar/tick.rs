//! The sidebar's part of the window's 400 ms tick: what moves on its own (a row's times), a
//! terminal that changed band, and what the bands learn off the UI thread (a detached HEAD's
//! commit) or from disk now and then (which repository a worktree belongs to).
use std::path::{Path, PathBuf};

use super::*;

impl Workspace {
    /// The repository a worktree belongs to, by its root; read from disk once per refresh.
    pub(super) fn worktree_of(&self, root: &Path) -> Option<PathBuf> {
        let mut cache = self.sidebar.worktrees.borrow_mut();
        cache.entry(root.to_path_buf()).or_insert_with(|| wormhole_core::git::worktree_of(root)).clone()
    }

    pub(in crate::workspace) fn sidebar_tick(&mut self, window_active: bool, cx: &mut Context<Self>) {
        if self.ticks % 5 == 1 {
            self.sidebar.worktrees.borrow_mut().clear();
        }
        // Only what is on screen: the bands' tooltips, the times on the rows.
        // The tab bar draws the rows' times too.
        let shown = self.panes.sidebar_open || self.sidebar.tabs.layout.is_some();
        if shown && self.sidebar.view.grouping == Grouping::Folder && settings(cx).sidebar_branch {
            self.look_up_detached_heads(cx);
        }
        self.follow_active_band(cx);
        // The times on the rows ("Working · 3m", a command's clock) move on their own; redraw
        // when one would read differently, but not for a window nobody is looking at.
        if window_active && shown {
            let now = row_times(&self.row_infos(cx), cx);
            if *self.sidebar.times_drawn.borrow() != now {
                *self.sidebar.times_drawn.borrow_mut() = now;
                cx.notify();
            }
        }
    }

    /// The selected terminal moved to another band (a `cd` out of its repository): it stays
    /// selected, comes into view and flashes once.
    fn follow_active_band(&mut self, cx: &mut Context<Self>) {
        if self.sidebar.view.grouping != Grouping::Folder || self.sessions.is_empty() {
            self.sidebar.active_band = None;
            return;
        }
        let session = self.active_session();
        let folder = session.terminal().read(cx).cwd.as_deref().map(|cwd| session.repo_root.of(cwd));
        let now = (session.id, folder);
        match &self.sidebar.active_band {
            Some((id, before)) if *id == now.0 && *before != now.1 => {
                let generation = self.sidebar.flash.map_or(0, |(_, n)| n + 1);
                self.sidebar.flash = Some((now.0, generation));
                self.sidebar.scroll_to = Some(now.0);
                cx.notify();
            }
            _ => {}
        }
        self.sidebar.active_band = Some(now);
    }

    /// Detached HEADs name their commit in the band's tooltip: ask git for it, off the UI
    /// thread, now and then while a repository's HEAD is detached.
    fn look_up_detached_heads(&mut self, cx: &mut Context<Self>) {
        const EVERY: u64 = 25;
        let detached: Vec<PathBuf> = self
            .sessions
            .iter()
            .filter_map(|s| {
                s.git.status.as_ref().filter(|status| status.branch.is_none()).map(|st| st.root.clone())
            })
            .collect();
        self.sidebar.heads.retain(|root, _| detached.contains(root));
        self.sidebar.head_lookups.retain(|task| !task.is_ready());
        for root in detached {
            let due = self.sidebar.heads.get(&root).is_none_or(|(at, _)| self.ticks >= at + EVERY);
            if !due {
                continue;
            }
            self.sidebar
                .heads
                .insert(root.clone(), (self.ticks, self.sidebar.heads.get(&root).and_then(|h| h.1.clone())));
            let lookup = cx.spawn(async move |this, cx| {
                let found = cx
                    .background_spawn({
                        let root = root.clone();
                        async move { wormhole_core::git::head_commit(&root) }
                    })
                    .await;
                this.update(cx, |this, cx| {
                    let commit = found.unwrap_or_else(|err| {
                        tracing::debug!("detached HEAD lookup failed: {err:#}");
                        None
                    });
                    if let Some(entry) = this.sidebar.heads.get_mut(&root)
                        && entry.1 != commit
                    {
                        entry.1 = commit;
                        cx.notify();
                    }
                })
                .ok();
            });
            self.sidebar.head_lookups.push(lookup);
        }
    }
}
