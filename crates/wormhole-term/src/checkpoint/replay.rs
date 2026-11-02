//! Drawing a seed before a shell starts: what a restored terminal shows above its prompt. A seed is VT with colors
//! and attributes that ends with its own divider, made by the app: a checkpoint's screen (`vt`) or the saved text
//! ([`text_seed`]). Only text, line ends and SGR get in ([`safe_replay`](super::vt::safe_replay)), and what the
//! engine queued is dropped after, so a seed obeys nothing.
//!
//! **The divider** is the app's words, in the person's language and date format, so nothing here may depend on
//! them: it is recognised by its look ([`is_divider_line`]): a line of dim italic text between two runs of four or
//! more `─`, which [`divider`] makes. The next checkpoint and the saved text leave such lines out, so they do not
//! pile up across restarts.

use crate::screen::Screen;

/// The line a seed ends with, saying `words` ("restored 2026-11-02 14:03", in the person's language): dim and
/// italic between rules, then a line end. `words` is cut to one short line.
pub fn divider(words: &str) -> String {
    let words: String = words.chars().filter(|c| !c.is_control()).take(80).collect();
    format!("\x1b[2;3m──── {words} ────\x1b[0m\r\n")
}

/// Whether a logical line is a divider: `plain` (its characters) is `────` or more, words, `────` or more, and
/// every character of it was drawn dim and italic (`dim_italic`).
pub(crate) fn is_divider_line(plain: &str, dim_italic: bool) -> bool {
    if !dim_italic {
        return false;
    }
    let line = plain.trim();
    let left = line.chars().take_while(|c| *c == '─').count();
    let right = line.chars().rev().take_while(|c| *c == '─').count();
    left >= 4 && right >= 4 && line.chars().count() > left + right
}

/// The saved text of a terminal as a seed: each line dim, then the app's divider line (`divider`, made with
/// [`divider`]). Control characters in the text are dropped: saved text came from a screen, and a file the person
/// can edit may hold anything.
pub fn text_seed(text: &str, divider: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let mut seed = String::with_capacity(text.len() + divider.len() + 64);
    for line in text.lines() {
        seed.push_str("\x1b[2m");
        seed.extend(line.chars().filter(|c| *c == '\t' || !c.is_control()));
        seed.push_str("\x1b[0m\r\n");
    }
    seed.push_str(divider);
    seed
}

/// Draw `seed` on a screen that has no program yet.
pub(crate) fn replay_seed(screen: &mut Screen, seed: &str) {
    if seed.trim().is_empty() {
        return;
    }
    screen.feed(&super::vt::safe_replay(seed.as_bytes()));
    screen.discard_output();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GridSize;

    fn screen() -> Screen {
        Screen::new(GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 }, 100)
    }

    #[test]
    fn a_seed_keeps_its_colors_and_acts_on_nothing() {
        let mut screen = screen();
        replay_seed(
            &mut screen,
            "\x1b[1;31mred\x1b[0m\r\n\x1b[6n\x1b]52;c;?\x07\x1b]0;t\x07\x1b[?1000hend\r\n",
        );
        assert!(screen.queued_replies().is_empty() && screen.drain_events().is_empty());
        let drawn = crate::contents::screen_text(&screen);
        assert!(drawn.contains("red") && drawn.contains("end"), "{drawn}");
        let mut cells = Vec::new();
        screen.copy_row(wormhole_vt::Line(0), &mut cells, None);
        assert!(cells[0].flags.contains(wormhole_vt::Flags::BOLD), "the colors and attributes are drawn");
        assert!(!screen.mode().contains(wormhole_vt::Modes::MOUSE_CLICK), "no mode was set");
    }

    #[test]
    fn a_seed_with_queries_and_escapes_in_saved_text_acts_on_nothing() {
        let mut screen = screen();
        let seed = text_seed(
            "before\n\x1b[6n\x1b]52;c;?\x07\x1b]0;hijacked\x07\x1b[5nafter\x07\n",
            &divider("restored"),
        );
        replay_seed(&mut screen, &seed);
        assert!(screen.queued_replies().is_empty(), "no reply for the program");
        assert!(screen.drain_events().is_empty(), "no event for the app");
        let drawn = crate::contents::screen_text(&screen);
        assert!(drawn.contains("before") && drawn.contains("after") && drawn.contains("restored"), "{drawn}");
        assert!(!drawn.contains('\u{1b}') && !drawn.contains("hijacked\u{7}"), "{drawn}");
    }

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
        assert!(screen.queued_replies().is_empty() && screen.drain_events().is_empty());
    }

    #[test]
    fn the_divider_is_a_look_not_a_word() {
        // Any language, any date format.
        for words in
            ["restored 2026-11-02 14:03", "wiederhergestellt 02.11.2026 14:03", "復元 2026/11/02 14:03"]
        {
            assert!(is_divider_line(&format!("──── {words} ────"), true), "{words}");
        }
        assert!(is_divider_line("──── anything at all ────", true));
        assert!(
            !is_divider_line("──── anything at all ────", false),
            "not dim and italic: someone's own rule"
        );
        assert!(!is_divider_line("────────────", true), "a rule with no words");
        assert!(!is_divider_line("── short ──", true));
        assert_eq!(divider("a\nb"), "\x1b[2;3m──── ab ────\x1b[0m\r\n");
    }

    #[test]
    fn saved_text_becomes_dim_lines_and_the_divider() {
        let seed = text_seed("one\ntwo\x1b[31m\n", &divider("restored"));
        assert_eq!(
            seed,
            "\x1b[2mone\x1b[0m\r\n\x1b[2mtwo[31m\x1b[0m\r\n\x1b[2;3m──── restored ────\x1b[0m\r\n"
        );
        assert_eq!(text_seed("  \n", &divider("restored")), "", "nothing to restore, no divider");
    }
}
