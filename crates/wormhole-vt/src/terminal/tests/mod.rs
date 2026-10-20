//! Unit tests for what the conformance suite (`tests/conformance`) does not say: the events and
//! replies in detail, the kitty keyboard flags, resizing, the snapshot, hostile input.

mod clusters;
mod events;
mod hostile;
mod keyboard;
mod passthrough;
mod rare;
mod replies;
mod resize;
mod selection;
mod snapshot;
mod tabs;
mod window;

use crate::terminal::Terminal;
use crate::types::Line;

/// A terminal and the helpers the tests read it with.
pub(super) struct Rig {
    pub term: Terminal,
}

impl Rig {
    pub fn new(cols: usize, rows: usize) -> Rig {
        Rig::with_scrollback(cols, rows, 100)
    }

    pub fn with_scrollback(cols: usize, rows: usize, lines: usize) -> Rig {
        Rig { term: Terminal::builder().size(cols, rows).scrollback(lines).build() }
    }

    pub fn feed(&mut self, text: &str) -> &mut Rig {
        self.term.feed(text.as_bytes());
        self
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) -> &mut Rig {
        self.term.feed(bytes);
        self
    }

    /// Row `y` of the screen without trailing blanks.
    pub fn row(&self, y: usize) -> String {
        self.term.row_text(Line(y as i32), true)
    }

    /// The replies since the last call, as text.
    pub fn replies(&mut self) -> String {
        String::from_utf8_lossy(&self.term.take_replies()).into_owned()
    }

    pub fn events(&mut self) -> Vec<crate::terminal::Event> {
        self.term.events().collect()
    }

    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.cursor_point();
        (point.line.0 as usize, point.column.0)
    }
}
