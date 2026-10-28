//! Keeping the tree true to the disk: which folder it is of, and reading folders off the main thread.
//!
//! A folder is listed on GPUI's background executor, never where a frame is drawn: a huge folder or a
//! network volume that answers slowly must not freeze the window. At most one read of a folder is in
//! flight, an answer for a root the tree has left is dropped, and the window is told only when an answer
//! changes what is shown.

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::storage::files::listing;
use wormhole_core::view::file_tree::{Dir, ENTRY_CAP, Failure};

use super::{Files, Line, REFRESH_EVERY};
use crate::window::facts::WindowFacts;

impl Files {
    /// Points the tree of `session` at the folder it should show: the one chosen by hand, else the
    /// terminal's. A new root starts a new tree (and a new generation, so a read still on its way for
    /// the old one is ignored). Whether the root changed; nothing is notified, so the draw can call it.
    pub(super) fn retarget(&mut self, session: i64, cx: &mut Context<Self>) -> bool {
        let of = self.of_mut(session);
        let wanted = of.chosen.clone().or_else(|| of.followed.clone());
        if of.root == wanted {
            return false;
        }
        of.root = wanted;
        of.generation += 1;
        of.tree.clear();
        of.reading.clear();
        of.selected = None;
        of.stale = true;
        if of.root.is_some() {
            self.read(session, PathBuf::new(), cx);
        }
        true
    }

    /// The terminal is in `folder`: the tree follows it unless a folder was chosen.
    pub(super) fn follow_terminal_folder(
        &mut self,
        session: i64,
        folder: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.of_mut(session).followed = folder;
        self.retarget(session, cx)
    }

    /// Reads the folder `rel` (below the root, `""` for the root) of `session`'s tree in the background,
    /// unless it is being read already.
    pub(super) fn read(&mut self, session: i64, rel: PathBuf, cx: &mut Context<Self>) {
        let Some(of) = self.of.get_mut(&session) else { return };
        let Some(root) = of.root.clone() else { return };
        if !of.reading.insert(rel.clone()) {
            return;
        }
        let generation = of.generation;
        cx.spawn(async move |this, cx| {
            let asked = rel.clone();
            let answer = cx
                .background_spawn(async move {
                    match listing::list_below(&root, &asked) {
                        Ok(found) => Ok(Dir::from_listing(found, ENTRY_CAP)),
                        Err(err) => Err(Failure::of(&err)),
                    }
                })
                .await;
            this.update(cx, |files, cx| files.arrived(session, generation, rel, answer, cx)).ok();
        })
        .detach();
    }

    /// A read came back. It is dropped when the tree has moved to another root since it was asked for.
    fn arrived(
        &mut self,
        session: i64,
        generation: u64,
        rel: PathBuf,
        answer: Result<Dir, Failure>,
        cx: &mut Context<Self>,
    ) {
        let Some(of) = self.of.get_mut(&session).filter(|of| of.generation == generation) else { return };
        of.reading.remove(&rel);
        if of.tree.set(&rel, answer) {
            of.stale = true;
            cx.notify();
        }
    }

    /// Reads every folder on show again: the root and the folders open under it.
    pub(super) fn reread(&mut self, session: i64, cx: &mut Context<Self>) {
        let Some(of) = self.of.get(&session) else { return };
        for rel in of.tree.wanted() {
            self.read(session, rel, cx);
        }
    }

    /// The tab was shown, or the window came forward: what the disk says now.
    pub(super) fn look_again(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        let Some(active) = facts.active() else { return };
        let session = active.id;
        self.follow(facts);
        if self.follow_terminal_folder(session, active.band_folder(), cx) {
            cx.notify();
        }
        self.reread(session, cx);
    }

    /// Once per window tick. While the tab is on show the terminal's folder is followed, and about every
    /// two seconds the folders on show are read again, so a file made in the terminal appears. Nothing
    /// is read while the tab is hidden.
    pub(super) fn tick(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        if facts.tab != wormhole_core::config::TabId::Files || !facts.panel_shown {
            return;
        }
        let Some(active) = facts.active() else { return };
        let session = active.id;
        self.follow(facts);
        if self.follow_terminal_folder(session, active.band_folder(), cx) {
            cx.notify();
        }
        if facts.ticks.is_multiple_of(REFRESH_EVERY) {
            self.reread(session, cx);
        }
    }

    /// Rebuilds the list from the tree if the tree changed since it was last built. Cheap when it
    /// did not, so the draw calls it every frame (a scroll tick too).
    pub(super) fn sync_list(&mut self, session: i64) {
        let of = self.of_mut(session);
        if !std::mem::take(&mut of.stale) {
            return;
        }
        let mut lines = Vec::new();
        if of.root.is_some() {
            lines.push(Line::Head);
            lines.extend(of.tree.rows().into_iter().map(Line::Row));
            lines.push(Line::Spacer);
        }
        of.list.set(lines);
    }
}
