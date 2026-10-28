//! Where in a byte stream a terminal's parser is between sequences.
//!
//! A checkpoint (`ESC [ 5 n`) may only go where the terminal has nothing open: not after the `ESC` of a
//! sequence, not inside a CSI, OSC, DCS or APC string, not in the middle of a UTF-8 character. Inside
//! any of those the query's own bytes would become part of the payload's sequence and change what the
//! payload means.
//!
//! This is a scanner of its own and not `term::scan`, which reads what a terminal sends back. It follows
//! the state machine of xterm-style parsers: ESC, CSI, strings that end with ST (or BEL for an OSC), CAN
//! and SUB, UTF-8. Where such parsers are known to differ on a malformed sequence (a BEL in a DCS, a
//! control character inside a CSI) it stays "open". That is its judgement of the cases it knows, not a
//! promise about every parser. C1 controls (a raw 0x80-0x9f, or U+0080-U+009F as UTF-8) are not modelled;
//! no payload contains one (a test checks), and the plan only cuts between two ASCII bytes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ground,
    /// After `ESC`.
    Escape,
    /// `ESC` and then intermediate bytes (`ESC ( B`, `ESC # 8`): the final byte is missing.
    EscapeIntermediate,
    /// Inside `CSI`, before its final byte.
    Csi,
    /// Inside a string (`OSC`, `DCS`, `APC`, `PM`, `SOS`) that ends with ST, or with BEL for an OSC.
    Text {
        osc: bool,
    },
    /// Inside a string, right after an `ESC`: a `\` ends it.
    TextEscape {
        osc: bool,
    },
    /// Inside a UTF-8 character: this many continuation bytes are missing.
    Utf8(u8),
}

#[derive(Clone, Copy, Debug)]
pub struct Scanner {
    state: State,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self { state: State::Ground }
    }

    /// True when the bytes fed so far end between sequences and between characters.
    pub fn is_ground(&self) -> bool {
        self.state == State::Ground
    }

    pub fn feed(&mut self, b: u8) {
        use State::*;
        // CAN and SUB cancel whatever is open in the parsers this follows (xterm's, VT500's).
        if matches!(b, 0x18 | 0x1a) {
            self.state = Ground;
            return;
        }
        self.state = match (self.state, b) {
            // An ESC ends a string only with the `\` after it; anywhere else it starts over.
            (Text { osc }, 0x1b) => TextEscape { osc },
            (TextEscape { .. }, b'\\') => Ground,
            (TextEscape { .. }, _) => {
                self.state = Escape;
                return self.feed(b);
            }
            (_, 0x1b) => Escape,
            (Text { osc: true }, 0x07) => Ground,
            (Text { osc }, _) => Text { osc },

            (Escape, b'[') => Csi,
            (Escape, b']') => Text { osc: true },
            (Escape, b'P' | b'_' | b'^' | b'X') => Text { osc: false },
            (Escape | EscapeIntermediate, 0x20..=0x2f) => EscapeIntermediate,
            (Escape | EscapeIntermediate, 0x30..=0x7e) => Ground,
            // A control character inside an escape sequence is acted on and the sequence goes on; a
            // byte that can't belong ends it in some parsers and not in others, so it stays open.
            (Escape, _) => Escape,
            (EscapeIntermediate, _) => EscapeIntermediate,

            (Csi, 0x40..=0x7e) => Ground,
            (Csi, _) => Csi,

            (Utf8(n), 0x80..=0xbf) => {
                if n == 1 {
                    Ground
                } else {
                    Utf8(n - 1)
                }
            }
            // Not a continuation: the character was cut short, and this byte starts afresh.
            (Utf8(_), _) => {
                self.state = Ground;
                return self.feed(b);
            }

            (Ground, 0xc2..=0xdf) => Utf8(1),
            (Ground, 0xe0..=0xef) => Utf8(2),
            (Ground, 0xf0..=0xf4) => Utf8(3),
            (Ground, _) => Ground,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the scanner is at ground after each byte of `bytes` (index i: after i + 1 bytes).
    fn grounds(bytes: &[u8]) -> Vec<bool> {
        let mut s = Scanner::new();
        bytes
            .iter()
            .map(|&b| {
                s.feed(b);
                s.is_ground()
            })
            .collect()
    }

    #[test]
    fn plain_text_is_closed_everywhere() {
        assert!(grounds(b"hello\r\nworld\x07\t").into_iter().all(|g| g));
    }

    #[test]
    fn a_csi_is_open_until_its_final_byte() {
        // ESC [ 3 8 ; 2 ; 1 m x
        let g = grounds(b"\x1b[38;2;1mx");
        assert_eq!(g, [false, false, false, false, false, false, false, false, true, true]);
        // Intermediates and private markers are part of it too.
        assert_eq!(grounds(b"\x1b[?25h"), [false, false, false, false, false, true]);
        assert_eq!(grounds(b"\x1b[ q"), [false, false, false, true]);
    }

    #[test]
    fn short_escapes_close_on_their_final_byte() {
        assert_eq!(grounds(b"\x1b7"), [false, true]);
        assert_eq!(grounds(b"\x1b(B"), [false, false, true]);
        assert_eq!(grounds(b"\x1b#8"), [false, false, true]);
        assert_eq!(grounds(b"\x1bM"), [false, true]);
    }

    #[test]
    fn strings_are_open_until_st_and_an_osc_also_until_bel() {
        let osc = b"\x1b]8;;http://x\x07a";
        let g = grounds(osc);
        assert!(g[..g.len() - 2].iter().all(|g| !g) && g[g.len() - 2] && g[g.len() - 1]);
        let st = b"\x1b]8;;\x1b\\a";
        assert_eq!(grounds(st), [false, false, false, false, false, false, true, true]);
        // BEL does not end a DCS or an APC: parsers differ, so it stays open.
        let dcs = b"\x1bP1$r\x07x\x1b\\";
        let g = grounds(dcs);
        assert!(g[..g.len() - 1].iter().all(|g| !g) && g[g.len() - 1]);
        let apc = b"\x1b_Ga=q;AAAA\x1b\\";
        let g = grounds(apc);
        assert!(g[..g.len() - 1].iter().all(|g| !g) && g[g.len() - 1]);
    }

    #[test]
    fn an_escape_inside_a_string_that_is_not_st_starts_a_new_sequence() {
        // The OSC is cut off by ESC [ ..., which is a CSI, open until its final byte.
        assert_eq!(
            grounds(b"\x1b]0;t\x1b[1m"),
            [false, false, false, false, false, false, false, false, true]
        );
    }

    #[test]
    fn utf8_characters_are_open_until_their_last_byte() {
        // é (2 bytes), € (3), 😀 (4)
        assert_eq!(grounds("é".as_bytes()), [false, true]);
        assert_eq!(grounds("€".as_bytes()), [false, false, true]);
        assert_eq!(grounds("😀".as_bytes()), [false, false, false, true]);
        // A character cut short by ASCII is over; the ASCII is then plain.
        assert_eq!(grounds(b"\xe2\x82a"), [false, false, true]);
    }

    #[test]
    fn can_and_sub_cancel() {
        assert_eq!(grounds(b"\x1b[3\x18a"), [false, false, false, true, true]);
        assert_eq!(grounds(b"\x1b]0;t\x1ax"), [false, false, false, false, false, true, true]);
    }

    #[test]
    fn a_control_character_inside_a_csi_does_not_end_it() {
        assert_eq!(grounds(b"\x1b[1\n2m"), [false, false, false, false, false, true]);
    }
}
