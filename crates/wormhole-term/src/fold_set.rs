//! What a terminal keeps of its folds: two pins to each fold's rows and whether it is open. No code
//! beyond that, so `screen`, which holds it, and `folds`, which reads the screen through it, do not
//! depend on each other.

use wormhole_vt::Pin;

pub(crate) struct Fold {
    pub(crate) id: u64,
    pub(crate) first: Pin,
    pub(crate) last: Pin,
    pub(crate) expanded: bool,
    pub(crate) failed: bool,
}

/// A terminal's folds, in the order they were made.
#[derive(Default)]
pub(crate) struct FoldSet {
    pub(crate) folds: Vec<Fold>,
    pub(crate) next_id: u64,
    pub(crate) needs_you: bool,
}

impl FoldSet {
    pub(crate) fn is_empty(&self) -> bool {
        self.folds.is_empty()
    }
}
