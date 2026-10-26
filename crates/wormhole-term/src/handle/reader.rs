//! A terminal's screen read from another thread: the handle the control socket's reads, the
//! output worker and the saving of scrollback use, so none of them goes through the UI.
//!
//! A [`ScreenReader`] holds no lock between calls; each method takes the screen's lock once per
//! chunk of rows and copies, never builds strings or waits for anything while holding it (the
//! lock is the fair one the PTY thread parses under, so a reader waits for at most the holder it
//! found). Hosted terminals read their replica, which is what the window draws.

use std::sync::{Arc, Weak};

use crate::events::Shared;
use crate::pty::FairLock;
use crate::screen::Screen;
use crate::{Pty, contents, find};

impl Pty {
    /// A handle that reads the screen from any thread (see [`ScreenReader`]).
    pub fn reader(&self) -> ScreenReader {
        ScreenReader { term: self.term.clone(), shared: self.shared.clone() }
    }
}

/// Reads a terminal's screen, history and command blocks; `Send` and `Sync`.
#[derive(Clone)]
pub struct ScreenReader {
    pub(crate) term: Arc<FairLock<Screen>>,
    pub(crate) shared: Arc<Shared>,
}

/// A [`ScreenReader`] that does not keep a closed terminal's screen alive.
#[derive(Clone)]
pub struct WeakScreenReader {
    term: Weak<FairLock<Screen>>,
    shared: Weak<Shared>,
}

impl ScreenReader {
    /// A reader that goes away with the terminal.
    pub fn downgrade(&self) -> WeakScreenReader {
        WeakScreenReader { term: Arc::downgrade(&self.term), shared: Arc::downgrade(&self.shared) }
    }

    /// Bumped whenever the program printed: compare two readings to tell whether the terminal
    /// changed in between.
    pub fn revision(&self) -> u64 {
        self.shared.revision()
    }

    /// The screen's size as `(rows, columns)`.
    pub fn size(&self) -> (usize, usize) {
        let term = self.term.lock();
        (term.screen_lines(), term.columns())
    }

    /// The last `max_lines` lines of history + screen as saved text (see
    /// [`contents::scrollback_text`]); `None` while a full-screen program has the screen.
    pub fn text(&self, max_lines: usize) -> Option<String> {
        contents::scrollback_text(&self.term.lock(), max_lines)
    }

    /// The history and the screen copied for a search ([`find`]), with the output revision it is
    /// as of.
    pub fn grid_text(&self) -> (find::GridText, u64) {
        let term = self.term.lock();
        (find::GridText::copy(&term), self.shared.revision())
    }

    /// A reader over a screen of a test's own.
    #[cfg(test)]
    pub(crate) fn of(screen: Screen) -> Self {
        Self { term: Arc::new(FairLock::new(screen)), shared: Arc::default() }
    }

    /// Print `bytes` into the screen as a program would (tests).
    #[cfg(test)]
    pub(crate) fn feed(&self, bytes: &[u8]) {
        self.term.lock().feed(bytes);
    }
}

impl WeakScreenReader {
    /// The reader, while the terminal is open.
    pub fn upgrade(&self) -> Option<ScreenReader> {
        Some(ScreenReader { term: self.term.upgrade()?, shared: self.shared.upgrade()? })
    }
}
