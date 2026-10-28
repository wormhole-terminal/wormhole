//! The keys over the tab's rows (F-70, as for the Changes list) and the actions they and the mouse share.
//! The path button is the card's head, a ⇥ stop; ↑ ↓ move over the entries, → opens a folder and ← closes
//! it (on an entry inside a folder it goes to the folder), ↩ opens a folder or shows a file in the viewer
//! and, on the head, the path menu. The list keeps the keyboard.

use gpui_kit::*;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_core::view::focus::list::Item;

use super::{Files, Line};
use crate::focus_keys::{Activate, key_menu_origin};
use crate::window::feature::{WalkFns, Walkable};

impl Files {
    /// The rows the keys can be on: the head (a band, so a ⇥ stop) and the entries.
    fn walkable(&self, session: i64) -> Vec<Walkable> {
        let Some(of) = self.of.get(&session) else { return Vec::new() };
        (of.list.items().iter().enumerate())
            .filter_map(|(row, line)| {
                let (level, fold, band) = match line {
                    Line::Head => (0, None, true),
                    Line::Row(entry) if entry.is_entry() => {
                        let fold = match entry.kind {
                            RowKind::Folder { open } => Some(!open),
                            _ => None,
                        };
                        (1 + entry.depth as u8, fold, false)
                    }
                    Line::Row(_) | Line::Spacer => return None,
                };
                Some(Walkable { row, item: Item { level, fold }, band })
            })
            .collect()
    }

    /// The line at `row` of the list.
    fn line(&self, session: i64, row: usize) -> Option<Line> {
        self.of.get(&session)?.list.items().get(row).cloned()
    }

    /// Makes `row` the selected one.
    pub(super) fn select(&mut self, session: i64, row: &Row, cx: &mut Context<Self>) {
        let of = self.of_mut(session);
        if of.selected.as_deref() != Some(row.path.as_path()) {
            of.selected = Some(row.path.clone());
            cx.notify();
        }
    }

    /// Opens a closed folder or closes an open one, and reads it when it was not read yet.
    pub(super) fn toggle(&mut self, session: i64, row: &Row, cx: &mut Context<Self>) {
        let of = self.of_mut(session);
        let needs_read = of.tree.toggle(&row.path);
        of.stale = true;
        if needs_read {
            self.read(session, row.path.clone(), cx);
        }
        cx.notify();
    }

    /// ↩ on a row: the path menu on the head, a folder opens or closes, a file opens in the viewer. With
    /// `following` (the viewer is up and the keys only moved onto the row) a file shows in it and nothing
    /// else happens: a folder is not opened by walking over it.
    fn press_row(
        &mut self,
        session: i64,
        row: usize,
        following: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match self.line(session, row) {
            Some(Line::Head) if following => {}
            Some(Line::Head) => {
                let Some(of) = self.of.get(&session) else { return true };
                let at = (of.list.state().bounds_for_item(row))
                    .map_or_else(|| of.head_at.below(), key_menu_origin);
                self.show_path_menu(session, at, window, cx);
            }
            Some(Line::Row(entry)) => match entry.kind {
                RowKind::File => self.show_file(session, &entry.path, cx),
                RowKind::Folder { .. } if !following => {
                    self.select(session, &entry, cx);
                    self.toggle(session, &entry, cx);
                }
                _ => self.select(session, &entry, cx),
            },
            _ => {}
        }
        true
    }

    /// → opens a folder, ← closes it.
    fn fold_row(&mut self, session: i64, row: usize, cx: &mut Context<Self>) {
        if let Some(Line::Row(entry)) = self.line(session, row)
            && entry.is_folder()
        {
            self.toggle(session, &entry, cx);
        }
    }

    fn reveal(&self, session: i64, row: usize) {
        if let Some(of) = self.of.get(&session) {
            of.list.reveal(row);
        }
    }

    /// The list, to scroll from outside.
    fn list_state(&self, session: i64) -> Option<ListState> {
        Some(self.of.get(&session)?.list.state())
    }
}

/// The Files tab's keys, over the entity.
pub(super) const WALK: WalkFns = WalkFns {
    walkable: |entity, facts, cx| {
        let files = super::def::typed(entity);
        facts.active().map(|active| files.read(cx).walkable(active.id)).unwrap_or_default()
    },
    press: |entity, _, session, row, _: Activate, following, window, cx| {
        super::def::typed(entity).update(cx, |files, cx| files.press_row(session, row, following, window, cx))
    },
    fold: |entity, _, session, row, cx| {
        super::def::typed(entity).update(cx, |files, cx| files.fold_row(session, row, cx));
    },
    copy: None,
    reveal: |entity, facts, row, cx| {
        if let Some(active) = facts.active() {
            super::def::typed(entity).read(cx).reveal(active.id, row);
        }
    },
    list: |entity, facts, cx| super::def::typed(entity).read(cx).list_state(facts.active()?.id),
};
