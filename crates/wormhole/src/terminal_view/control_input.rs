//! What the control line (`wh send-keys`, `wh mouse`) does to a terminal, and the handle it reads
//! one with: keys and pointer events encoded for the program's modes as they are when each goes
//! out, written like typing but not claimed by any selection or scroll position of the person's.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::ipc::control::MouseOp;
use wormhole_core::ipc::control::keys::Stroke;
use wormhole_core::ipc::control::mouse::ButtonName;
use wormhole_term::input::mouse::{self, LocatorPointer, MouseButton};
use wormhole_term::{KeySent, ScreenReader};

use super::TerminalView;

/// What became of a pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseSent {
    Sent,
    /// The program did not ask for the mouse (reports or the DEC locator), or what it asked for
    /// does not include this event.
    Unwanted,
    /// The row or column is outside the grid, which is this big.
    OutOfGrid {
        rows: usize,
        cols: usize,
    },
}

impl TerminalView {
    /// A handle that reads this terminal's screen from any thread, while its shell runs.
    pub fn reader(&self) -> Option<ScreenReader> {
        self.pty.as_ref().map(|pty| pty.reader())
    }

    /// Press `strokes`, in order, each encoded with the modes the program has when it goes out;
    /// how many were sent (a stroke the modes make mean nothing is not).
    pub fn send_strokes(&mut self, strokes: &[Stroke], cx: &mut Context<Self>) -> usize {
        let Some(pty) = &self.pty else { return 0 };
        let mut sent = 0;
        for stroke in strokes {
            let key = pty.send_key(|modes| Some(stroke.encode(modes)).filter(|bytes| !bytes.is_empty()));
            if matches!(key, KeySent::Sent { .. }) {
                sent += 1;
            }
        }
        if sent > 0 {
            // The echo is the person's typing, as far as the status heuristics go.
            self.last_input = Instant::now();
            self.app_selection.forget();
            self.forget_block_cursor();
            cx.notify();
        }
        sent
    }

    /// Send `op` to a program that asked for the mouse, at a row and column of the grid.
    pub fn send_mouse(&mut self, op: &MouseOp) -> MouseSent {
        let Some(pty) = &self.pty else { return MouseSent::Unwanted };
        let size = pty.size();
        let (rows, cols) = (usize::from(size.rows), usize::from(size.cols));
        let (row, col) = op.at();
        if row as usize >= rows || col as usize >= cols {
            return MouseSent::OutOfGrid { rows, cols };
        }
        let modes = pty.mode();
        if mouse::wants_mouse(modes) {
            let cell = (size.cell_width, size.cell_height);
            return if pty.write_encoded(|modes| op.encode(modes, cell)) {
                MouseSent::Sent
            } else {
                MouseSent::Unwanted
            };
        }
        if !mouse::wants_locator(modes) || op.is_wheel() {
            return MouseSent::Unwanted;
        }
        let at = LocatorPointer {
            col: col as usize,
            row: row as usize,
            x: (col as f32 * size.cell_width + size.cell_width / 2.0) as u32,
            y: (row as f32 * size.cell_height + size.cell_height / 2.0) as u32,
        };
        let button = |name: ButtonName| match name {
            ButtonName::Left => MouseButton::Left,
            ButtonName::Middle => MouseButton::Middle,
            ButtonName::Right => MouseButton::Right,
        };
        match op {
            MouseOp::Click(p) => {
                for _ in 0..p.count.max(1) {
                    pty.locator(Some((button(p.button), true)), at);
                    pty.locator(Some((button(p.button), false)), at);
                }
            }
            MouseOp::Press(p) => pty.locator(Some((button(p.button), true)), at),
            MouseOp::Release(p) => pty.locator(Some((button(p.button), false)), at),
            MouseOp::Move(_) => pty.locator(None, at),
            MouseOp::Scroll(_) => return MouseSent::Unwanted,
        }
        MouseSent::Sent
    }
}
