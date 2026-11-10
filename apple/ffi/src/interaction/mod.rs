//! What a person does to a terminal besides typing a key: the pointer, the wheel, selecting, links,
//! paste, focus. The decisions (who gets the pointer, what a wheel step means in a full-screen
//! program, what a click count selects) are the app's behaviour, written once here over
//! `wormhole-term`'s encoders so that the Swift view only translates `NSEvent`s into these calls.
//!
//! Positions are viewport cells (0-based, row 0 at the top of what the frame shows); this side turns
//! them into grid lines with the display offset of the last frame, which is what the person saw.

mod app_selection;
mod links;
mod pointer;
mod selection;

use std::sync::{Mutex, PoisonError};

use wormhole_term::input::app_selection::AppSelection;
use wormhole_term::{Pty, ScreenSnapshot};

use crate::terminal::Terminal;

pub use links::{LinkHit, OpenTarget};
pub use pointer::{PointerAction, PointerButton, PointerEvent, WheelStep};
pub use selection::{CellSide, RowSpan, SelectionKind};

/// The state the calls below keep between calls: nothing the engine holds.
#[derive(Default)]
pub(crate) struct Interaction {
    pointer: Mutex<pointer::Memory>,
    /// A selection a full-screen program drew itself, for ⌘V over it.
    app_selection: Mutex<AppSelection>,
}

impl Interaction {
    pub(crate) fn pointer(&self) -> std::sync::MutexGuard<'_, pointer::Memory> {
        self.pointer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn app_selection(&self) -> std::sync::MutexGuard<'_, AppSelection> {
        self.app_selection.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Terminal {
    /// Run `f` on the PTY, `None` after `close`.
    pub(crate) fn with_pty<R>(&self, f: impl FnOnce(&Pty) -> R) -> Option<R> {
        self.pty.read().unwrap_or_else(PoisonError::into_inner).as_ref().map(f)
    }

    /// Run `f` on the screen as the last frame left it (selection, display offset, rows).
    pub(crate) fn with_snapshot<R>(&self, f: impl FnOnce(&ScreenSnapshot) -> R) -> R {
        f(&self.snapshot.lock().unwrap_or_else(PoisonError::into_inner))
    }
}
