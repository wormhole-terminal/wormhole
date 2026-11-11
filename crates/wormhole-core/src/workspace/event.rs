//! What the model tells a shell after a change. Facts, not instructions to draw: a shell pulls the
//! snapshot a revision says moved. `Opened` and `Closed` are the two the shell must act on, by
//! starting or ending a program.

use std::path::PathBuf;

use crate::view::panes::DropError;

/// Why an action changed nothing, as data (the shell words it in the person's language).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A terminal or group the action names is not here.
    Unknown,
    /// A pane would end up under the minimum size.
    TooSmall,
    /// Nothing would change.
    Unchanged,
}

impl From<DropError> for Refusal {
    fn from(error: DropError) -> Self {
        match error {
            DropError::Unknown => Self::Unknown,
            DropError::TooSmall => Self::TooSmall,
            DropError::Unchanged => Self::Unchanged,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A terminal was made: start a program for it, in `cwd` when there is one.
    Opened { id: i64, cwd: Option<PathBuf> },
    /// A terminal was closed: end its program.
    Closed { id: i64 },
    /// A terminal came from another window, its program running: nothing to start, only a view to make.
    Adopted { id: i64 },
    /// The selection moved (`None`: nothing is selected).
    Selected { id: Option<i64> },
    /// A row was moved by a drop or a split's pane joined it: a shell may animate it.
    Moved { id: i64 },
    /// The sidebar's rows, groups or numbers changed: pull `sidebar` (and `terminals`).
    Listing,
    /// The splits or the panes' focus changed: pull `panes`.
    Layout,
    /// The sidebar's or panel's visibility, the panel's tab, the search or the overlay changed.
    Chrome,
    /// A terminal's "Forget text" was set: erase what was saved of its screen.
    SavedTextForgotten { id: i64 },
    /// A group is to move to a window of its own: `terminals` are its terminals and the rest of
    /// every split they are in, in the window's order. Nothing moves in this model.
    GroupToWindow { group: u32, terminals: Vec<i64> },
    /// The action changed nothing.
    Refused(Refusal),
}

/// How often each part of the snapshot changed since the model was made: a view compares numbers,
/// never values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revisions {
    pub sidebar: u64,
    pub layout: u64,
    pub chrome: u64,
    /// The list of every terminal (`terminals_snapshot`): moves with the sidebar and the panes.
    pub terminals: u64,
}

/// The events of one change, coalesced: a part that changed twice is told once, the selection last
/// wins.
#[derive(Debug, Default)]
pub(super) struct Out {
    events: Vec<Event>,
}

impl Out {
    pub(super) fn push(&mut self, event: Event) {
        match event {
            Event::Selected { .. } => {
                self.events.retain(|known| !matches!(known, Event::Selected { .. }));
                self.events.push(event);
            }
            Event::Listing | Event::Layout | Event::Chrome if self.events.contains(&event) => {}
            _ => self.events.push(event),
        }
    }

    pub(super) fn take(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}
