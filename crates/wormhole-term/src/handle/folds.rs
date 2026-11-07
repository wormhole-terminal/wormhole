//! The folds of a terminal's history (`crate::folds`), asked for by the app.

use wormhole_vt::Line;

use crate::folds::{Refused, Request};
use crate::{BlockState, Pty, blocks};

impl Pty {
    /// Fold the output of the finished command block `id` into one marker row. Only what the rules in
    /// `folds` allow is folded (the live screen is never); the result is the fold's id.
    pub fn fold_block_output(&self, id: u64) -> Result<u64, Refused> {
        let mut term = self.term.lock();
        let block = blocks::block(&term, id, &self.shared.commands()).ok_or(Refused::Unfinished)?;
        let (Some((first, last)), BlockState::Finished { .. }) = (block.output_lines(), block.state) else {
            return Err(Refused::Unfinished);
        };
        term.fold_add(first, last, Request { finished: true, failed: block.state.failed() })
    }

    /// Open or close fold `id`; whether it exists.
    pub fn fold_toggle(&self, id: u64) -> bool {
        self.term.lock().fold_toggle(id)
    }

    /// Open the fold hiding `line`, if there is one (find stepping to a match in it); whether one opened.
    pub fn fold_expand_at(&self, line: Line) -> bool {
        self.term.lock().fold_expand_at(line)
    }

    /// Say whether the terminal needs the person (see `folds`): it opens the recent folds and refuses
    /// new ones while it does.
    pub fn set_fold_needs_you(&self, needs_you: bool) {
        self.term.lock().fold_set_needs_you(needs_you);
    }
}
