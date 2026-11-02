//! Replaying a previous session's text into a fresh terminal.

use super::screen::Screen;

/// The marker `replay` draws under replayed text, which `contents` leaves out of what it saves.
pub(crate) const RESTORED_MARKER: &str = "──── restored ────";

/// Whether `line` is a "restored" divider, with a time or without: the screen's text and a checkpoint leave
/// them out, so they do not pile up across restarts.
pub(crate) fn is_marker(line: &str) -> bool {
    let line = line.trim();
    line == RESTORED_MARKER || (line.starts_with("──── restored ") && line.ends_with(" ────"))
}

/// What a saved line may hold: text, and tabs. Saved text came from a screen, and a file the person can edit
/// (or a repository's hostile one) may hold anything: an escape sequence in it would be run as if a program
/// had printed it, to answer a query into the PTY, set the title or write the clipboard.
fn plain(line: &str) -> String {
    line.chars().filter(|c| *c == '\t' || !c.is_control()).collect()
}

/// Draw `text` dimmed, followed by a marker line, before the shell starts. Nothing in it acts: control
/// characters are dropped, and what the engine queued is dropped after.
pub(crate) fn replay(screen: &mut Screen, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let mut bytes = Vec::with_capacity(text.len() + 64);
    for line in text.lines() {
        bytes.extend_from_slice(b"\x1b[2m");
        bytes.extend_from_slice(plain(line).as_bytes());
        bytes.extend_from_slice(b"\x1b[0m\r\n");
    }
    bytes.extend_from_slice(format!("\x1b[2m{}\x1b[0m\r\n", RESTORED_MARKER).as_bytes());
    screen.feed(&bytes);
    // Defence in depth: whatever got through, no reply is written to the PTY and no event is heard.
    screen.discard_output();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GridSize;

    fn screen() -> Screen {
        Screen::new(GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 }, 100)
    }

    /// A seed that asks the terminal things (a cursor position report, a clipboard read) and sets the title
    /// writes nothing to the PTY and raises no event: it is drawn as text.
    #[test]
    fn a_seed_with_queries_and_escapes_acts_on_nothing() {
        let mut screen = screen();
        replay(&mut screen, "before\n\x1b[6n\x1b]52;c;?\x07\x1b]0;hijacked\x07\x1b[5nafter\x07\n");
        assert!(screen.queued_replies().is_empty(), "no reply for the program");
        assert!(screen.drain_events().is_empty(), "no event for the app");
        let drawn = super::super::contents::screen_text(&screen);
        assert!(drawn.contains("before") && drawn.contains("after") && drawn.contains("restored"), "{drawn}");
        assert!(!drawn.contains('\u{1b}') && !drawn.contains("hijacked\u{7}"), "{drawn}");
    }

    /// Even a sequence that got through is dropped with what it queued.
    #[test]
    fn what_the_engine_queued_is_dropped() {
        let mut screen = screen();
        screen.feed(b"\x1b[6n\x1b]52;c;?\x07\x1b]0;a title\x07");
        assert!(
            !screen.queued_replies().is_empty() || !screen.drain_events().is_empty(),
            "the engine did queue"
        );
        screen.feed(b"\x1b[6n\x1b]52;c;?\x07\x1b]0;a title\x07");
        screen.discard_output();
        assert!(screen.queued_replies().is_empty());
        assert!(screen.drain_events().is_empty());
    }

    #[test]
    fn dividers_with_and_without_a_time_are_markers() {
        assert!(is_marker(RESTORED_MARKER));
        assert!(is_marker("──── restored 2026-11-02 14:03 ────"));
        assert!(!is_marker("restored"));
        assert!(!is_marker("──── restored"));
    }

    #[test]
    fn tabs_stay_and_other_control_characters_go() {
        assert_eq!(plain("a\tb\u{1b}[31mc\u{7}d\u{9b}e"), "a\tb[31mcde");
    }
}
