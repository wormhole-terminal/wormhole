//! The Changes list: the rows built from a terminal's status (cheaply kept up to date, since the frame
//! asks every time), the keys over them, and the folds.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::git::{DiffSpec, FileChange, RepoStatus};
use wormhole_core::view::focus::list::Item;
use wormhole_core::view::panel as rules;

use super::rows::{ChangeRow, Folded, Group, build_rows, collapse_key, display_order, untracked_apart};
use super::{Changes, ChangesOf};
use crate::focus_keys::Activate;
use crate::settings::settings_in;
use crate::style::panel_list::Rows;
use crate::window::feature::WalkFns;
use crate::window::focus::Walkable;

/// What opening a file's diff from the list needs of the repository, kept with the rows so a click or a
/// key needs neither the status nor a new look at the settings: where the repository is, what the working
/// tree is compared with, and the files the diff's arrows step through.
pub(in crate::window::features::git) struct DiffTarget {
    pub root: std::path::PathBuf,
    pub base_rev: String,
    pub siblings: Vec<FileChange>,
}

/// What the rows were built from. They are rebuilt only when this changes, not on every scroll tick.
pub(super) struct Built {
    pub files: Rc<Vec<FileChange>>,
    pub target: Rc<DiffTarget>,
    tree: bool,
    group_untracked: bool,
    collapsed: HashSet<String>,
    folds: HashMap<Group, bool>,
}

#[derive(Default)]
pub(super) struct ChangesList {
    pub built: Option<Built>,
    pub rows: Rows<ChangeRow>,
}

impl ChangesList {
    fn sync(
        &mut self,
        status: Option<&RepoStatus>,
        tree: bool,
        group_untracked: bool,
        folded: Folded,
        folds: &HashMap<Group, bool>,
    ) {
        let Some(status) = status else {
            self.built = None;
            self.rows.set(Vec::new());
            return;
        };
        let group_untracked = untracked_apart(status, group_untracked);
        let same_files = self.built.as_ref().is_some_and(|b| {
            b.files[..] == status.files[..]
                && b.target.root == status.root
                && b.target.base_rev == status.base_rev
        });
        if same_files
            && self.built.as_ref().is_some_and(|b| {
                b.tree == tree
                    && b.group_untracked == group_untracked
                    && b.collapsed == *folded.set
                    && b.folds == *folds
            })
        {
            return;
        }
        let files = match &self.built {
            Some(b) if same_files => b.files.clone(),
            _ => Rc::new(status.files.clone()),
        };
        let target = match &self.built {
            Some(b) if same_files && b.tree == tree && b.group_untracked == group_untracked => {
                b.target.clone()
            }
            _ => Rc::new(DiffTarget {
                root: status.root.clone(),
                base_rev: status.base_rev.clone(),
                siblings: display_order(status, tree, group_untracked),
            }),
        };
        self.rows.set(build_rows(&files, tree, group_untracked, folded, folds));
        self.built = Some(Built {
            files,
            target,
            tree,
            group_untracked,
            collapsed: folded.set.clone(),
            folds: folds.clone(),
        });
    }
}

impl Changes {
    /// Refresh `session`'s rows from its git status and the settings. Cheap when nothing changed, so it
    /// runs on every frame (including every scroll tick).
    pub(super) fn sync_list(&mut self, session: i64, status: Option<&RepoStatus>, cx: &App) {
        let s = settings_in(&self.profile, cx);
        let (tree, group_untracked) = (s.changes_tree, s.group_untracked);
        let ChangesOf { list, collapsed, folds, .. } = self.of_mut(session);
        list.sync(status, tree, group_untracked, Folded { set: collapsed, session }, folds);
    }

    /// A click on a section's band: fold it, or unfold it (one folded for its size too).
    pub(super) fn toggle_section(
        &mut self,
        session: i64,
        group: Group,
        count: usize,
        cx: &mut Context<Self>,
    ) {
        let of = self.of_mut(session);
        let folded = rules::section_folded(count, of.folds.get(&group).copied());
        of.folds.insert(group, !folded);
        cx.notify();
    }

    pub(super) fn toggle_dir(&mut self, session: i64, group: Group, path: String, cx: &mut Context<Self>) {
        let key = collapse_key(session, group, &path);
        let collapsed = &mut self.of_mut(session).collapsed;
        if !collapsed.remove(&key) {
            collapsed.insert(key);
        }
        cx.notify();
    }

    /// Opens the diff of working-tree change `file` of `session`.
    pub(in crate::window::features::git) fn open_change(
        &mut self,
        session: i64,
        file: FileChange,
        target: &DiffTarget,
        cx: &mut Context<Self>,
    ) {
        let spec = DiffSpec { old: target.base_rev.clone(), new: None };
        let siblings = Rc::new(target.siblings.clone());
        self.open_diff(session, target.root.clone(), file, spec, siblings, None, cx);
    }

    /// The Changes rows the keys can be on: each section's band (a ⇥ stop), its folders and files.
    fn walkable(&self, session: i64) -> Vec<Walkable> {
        let Some(of) = self.of.get(&session) else { return Vec::new() };
        (of.list.rows.items().iter().enumerate())
            .filter_map(|(row, line)| {
                let (level, fold, band) = match line {
                    ChangeRow::Band { count, folded, .. } => (0, (*count > 0).then_some(!folded), true),
                    ChangeRow::Dir { depth, collapsed, .. } => (1 + *depth as u8, Some(!collapsed), false),
                    ChangeRow::File { depth, .. } => (1 + *depth as u8, None, false),
                    ChangeRow::FoldNote(_) | ChangeRow::Spacer => return None,
                };
                Some(Walkable { row, item: Item { level, fold }, band })
            })
            .collect()
    }

    /// → / ← on a band folds or opens its section, on a folder the folder.
    fn fold_row(&mut self, session: i64, row: usize, cx: &mut Context<Self>) {
        let line = self.of.get(&session).and_then(|of| of.list.rows.items().get(row).cloned());
        match line {
            Some(ChangeRow::Band { group, count, .. }) => self.toggle_section(session, group, count, cx),
            Some(ChangeRow::Dir { group, path, .. }) => self.toggle_dir(session, group, path, cx),
            _ => {}
        }
    }

    /// A press on a file opens its diff, on a band or folder folds it (`following`: only a file's).
    /// `focus/panel.rs` sends ↩ here, and Space only where it folds. The list keeps the keyboard.
    fn press_row(&mut self, session: i64, row: usize, following: bool, cx: &mut Context<Self>) -> bool {
        let Some(of) = self.of.get(&session) else { return true };
        match of.list.rows.items().get(row) {
            Some(ChangeRow::File { ix, .. }) => {
                let opened =
                    of.list.built.as_ref().and_then(|b| Some((b.files.get(*ix)?.clone(), b.target.clone())));
                if let Some((file, target)) = opened {
                    self.open_change(session, file, &target, cx);
                }
            }
            Some(ChangeRow::Band { .. } | ChangeRow::Dir { .. }) if !following => {
                self.fold_row(session, row, cx)
            }
            _ => {}
        }
        true
    }

    fn reveal(&self, session: i64, row: usize) {
        if let Some(of) = self.of.get(&session) {
            of.list.rows.reveal(row);
        }
    }

    /// The list, to scroll from outside.
    pub(super) fn list_state(&self, session: i64) -> Option<ListState> {
        Some(self.of.get(&session)?.list.rows.state())
    }
}

/// The Changes tab's keys, over the entity.
pub(super) const WALK: WalkFns = WalkFns {
    walkable: |entity, facts, cx| {
        let changes = super::def::typed(entity);
        facts.active().map(|active| changes.read(cx).walkable(active.id)).unwrap_or_default()
    },
    press: |entity, _, session, row, _: Activate, following, _, cx| {
        super::def::typed(entity).update(cx, |changes, cx| changes.press_row(session, row, following, cx))
    },
    fold: |entity, _, session, row, cx| {
        super::def::typed(entity).update(cx, |changes, cx| changes.fold_row(session, row, cx));
    },
    reveal: |entity, facts, row, cx| {
        if let Some(active) = facts.active() {
            super::def::typed(entity).read(cx).reveal(active.id, row);
        }
    },
    list: |entity, facts, cx| super::def::typed(entity).read(cx).list_state(facts.active()?.id),
};
