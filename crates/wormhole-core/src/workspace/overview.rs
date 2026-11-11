//! What a program that controls the app (the control socket's `list`) is told of the splits: which
//! terminals each shows and which has the keyboard. The panes' own snapshot is only of the selected split.

use crate::view::sidebar::SplitId;

use super::model::WorkspaceModel;

/// One split as `list` tells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitOverview {
    pub id: SplitId,
    /// Its terminals in reading order (left to right, then top to bottom).
    pub terminals: Vec<i64>,
    /// The one with the keyboard when the split is shown.
    pub focused: i64,
}

impl WorkspaceModel {
    /// Every split of the window, in the order they were made.
    pub fn splits_overview(&self) -> Vec<SplitOverview> {
        self.splits
            .iter()
            .map(|split| SplitOverview {
                id: split.id,
                terminals: split.layout.panes(),
                focused: split.layout.focused(),
            })
            .collect()
    }
}
