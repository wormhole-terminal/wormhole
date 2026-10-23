//! The screen and the history as plain text, and the handle that builds it from another thread.

use crate::Pty;
use std::sync::Arc;

use wormhole_vt::Line;

use crate::events::Shared;
use crate::pty::FairLock;
use crate::screen::Screen;
use crate::{blocks, contents, find};

impl Pty {
    /// The text of lines `first..=last`, trailing blanks trimmed (a block's output to copy).
    pub fn lines_text(&self, first: Line, last: Line) -> String {
        blocks::lines_text(&self.term.lock(), first, last)
    }

    /// Bumped whenever the program printed: compare two readings to tell
    /// whether the terminal changed in between.
    pub fn output_revision(&self) -> u64 {
        self.shared.revision()
    }

    /// The visible screen as plain text, one line per row.
    pub fn screen_text(&self) -> String {
        contents::screen_text(&self.term.lock())
    }

    /// What's typed in an AI CLI's input box on the screen.
    pub fn grab_prompt(&self) -> Option<String> {
        contents::grab_prompt(&self.term.lock())
    }

    /// A handle that builds the scrollback text, also from another thread.
    pub fn scrollback(&self) -> ScrollbackSource {
        ScrollbackSource { term: self.term.clone(), shared: self.shared.clone() }
    }
}

/// Builds a terminal's saved scrollback; `Send`, so a save can run on a
/// background thread (it locks the terminal only while copying text).
#[derive(Clone)]
pub struct ScrollbackSource {
    term: Arc<FairLock<Screen>>,
    shared: Arc<Shared>,
}

impl ScrollbackSource {
    /// See [`Pty::output_revision`].
    pub fn output_revision(&self) -> u64 {
        self.shared.revision()
    }

    /// The last `max_lines` lines of history + screen.
    pub fn text(&self, max_lines: usize) -> Option<String> {
        contents::scrollback_text(&self.term.lock(), max_lines)
    }

    /// The history and the screen copied for a search ([`find`]), with the output revision it
    /// is as of.
    pub fn grid_text(&self) -> (find::GridText, u64) {
        let term = self.term.lock();
        (find::GridText::copy(&term), self.shared.revision())
    }
}
