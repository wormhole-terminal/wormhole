//! Replaying a previous session's text into a fresh terminal.

use super::screen::Screen;

/// The marker `replay` draws under replayed text, which `contents` leaves out of what it saves.
pub(super) const RESTORED_MARKER: &str = "──── restored ────";

/// Draw `text` dimmed, followed by a marker line, before the shell starts.
pub(super) fn replay(screen: &mut Screen, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let mut bytes = Vec::with_capacity(text.len() + 64);
    for line in text.lines() {
        bytes.extend_from_slice(b"\x1b[2m");
        bytes.extend_from_slice(line.as_bytes());
        bytes.extend_from_slice(b"\x1b[0m\r\n");
    }
    bytes.extend_from_slice(format!("\x1b[2m{}\x1b[0m\r\n", RESTORED_MARKER).as_bytes());
    screen.feed(&bytes);
}
