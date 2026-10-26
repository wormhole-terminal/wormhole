//! A group folding and opening: what the list needs to know that a change of its rows is a fold and
//! not terminals opening or closing, and the fade-in of the folded head's summary (its tiles and count)
//! that some folds end with. The person's toggle is noted (`FoldNote`) as it is made and read by the
//! next change of the list; the rows' poses are core's (`pose::fold`), drawn by `row` and `leaving`.

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::view::motion::FoldMotion;
use wormhole_core::view::motion::pose::{self, Phase};

use crate::sidebar::Sidebar;
use crate::style::SummaryFade;

/// A group the person just folded (`folding`) or opened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FoldNote {
    pub(super) group: u32,
    pub(super) folding: bool,
}

impl FoldNote {
    /// Whether a row of `group` that goes (`Exit`) or comes (`Enter`) is one of this fold's: folding
    /// takes the group's rows from the list, opening brings them.
    pub(super) fn owns(&self, group: Option<u32>, phase: Phase) -> bool {
        group == Some(self.group) && (phase == Phase::Exit) == self.folding
    }
}

/// A folded group's head that is waiting for its rows to be gone to show what it holds.
struct Summary {
    group: u32,
    motion: FoldMotion,
    rows: usize,
    batch: u64,
}

/// The heads that fade their summary in.
#[derive(Default)]
pub(super) struct Summaries(Vec<Summary>);

impl Summaries {
    pub(super) fn push(&mut self, group: u32, motion: FoldMotion, rows: usize, batch: u64) {
        self.0.retain(|summary| summary.group != group);
        self.0.push(Summary { group, motion, rows, batch });
    }

    /// The summary of change `batch` is up: the head shows it whole.
    pub(super) fn retire(&mut self, batch: u64) -> bool {
        let before = self.0.len();
        self.0.retain(|summary| summary.batch != batch);
        self.0.len() != before
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    /// Whether any head is waiting to show its summary.
    #[cfg(test)]
    pub(super) fn is_up(&self) -> bool {
        !self.0.is_empty()
    }

    /// How long the summary of a fold of `rows` rows takes to be up.
    pub(super) fn duration(motion: FoldMotion, rows: usize) -> Duration {
        pose::fold::summary_duration(motion, rows)
    }
}

impl Sidebar {
    /// The fade of the summary on group `group`'s head, while it has one to do.
    pub(in crate::sidebar) fn summary_fade(&self, group: u32) -> Option<SummaryFade> {
        let summary = self.motion.summaries.0.iter().find(|summary| summary.group == group)?;
        let (motion, rows) = (summary.motion, summary.rows);
        let total = Summaries::duration(motion, rows);
        Some(SummaryFade::Over {
            id: ElementId::from(("head-summary", u64::from(group) << 32 | summary.batch)),
            over: total,
            opacity: Box::new(move |t| pose::fold::summary_opacity(motion, rows, total.mul_f32(t))),
        })
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::view::motion::pose::Phase;

    use super::FoldNote;

    #[test]
    fn folding_owns_the_rows_that_go_and_opening_the_rows_that_come() {
        let folding = FoldNote { group: 3, folding: true };
        assert!(folding.owns(Some(3), Phase::Exit));
        assert!(!folding.owns(Some(3), Phase::Enter), "a row that comes while a group folds is a terminal");
        assert!(!folding.owns(Some(4), Phase::Exit), "another group's row is not this fold's");
        assert!(!folding.owns(None, Phase::Exit));
        let opening = FoldNote { group: 3, folding: false };
        assert!(opening.owns(Some(3), Phase::Enter));
        assert!(!opening.owns(Some(3), Phase::Exit));
    }
}
