//! The terminal's contents as plain text: the screen (devctl, prompt grabbing)
//! and the scrollback saved for restoring a session later.

use wormhole_vt::{Cell, Flags, Line, Modes, RowExtras};

use super::restore::RESTORED_MARKER;
use super::screen::Screen;

/// One grid line as text, trailing blanks trimmed. `cells` is scratch for the copy.
fn line_text(screen: &Screen, line: i32, cells: &mut Vec<Cell>) -> String {
    screen.copy_row(Line(line), cells, None);
    let text: String = cells
        .iter()
        .filter(|cell| !cell.flags.contains(Flags::WIDE_CHAR_SPACER))
        .map(|cell| cell.c)
        .collect();
    text.trim_end().to_string()
}

/// The visible screen, one line per row.
pub(super) fn screen_text(screen: &Screen) -> String {
    let mut out = String::new();
    let mut cells = Vec::new();
    for line in 0..screen.screen_lines() as i32 {
        out.push_str(&line_text(screen, line, &mut cells));
        out.push('\n');
    }
    out
}

/// The last `max_lines` rows of history + screen, for restoring later, or
/// `None` while a full-screen program (vim, less, htop) runs: its alternate
/// screen has no history, and the engine keeps the real one out of reach until
/// it exits, so the copy saved before is the one to keep.
///
/// Soft-wrapped rows come back as one line (a restore re-wraps it to the new
/// width) and combining marks stay with their characters.
pub(super) fn scrollback_text(screen: &Screen, max_lines: usize) -> Option<String> {
    if screen.mode().contains(Modes::ALT_SCREEN) {
        return None;
    }
    let top = screen.topmost_line().0;
    let bottom = screen.bottommost_line().0;
    let start = (bottom - max_lines as i32 + 1).max(top);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut finish = |current: &mut String| {
        let line = current.trim_end();
        // Don't let restore markers pile up across restarts.
        if line != RESTORED_MARKER {
            lines.push(line.to_string());
        }
        current.clear();
    };
    let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
    for line in start..=bottom {
        screen.copy_row(Line(line), &mut cells, Some(&mut extras));
        for (col, cell) in cells.iter().enumerate() {
            if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            // An image's placeholders (and their marks) are not text to bring back: the
            // picture is gone with the process, and the cell shows as blank.
            if cell.c == super::images::PLACEHOLDER {
                current.push(' ');
                continue;
            }
            current.push(cell.c);
            current.extend(extras.zerowidth(col));
        }
        let wrapped = cells.last().is_some_and(|last| last.flags.contains(Flags::WRAPLINE));
        if !wrapped {
            finish(&mut current);
        }
    }
    if !current.is_empty() {
        finish(&mut current);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    Some(lines.join("\n"))
}

/// What's typed in an AI CLI's input box on the visible screen.
pub(super) fn grab_prompt(screen: &Screen) -> Option<String> {
    let mut cells = Vec::new();
    let rows: Vec<String> =
        (0..screen.screen_lines() as i32).map(|line| line_text(screen, line, &mut cells)).collect();
    prompt_in(&rows)
}

/// Heuristic: find the last line starting with a prompt marker (`>` / `❯`,
/// possibly inside a `│` box), then expand to the surrounding horizontal rules
/// that Claude Code / Codex draw around their input.
pub fn prompt_in(rows: &[String]) -> Option<String> {
    let strip_box = |s: &str| -> String {
        let s = s.trim_start().strip_prefix('│').unwrap_or(s.trim_start());
        let s = s.trim_end().strip_suffix('│').unwrap_or(s.trim_end());
        s.trim_end().to_string()
    };
    let prompt_body = |s: &str| -> Option<String> {
        let s = strip_box(s);
        let t = s.trim_start();
        ["> ", "❯ ", "›"]
            .iter()
            .find_map(|marker| t.strip_prefix(marker))
            .map(|rest| rest.trim_start().to_string())
            .or_else(|| (t == ">" || t == "❯").then(String::new))
    };
    let is_rule = |s: &str| {
        let t = s.trim();
        t.chars().count() >= 3
            && t.chars().all(|c| matches!(c, '─' | '━' | '═' | '╭' | '╮' | '╰' | '╯' | '-'))
    };

    let anchor = (0..rows.len()).rev().find(|&line| prompt_body(&rows[line]).is_some())?;
    let mut lines = vec![prompt_body(&rows[anchor])?];
    let mut line = anchor + 1;
    while line < rows.len() && line < anchor + 60 {
        let text = &rows[line];
        if is_rule(text) {
            break;
        }
        let body = strip_box(text);
        lines.push(body.strip_prefix("  ").unwrap_or(&body).to_string());
        line += 1;
    }
    let text = lines.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::{grab_prompt, prompt_in, screen_text, scrollback_text};
    use crate::testing::TestTerm;

    fn term(cols: u16, rows: u16, bytes: &[u8]) -> TestTerm {
        let mut term = TestTerm::new(cols, rows, 1000);
        term.feed(bytes);
        term
    }

    fn rows(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn screen_text_has_one_trimmed_line_per_row() {
        let term = term(10, 3, b"ab  \r\n\xe6\x97\xa5x");
        assert_eq!(screen_text(term.screen()), "ab\n日x\n\n");
    }

    #[test]
    fn scrollback_keeps_the_last_lines_and_drops_markers_and_trailing_blanks() {
        let mut bytes = Vec::new();
        for i in 0..8 {
            bytes.extend_from_slice(format!("line {i}\r\n").as_bytes());
        }
        bytes.extend_from_slice("──── restored ────\r\nafter\r\n\r\n".as_bytes());
        let term = term(20, 4, &bytes);
        assert_eq!(
            scrollback_text(term.screen(), 1000).as_deref(),
            Some("line 0\nline 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nafter")
        );
        // The window counts the blank rows under "after" (and the dropped marker).
        assert_eq!(scrollback_text(term.screen(), 4).as_deref(), Some("after"));
        assert_eq!(scrollback_text(term.screen(), 5).as_deref(), Some("line 7\nafter"));
    }

    #[test]
    fn a_save_in_the_alt_screen_keeps_the_history() {
        // vim, less, htop: the active grid is the alternate screen, which has no history.
        let mut bytes = Vec::new();
        for i in 0..50 {
            bytes.extend_from_slice(format!("shell line {i}\r\n").as_bytes());
        }
        bytes.extend_from_slice(b"\x1b[?1049hVIM SCREEN");
        let term = term(20, 5, &bytes);
        assert_eq!(scrollback_text(term.screen(), 1000), None);
    }

    #[test]
    fn wrapped_lines_restore_as_one() {
        let wrapped = term(10, 4, b"0123456789abcdef\r\nnext");
        assert_eq!(scrollback_text(wrapped.screen(), 1000).as_deref(), Some("0123456789abcdef\nnext"));
        // A space at the wrap point is part of the line.
        let spaced = term(5, 4, b"abcd efgh\r\n");
        assert_eq!(scrollback_text(spaced.screen(), 1000).as_deref(), Some("abcd efgh"));
    }

    #[test]
    fn combining_marks_survive_the_round_trip() {
        let term = term(10, 3, "cafe\u{301} ok".as_bytes());
        assert_eq!(scrollback_text(term.screen(), 1000).as_deref(), Some("cafe\u{301} ok"));
    }

    #[test]
    fn prompt_between_rules_is_grabbed_up_to_the_rule() {
        let screen = rows(&["previous output", "─────────", "> fix the", "  build", "─────────", "  ? help"]);
        assert_eq!(prompt_in(&screen).as_deref(), Some("fix the\nbuild"));
    }

    #[test]
    fn prompt_inside_a_box_loses_the_box() {
        let screen = rows(&["╭──────────────╮", "│ > fix the    │", "│   build      │", "╰──────────────╯"]);
        // Continuation lines keep what's left of the box's indent after two spaces.
        assert_eq!(prompt_in(&screen).as_deref(), Some("fix the\n build"));
    }

    #[test]
    fn the_last_prompt_on_screen_wins_and_empty_prompts_are_none() {
        assert_eq!(prompt_in(&rows(&["> old", "out", "❯ new", "───"])).as_deref(), Some("new"));
        assert_eq!(prompt_in(&rows(&["› codex prompt"])).as_deref(), Some("codex prompt"));
        assert_eq!(prompt_in(&rows(&["> ", "───"])), None);
        assert_eq!(prompt_in(&rows(&["no prompt here"])), None);
    }

    #[test]
    fn grab_prompt_reads_the_screen() {
        let term = term(20, 4, "> hello\r\n───\r\n".as_bytes());
        assert_eq!(grab_prompt(term.screen()).as_deref(), Some("hello"));
    }
}
