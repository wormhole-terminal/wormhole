//! Replaying a previous session's text into a fresh terminal.

use alacritty_terminal::event::EventListener;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::Processor;

/// Draw `text` dimmed, followed by a marker line, before the shell starts.
pub(super) fn replay<T: EventListener>(term: &mut Term<T>, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let mut bytes = Vec::with_capacity(text.len() + 64);
    for line in text.lines() {
        bytes.extend_from_slice(b"\x1b[2m");
        bytes.extend_from_slice(line.as_bytes());
        bytes.extend_from_slice(b"\x1b[0m\r\n");
    }
    bytes.extend_from_slice(format!("\x1b[2m{}\x1b[0m\r\n", super::text::RESTORED_MARKER).as_bytes());
    let mut processor: Processor = Processor::new();
    processor.advance(term, &bytes);
}
