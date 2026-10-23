//! The terminal Wormhole's own programs run in, as the first-run welcome uses it: whether there is
//! one, raw mode, its size and waiting for a key.

use std::io::IsTerminal as _;

use super::unix::tty as os;
pub use super::unix::tty::RawMode;

/// Whether the program is attached to a terminal at both ends.
pub fn is_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// The terminal's size in cells (columns, rows), when it says. A terminal that reports no size at all
/// (zero) is treated as unknown.
pub fn size() -> Option<(usize, usize)> {
    os::size()
}

/// Read what has been typed, waiting at most `wait_ms`; empty when nothing was, `None` at the end of
/// the input (the terminal went away).
pub fn read_input(buf: &mut [u8], wait_ms: i32) -> Option<usize> {
    os::read_input(buf, wait_ms)
}
