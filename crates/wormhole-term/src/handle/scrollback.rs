//! The screen and the history as plain text, and the handle that builds it from another thread.

use crate::Pty;

use wormhole_vt::Line;

use super::reader::ScreenReader;
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
        ScrollbackSource(self.reader())
    }
}

/// Builds a terminal's saved scrollback; `Send`, so a save can run on a
/// background thread (it locks the terminal only while copying text). A [`ScreenReader`] with the
/// two methods saving needs.
#[derive(Clone)]
pub struct ScrollbackSource(ScreenReader);

impl ScrollbackSource {
    /// See [`Pty::output_revision`].
    pub fn output_revision(&self) -> u64 {
        self.0.revision()
    }

    /// The last `max_lines` lines of history + screen.
    pub fn text(&self, max_lines: usize) -> Option<String> {
        self.0.text(max_lines)
    }

    /// The history and the screen copied for a search ([`find`]), with the output revision it
    /// is as of.
    pub fn grid_text(&self) -> (find::GridText, u64) {
        self.0.grid_text()
    }
}
