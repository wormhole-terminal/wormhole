//! The size of a terminal and what the engine is told of the window around it.

use crate::Pty;
use crate::{Backend, GridSize};
use wormhole_vt::window::WindowGeometry;

impl Pty {
    pub fn resize(&mut self, size: GridSize) {
        if size == self.size {
            return;
        }
        self.size = size;
        match &self.io {
            Backend::Local(io) => {
                let replies = self.term.lock().resize(size);
                io.resize(size.window_size());
                // After the PTY has its new size: a program told in band that the window changed
                // reads the size from the report and finds it the same everywhere.
                if !replies.is_empty() && self.is_alive() {
                    self.shared.reply(replies);
                }
            }
            // The screen follows when the host says it applied the size, in the stream between
            // exactly the bytes it was applied between.
            Backend::Hosted(host) => host.resize(size),
        }
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    /// Tell the engine where the window is: a program asks for it, its size and the screen's
    /// (XTWINOPS 11, 13, 14, 15, 19). The latest is kept.
    pub fn set_window(&self, geometry: WindowGeometry) {
        self.term.lock().set_window(geometry);
    }

    /// Whether programs in this terminal may act on the window (`allow_window_ops`).
    pub fn window_ops_allowed(&self) -> bool {
        self.term.lock().window_ops_allowed()
    }

    /// Take the engine back to the size this terminal has. A program that asked for another size
    /// (XTWINOPS 8, DECSLPP, DECCOLM) was given it at once; when the window could not follow, or
    /// finished following, this makes the two agree.
    pub fn sync_size(&self) {
        match &self.io {
            Backend::Local(_) => {
                let replies = self.term.lock().resize(self.size);
                if !replies.is_empty() && self.is_alive() {
                    self.shared.reply(replies);
                }
            }
            // The host's engine took the program's size: ask it for this one again.
            Backend::Hosted(host) => host.resize(self.size),
        }
    }
}
