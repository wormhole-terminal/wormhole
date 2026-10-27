//! History under the keys: the commits and the open one's details and files, → ← to show or fold a
//! commit's files, ↩ / Space on a file opening its diff with the list keeping the keyboard, ↩ on the
//! details opening their menu (the copies, "Show more", the parents).

use gpui_kit::*;
use wormhole_core::view::focus::list::Item;

use super::{History, HistoryRow};
use crate::features::git::changes::Changes;
use crate::window::feature::WalkFns;
use crate::window::feature::Walkable;

impl History {
    /// The History rows the keys can be on: the commits, and the open one's files under it.
    fn walkable(&self, session: i64) -> Vec<Walkable> {
        let Some(history) = self.of.get(&session) else { return Vec::new() };
        let item = |level, fold| Item { level, fold };
        (history.list.items().iter().enumerate())
            .filter_map(|(row, line)| {
                let item = match line {
                    HistoryRow::Commit(ix) => {
                        let sha = history.commits.get(*ix).map(|c| c.sha.as_str());
                        item(0, Some(sha.is_some() && history.expanded.as_deref() == sha))
                    }
                    // ↩ on the details is "Show more" / "Show less".
                    HistoryRow::File(_) | HistoryRow::Details(_) => item(1, None),
                    HistoryRow::Head
                    | HistoryRow::Error
                    | HistoryRow::Note(_)
                    | HistoryRow::FilesError
                    | HistoryRow::Spacer => return None,
                };
                Some(Walkable { row, item, band: false })
            })
            .collect()
    }

    /// → / ← on a commit: show or fold its files.
    fn fold_row(&mut self, session: i64, row: usize, cx: &mut Context<Self>) {
        let Some(history) = self.of.get(&session) else { return };
        if let Some(HistoryRow::Commit(ix)) = history.list.items().get(row)
            && let Some(sha) = history.commits.get(*ix).map(|c| c.sha.clone())
        {
            self.toggle_commit(session, sha, cx);
        }
    }

    /// ↩ / Space on a commit folds it, on a file opens its diff (`following`: only a file's). The
    /// list keeps the keyboard.
    fn press_row(
        &mut self,
        changes: Option<Entity<Changes>>,
        session: i64,
        row: usize,
        following: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(history) = self.of.get(&session) else { return true };
        match history.list.items().get(row) {
            Some(HistoryRow::Commit(_)) if !following => self.fold_row(session, row, cx),
            Some(HistoryRow::Details(_)) if !following => {
                self.show_details_menu_at_row(session, row, window, cx);
            }
            Some(HistoryRow::File(ix)) => {
                let commit =
                    history.expanded.as_ref().and_then(|sha| history.commits.iter().find(|c| &c.sha == sha));
                let file =
                    history.expanded.as_ref().and_then(|sha| history.loaded(sha)).and_then(|f| f.get(*ix));
                if let (Some(commit), Some(file), Some(changes)) = (commit.cloned(), file.cloned(), changes) {
                    self.open_commit_file(session, commit, file, &changes, cx);
                }
            }
            _ => {}
        }
        true
    }

    fn reveal(&self, session: i64, row: usize) {
        if let Some(history) = self.of.get(&session) {
            history.list.reveal(row);
        }
    }
}

/// The History tab's keys, over the entity.
pub(super) const WALK: WalkFns = WalkFns {
    walkable: |entity, facts, cx| {
        let history = super::def::typed(entity);
        facts.active().map(|active| history.read(cx).walkable(active.id)).unwrap_or_default()
    },
    press: |entity, facts, session, row, _, following, window, cx| {
        let changes = facts.feature::<Changes>();
        super::def::typed(entity)
            .update(cx, |history, cx| history.press_row(changes, session, row, following, window, cx))
    },
    fold: |entity, _, session, row, cx| {
        super::def::typed(entity).update(cx, |history, cx| history.fold_row(session, row, cx));
    },
    copy: Some(|entity, _, session, row, window, cx| {
        super::def::typed(entity).update(cx, |history, cx| history.copy_row(session, row, window, cx))
    }),
    reveal: |entity, facts, row, cx| {
        if let Some(active) = facts.active() {
            super::def::typed(entity).read(cx).reveal(active.id, row);
        }
    },
    list: |entity, facts, cx| {
        Some(super::def::typed(entity).read(cx).of.get(&facts.active()?.id)?.list.state())
    },
};
