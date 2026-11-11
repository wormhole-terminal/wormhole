//! What a terminal keeps of its folds: two pins to each fold's rows and whether it is open. No code
//! beyond that, so `screen`, which holds it, and `folds`, which reads the screen through it, do not
//! depend on each other.

use std::sync::Arc;

use wormhole_vt::{Pin, Point};

/// Who made a fold, when it was a Probe's hook and not the person: what its marker says and what takes
/// it away when the Probe is turned off.
#[derive(Debug, PartialEq, Eq)]
pub struct FoldOwner {
    /// The Probe's id.
    pub probe: String,
    /// The Probe's name, for the marker's accessibility label and tooltip.
    pub name: String,
    /// The manifest's fixed label for the marker: never text of the output.
    pub label: String,
}

pub(crate) struct Fold {
    pub(crate) id: u64,
    pub(crate) first: Pin,
    pub(crate) last: Pin,
    pub(crate) expanded: bool,
    pub(crate) failed: bool,
    pub(crate) owner: Option<Arc<FoldOwner>>,
}

/// A terminal's folds, in the order they were made.
#[derive(Default)]
pub(crate) struct FoldSet {
    pub(crate) folds: Vec<Fold>,
    pub(crate) next_id: u64,
    /// The terminal needs the person (an input from the host: status, see `folds`).
    pub(crate) needs_you: bool,
    /// A program is reading a secret (a password prompt: echo is off).
    pub(crate) secure: bool,
    /// The person never wants lines hidden: the view shows everything and nothing new is folded.
    pub(crate) off: bool,
    /// Where the selection being dragged began, to tell which way it crosses a marker.
    pub(crate) anchor: Option<Point>,
}

impl FoldSet {
    pub(crate) fn is_empty(&self) -> bool {
        self.folds.is_empty()
    }

    /// Something needs the person or a secret is being typed: the recent folds show open.
    pub(crate) fn guarded(&self) -> bool {
        self.needs_you || self.secure
    }
}
