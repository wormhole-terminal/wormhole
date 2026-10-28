//! Keeping the tree true to the disk: which folder it is of, and reading folders off the main thread.
//!
//! A folder is listed on GPUI's background executor, never where a frame is drawn: a huge folder or a
//! network volume that answers slowly must not freeze the window. A read cannot be cancelled, so a folder
//! is read once at a time whatever the tree is doing (`Files::in_flight`), and a folder that looks from
//! outside as it did when it was last read is not listed again (`FileStamp`). An answer for a root the tree
//! has left is dropped, and the window is told only when an answer changes what is shown.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::storage::files::FileStamp;
use wormhole_core::storage::files::listing;
use wormhole_core::view::file_tree::{Dir, ENTRY_CAP, Failure};

use super::{Files, Line, REFRESH_EVERY};
use crate::window::facts::WindowFacts;

/// What a read of a folder found.
pub(super) struct Read {
    pub dir: Result<Dir, Failure>,
    /// How the folder looked just before it was read.
    pub stamp: Option<FileStamp>,
}

/// Reads the folder `rel` below `root` unless its `prior` stamp says nothing changed (`None`). Blocks on
/// the disk.
fn fetch(root: &Path, rel: &Path, prior: Option<FileStamp>) -> Option<Read> {
    // Taken before the listing, so a change while it is read shows up as one the next time.
    let stamp = listing::stamp_below(root, rel).ok();
    if prior.is_some() && prior == stamp {
        return None;
    }
    let dir = match listing::list_below(root, rel) {
        Ok(found) => Ok(Dir::from_listing(found, ENTRY_CAP)),
        Err(err) => Err(Failure::of(&err)),
    };
    Some(Read { dir, stamp })
}

/// Whether the tab is offered and on show: its feature is on, it is the panel's tab, and the panel is drawn.
/// A saved layout can hold the tab while its flag is off (the panel draws Changes then): nothing is read.
fn showing(facts: &WindowFacts<'_>) -> bool {
    facts.features.has(FeatureId::Files) && facts.tab == TabId::Files && facts.panel_shown
}

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
        of.stamps.clear();
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
        let Some(of) = self.of.get(&session) else { return };
        let Some(root) = of.root.clone() else { return };
        let full = root.join(&rel);
        // Only a folder the tree holds an answer for can be known to be unchanged.
        let prior = of.stamps.get(&rel).copied().filter(|_| of.tree.answered(&rel));
        if !self.in_flight.insert(full.clone()) {
            return;
        }
        let generation = of.generation;
        cx.spawn(async move |this, cx| {
            let asked = rel.clone();
            let answer = cx.background_spawn(async move { fetch(&root, &asked, prior) }).await;
            this.update(cx, |files, cx| files.arrived(session, generation, &full, rel, answer, cx)).ok();
        })
        .detach();
    }

    /// A read came back. It is dropped when the tree has moved to another root since it was asked for.
    pub(super) fn arrived(
        &mut self,
        session: i64,
        generation: u64,
        full: &Path,
        rel: PathBuf,
        answer: Option<Read>,
        cx: &mut Context<Self>,
    ) {
        self.in_flight.remove(full);
        let Some(of) = self.of.get_mut(&session).filter(|of| of.generation == generation) else { return };
        // Nothing was listed: the folder looks as it did.
        let Some(Read { dir, stamp }) = answer else { return };
        match (dir.is_ok(), stamp) {
            (true, Some(stamp)) => {
                of.stamps.insert(rel.clone(), stamp);
            }
            _ => {
                of.stamps.remove(&rel);
            }
        }
        if of.tree.set(&rel, dir) {
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
        if !facts.features.has(FeatureId::Files) {
            return;
        }
        let Some(active) = facts.active() else { return };
        let session = active.id;
        self.follow(facts);
        if self.follow_terminal_folder(session, active.band_folder(), cx) {
            cx.notify();
        }
        self.reread(session, cx);
    }

    /// Once per window tick. While the tab is on show the terminal's folder is followed, and about every
    /// two seconds the folders on show are looked at again, so a file made in the terminal appears. Nothing
    /// is read while the tab is hidden or its feature is off.
    pub(super) fn tick(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        if !showing(facts) {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{Read, fetch};

    #[test]
    fn a_folder_that_looks_as_it_did_is_not_listed_again() {
        let root =
            wormhole_core::platform::paths::temp().join(format!("wormhole-fetch-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a"), b"").unwrap();
        let Some(Read { dir, stamp }) = fetch(&root, Path::new(""), None) else { panic!("first read") };
        assert_eq!(dir.unwrap().entries.len(), 1);
        assert!(stamp.is_some());
        assert!(fetch(&root, Path::new(""), stamp).is_none(), "unchanged");
        fs::write(root.join("b"), b"").unwrap();
        let Some(Read { dir, .. }) = fetch(&root, Path::new(""), stamp) else { panic!("it changed") };
        assert_eq!(dir.unwrap().entries.len(), 2);
        // A folder that is gone is an answer too, not "unchanged".
        assert!(matches!(fetch(&root, Path::new("gone"), stamp), Some(Read { dir: Err(_), stamp: None })));
        fs::remove_dir_all(&root).ok();
    }
}
