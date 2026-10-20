//! Pasting text and reporting focus.

use super::modes::InputModes;
use super::{ESC, push_char};

/// What starts and ends a bracketed paste (mode 2004): `CSI 200 ~` and `CSI 201 ~`.
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

/// Appends `text` as a terminal sends a paste.
///
/// With bracketed paste on (mode 2004) the text goes between `CSI 200 ~` and `CSI 201 ~`, so a
/// program can tell typing from pasting and take the text literally. **Everything that could end
/// the bracket early is removed**: ESC (and so the end marker `ESC [ 201 ~` itself) and the C1
/// controls U+0080 to U+009F (U+009B is CSI to a terminal in 8-bit mode). Without that, a web page
/// or a document could put `ESC [ 201 ~` in what a person copies, close the bracket and have
/// everything after it typed as commands: the "paste injection" attack bracketed paste exists to
/// stop. Other characters pass unchanged, tabs and line breaks included.
///
/// Line breaks (CRLF, LF) are sent as CR, the key a program sees for Return, in both modes. With
/// bracketed paste off a program reads each line as soon as the CR arrives, as xterm does for its
/// own selection paste.
pub fn paste(text: &str, modes: &InputModes, out: &mut Vec<u8>) {
    let bracketed = modes.bracketed_paste;
    out.reserve(text.len() + PASTE_START.len() + PASTE_END.len());
    if bracketed {
        out.extend_from_slice(PASTE_START);
    }
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                // CRLF is one line break.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push(b'\r');
            }
            '\n' => out.push(b'\r'),
            c if bracketed && (c == char::from(ESC) || ('\u{80}'..='\u{9f}').contains(&c)) => {}
            c => push_char(out, c),
        }
    }
    if bracketed {
        out.extend_from_slice(PASTE_END);
    }
}

/// Appends the report for the terminal gaining or losing focus: `CSI I` or `CSI O`, once the
/// program asked for them (mode 1004). Returns whether there was one.
pub fn focus(focused: bool, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    if !modes.focus_events {
        return false;
    }
    out.extend_from_slice(if focused { b"\x1b[I" } else { b"\x1b[O" });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasted(text: &str, bracketed: bool) -> Vec<u8> {
        let modes = InputModes { bracketed_paste: bracketed, ..InputModes::default() };
        let mut out = Vec::new();
        paste(text, &modes, &mut out);
        out
    }

    #[test]
    fn a_bracketed_paste_is_wrapped() {
        assert_eq!(pasted("ls -l", true), b"\x1b[200~ls -l\x1b[201~");
        assert_eq!(pasted("", true), b"\x1b[200~\x1b[201~");
    }

    #[test]
    fn an_unbracketed_paste_is_the_text() {
        assert_eq!(pasted("ls -l", false), b"ls -l");
        assert_eq!(pasted("Grüße 🌍", false), "Grüße 🌍".as_bytes());
    }

    #[test]
    fn line_breaks_become_returns() {
        assert_eq!(pasted("a\nb\r\nc\rd\n\n", false), b"a\rb\rc\rd\r\r");
        assert_eq!(pasted("a\nb", true), b"\x1b[200~a\rb\x1b[201~");
    }

    #[test]
    fn the_end_marker_cannot_be_smuggled_in() {
        let attack = "safe\x1b[201~rm -rf ~\n";
        let out = pasted(attack, true);
        assert_eq!(out, b"\x1b[200~safe[201~rm -rf ~\r\x1b[201~");
        // The C1 form of the same marker.
        assert_eq!(pasted("a\u{9b}201~b", true), b"\x1b[200~a201~b\x1b[201~");
    }

    #[test]
    fn c1_controls_and_escape_are_stripped_only_when_bracketed() {
        assert_eq!(pasted("a\u{85}b\u{9f}c\u{80}d", true), b"\x1b[200~abcd\x1b[201~");
        // Without the bracket there is nothing to escape from: the text is the text.
        assert_eq!(pasted("a\x1bb", false), b"a\x1bb");
        // U+00A0 and U+00FF are not controls.
        assert_eq!(pasted("\u{a0}\u{ff}", true), "\x1b[200~\u{a0}\u{ff}\x1b[201~".as_bytes());
    }

    /// Whatever is pasted, the bracket ends exactly once, at the very end, and no byte inside
    /// it could be read as the start of a sequence by a terminal in 7-bit or 8-bit mode.
    #[test]
    fn the_end_marker_never_appears_inside_the_bracket() {
        // A small deterministic generator, so a failure is reproducible.
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let pieces = [
            "\x1b",
            "[",
            "2",
            "0",
            "1",
            "~",
            "\x1b[201~",
            "\x1b[200~",
            "\u{9b}",
            "\u{9b}201~",
            "\u{85}",
            "\n",
            "\r",
            "\r\n",
            "a",
            " ",
            "é",
            "🌍",
            "\u{9f}",
            "\u{a0}",
            "\0",
            "\t",
        ];
        for _ in 0..2000 {
            let text: String =
                (0..(next() % 40)).map(|_| pieces[(next() % pieces.len() as u64) as usize]).collect();
            let out = pasted(&text, true);
            let inside = &out[PASTE_START.len()..out.len() - PASTE_END.len()];
            assert!(out.starts_with(PASTE_START) && out.ends_with(PASTE_END), "{text:?}");
            assert!(!inside.contains(&ESC), "ESC inside for {text:?}");
            let inside = String::from_utf8_lossy(inside);
            assert!(!inside.chars().any(|c| ('\u{80}'..='\u{9f}').contains(&c)), "C1 inside for {text:?}");
        }
    }

    #[test]
    fn focus_is_reported_only_when_asked_for() {
        let mut out = Vec::new();
        assert!(!focus(true, &InputModes::default(), &mut out));
        assert!(out.is_empty());
        let asked = InputModes { focus_events: true, ..InputModes::default() };
        assert!(focus(true, &asked, &mut out));
        assert!(focus(false, &asked, &mut out));
        assert_eq!(out, b"\x1b[I\x1b[O");
    }
}
