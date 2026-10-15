//! Unit tests for what the conformance suite (`tests/conformance`) does not say: the events and
//! replies in detail, the kitty keyboard flags, resizing, the snapshot, hostile input.

mod clusters;
mod events;
mod hostile;
mod keyboard;
mod rare;
mod replies;
mod resize;
mod selection;
mod snapshot;
mod tabs;
mod window;

use crate::parser::Parser;
use crate::term::Terminal;
use crate::types::Line;

/// A terminal and its parser.
pub(super) struct Rig {
    pub term: Terminal,
    parser: Parser,
}

impl Rig {
    pub fn new(cols: usize, rows: usize) -> Rig {
        Rig { term: Terminal::new(cols, rows, 100), parser: Parser::new() }
    }

    pub fn feed(&mut self, text: &str) -> &mut Rig {
        self.term.advance(&mut self.parser, text.as_bytes());
        self
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) -> &mut Rig {
        self.term.advance(&mut self.parser, bytes);
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

    pub fn events(&mut self) -> Vec<crate::term::Event> {
        self.term.drain_events().collect()
    }

    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.grid_cursor();
        (point.line.0 as usize, point.column.0)
    }
}
