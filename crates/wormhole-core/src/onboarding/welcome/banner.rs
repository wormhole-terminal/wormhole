//! What the terminal shows when the welcome hands over to the shell: a compact banner, three lines
//! under a blank one, kept in the scrollback like anything else the shell printed. The mark sits where
//! the welcome's mark did, so the hand-over doesn't move it. The keys are the keymap's, never typed
//! in here, so a rebound key shows as it is bound.

use super::ansi::{Style, color, text_width};
use super::art::{MARGIN, MARK_CELL};
use crate::keys::keymap::{self, ActionId};

/// The keys the banner names, with what they do.
const HINTS: [(ActionId, &str); 3] = [
    (ActionId::NewTerminal, "new terminal"),
    (ActionId::CommandPalette, "command palette"),
    (ActionId::KeyboardShortcuts, "keyboard shortcuts"),
];

/// The banner's lines as plain text and as styled spans, for a screen `cols` wide: hints that would
/// not fit are left out rather than cut.
pub(super) fn lines(cols: usize) -> Vec<Vec<(String, Style)>> {
    let bold = Style::PLAIN.bold();
    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
    // Blank rows so the mark is at MARK_CELL's row; then the mark, `>_`, at its column.
    for _ in 0..MARK_CELL.0 {
        lines.push(Vec::new());
    }
    let indent = " ".repeat(MARK_CELL.1);
    lines.push(vec![
        (indent.clone(), Style::PLAIN),
        (">_".to_string(), Style::PLAIN.fg(color::BRIGHT_WHITE).bold()),
        (" ".to_string(), Style::PLAIN),
        ("Wormhole".to_string(), bold),
    ]);
    let mut hints: Vec<(String, Style)> = vec![(" ".repeat(MARGIN), Style::PLAIN)];
    let mut used = MARGIN;
    for (action, what) in HINTS {
        let key = keymap::glyphs(action);
        if key.is_empty() {
            continue;
        }
        let width = text_width(&key) + 1 + text_width(what) + 3;
        if used + width > cols.saturating_sub(MARGIN) && used > MARGIN {
            break;
        }
        if used > MARGIN {
            hints.push(("   ".to_string(), Style::PLAIN));
        }
        hints.push((key.clone(), bold));
        hints.push((format!(" {what}"), Style::PLAIN.dim()));
        used += width;
    }
    lines.push(hints);
    lines.push(Vec::new());
    lines
}

/// The banner as the bytes to print after the screen is cleared: each line ends in CR LF, with the
/// styles reset at the end of each.
pub(super) fn bytes(cols: usize) -> Vec<u8> {
    let mut out = String::new();
    for line in lines(cols) {
        for (text, style) in line {
            if style != Style::PLAIN {
                out.push_str(&style.escape());
            }
            out.push_str(&text);
            if style != Style::PLAIN {
                out.push_str("\x1b[0m");
            }
        }
        out.push_str("\r\n");
    }
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(cols: usize) -> Vec<String> {
        lines(cols)
            .into_iter()
            .map(|line| line.into_iter().map(|(text, _)| text).collect::<String>().trim_end().to_string())
            .collect()
    }

    #[test]
    fn the_banner_is_a_mark_a_name_and_the_keymaps_keys() {
        let text = plain(100);
        assert_eq!(text.len(), 4, "{text:?}");
        assert!(text[0].is_empty() && text[3].is_empty());
        assert_eq!(text[1], "  >_ Wormhole", "the mark where the welcome had it");
        let t = keymap::glyphs(ActionId::NewTerminal);
        let p = keymap::glyphs(ActionId::CommandPalette);
        assert!(text[2].contains(&format!("{t} new terminal")), "{}", text[2]);
        assert!(text[2].contains(&format!("{p} command palette")), "{}", text[2]);
        // The keys are the table's: no typed-in glyphs to drift from it.
        assert!(!t.is_empty() && !p.is_empty());
    }

    #[test]
    fn the_mark_is_in_the_row_and_column_the_ship_docked_on() {
        let text = plain(100);
        let (row, col) = MARK_CELL;
        assert!(text[row].chars().skip(col).collect::<String>().starts_with(">_"));
    }

    #[test]
    fn hints_that_do_not_fit_are_left_out_not_cut() {
        let narrow = plain(40);
        assert!(narrow[2].contains("new terminal"));
        assert!(!narrow[2].contains("keyboard shortcuts"), "{}", narrow[2]);
        for line in &narrow {
            assert!(text_width(line) <= 40, "{line:?}");
        }
    }

    #[test]
    fn it_prints_with_resets_and_crlf() {
        let bytes = bytes(80);
        let text = String::from_utf8(bytes).expect("utf-8");
        assert_eq!(text.matches("\r\n").count(), 4);
        assert!(text.contains("\x1b[0;1m") && text.contains("\x1b[0m"));
        assert!(!text.contains("38;") && !text.contains("48;"), "palette colours only");
    }
}
