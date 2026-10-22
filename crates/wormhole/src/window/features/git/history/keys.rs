//! History under the keys: the commits and the open one's files, → ← to show or fold a
//! commit's files, ↩ / Space on a file opening its diff with the list keeping the keyboard.

use gpui_kit::*;
use wormhole_core::view::focus::list::Item;

use super::HistoryRow;
use crate::workspace::Workspace;
use crate::workspace::focus::Walkable;

impl Workspace {
    /// The History rows the keys can be on: the commits, and the open one's files under it.
    pub(in crate::window) fn history_walkable(&self) -> Vec<Walkable> {
        let history = &self.active_session().history;
        let item = |level, fold| Item { level, fold };
        (history.list.items().iter().enumerate())
            .filter_map(|(row, line)| {
                let item = match line {
                    HistoryRow::Commit(ix) => {
                        let sha = history.commits.get(*ix).map(|c| c.sha.as_str());
                        item(0, Some(sha.is_some() && history.expanded.as_deref() == sha))
                    }
                    HistoryRow::File(_) => item(1, None),
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
    pub(in crate::window) fn history_fold(&mut self, row: usize, cx: &mut Context<Self>) {
        let history = &self.active_session().history;
        if let Some(HistoryRow::Commit(ix)) = history.list.items().get(row)
            && let Some(sha) = history.commits.get(*ix).map(|c| c.sha.clone())
        {
            self.toggle_commit(sha, cx);
        }
    }

    /// ↩ / Space on a commit folds it, on a file opens its diff (`following`: only a file's). The
    /// list keeps the keyboard.
    pub(in crate::window) fn history_press(
        &mut self,
        row: usize,
        following: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let history = &self.active_session().history;
        match history.list.items().get(row) {
            Some(HistoryRow::Commit(_)) if !following => self.history_fold(row, cx),
            Some(HistoryRow::File(ix)) => {
                let commit =
                    history.expanded.as_ref().and_then(|sha| history.commits.iter().find(|c| &c.sha == sha));
                let file =
                    history.expanded.as_ref().and_then(|sha| history.loaded(sha)).and_then(|f| f.get(*ix));
                if let (Some(commit), Some(file)) = (commit.cloned(), file.cloned()) {
                    self.open_commit_file(commit, file, cx);
                }
            }
            _ => {}
        }
        true
    }

    pub(in crate::window) fn history_reveal(&self, row: usize) {
        self.active_session().history.list.reveal(row);
    }
}
