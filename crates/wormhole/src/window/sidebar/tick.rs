//! The sidebar's part of the window's 400 ms tick: what moves on its own (a row's times), and
//! which repository a worktree belongs to, read from disk now and then.
use std::path::{Path, PathBuf};

use gpui_kit::*;

use super::facts::row_times;
use crate::workspace::Workspace;

impl Workspace {
    /// The repository a worktree belongs to, by its root; read from disk once per refresh.
    pub(super) fn worktree_of(&self, root: &Path) -> Option<PathBuf> {
        let mut cache = self.sidebar.worktrees.borrow_mut();
        cache.entry(root.to_path_buf()).or_insert_with(|| wormhole_core::git::worktree_of(root)).clone()
    }

    pub(in crate::window) fn sidebar_tick(&mut self, window_active: bool, cx: &mut Context<Self>) {
        if self.ticks % 5 == 1 {
            self.sidebar.worktrees.borrow_mut().clear();
        }
        // Only what is on screen: the times on the rows (the tab bar draws them too).
        let shown = self.panes.sidebar_open || self.sidebar.tabs.on;
        // The times on the rows ("Working · 3m", a command's clock) move on their own; redraw
        // when one would read differently, but not for a window nobody is looking at.
        if window_active && shown {
            let now = row_times(&self.row_infos(cx), &self.profile, cx);
            if *self.sidebar.times_drawn.borrow() != now {
                *self.sidebar.times_drawn.borrow_mut() = now;
                cx.notify();
            }
        }
    }
}
