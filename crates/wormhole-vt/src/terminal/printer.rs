//! Media copy (MC, `CSI Ps i`) and the printer controller mode.
//!
//! Spec: ECMA-48 (MC), the VT510 manual ("MC: Media Copy") and xterm's ctlseqs ("CSI Ps i",
//! "CSI ? Ps i"). `CSI 5 i` turns the printer controller mode on: from then on the terminal sends
//! everything it receives to the printer, not the screen, until it reads `CSI 4 i`. The terminal
//! has no printer, so the data is discarded as it is read: nothing is kept, nothing reaches the
//! screen, nothing is replied. The end sequence is searched byte by byte over what the printer
//! would have got, with its progress kept between [`feed`](Terminal::feed) calls, so it is found
//! however the input is cut. With 8-bit controls on, the C1 form of CSI (0x9B) ends it as well.
//!
//! The other media copy functions (`CSI 0 i` print screen, `CSI ? 1 i` print line, `CSI ? 4 i` and
//! `CSI ? 5 i` auto print off and on, `CSI ? 10 i`, `CSI ? 11 i`, xterm's `CSI 10 i` and `11 i`
//! screen dumps) have no printer to go to and no result to report (DSR 15 says there is none):
//! they are ignored.
//!
//! A program that turns the mode on and never off leaves the terminal ignoring its output until
//! the host resets it ([`Terminal::reset`]); like a keyboard lock, it is the program's own output
//! that it silences, and it cannot grow memory.

use crate::terminal::Terminal;

/// Whether the printer controller mode is on, and how much of its end sequence the data just read
/// ended with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct Printer {
    on: bool,
    /// How far the last bytes got into `ESC [ 4 i`: 0 nothing, 1 the ESC, 2 the `[` (or C1 CSI),
    /// 3 the `4`.
    seen: u8,
}

impl Printer {
    /// Whether the mode is on: the bytes read are the printer's.
    pub(in crate::terminal) fn on(&self) -> bool {
        self.on
    }

    /// Whether the end sequence has begun, so that a screen rebuilt now would lose its start.
    pub(in crate::terminal) fn mid_sequence(&self) -> bool {
        self.seen != 0
    }
}

impl Terminal {
    /// `CSI Ps i`: 5 starts the printer controller mode, 4 ends it (outside it, a no-op); the other
    /// functions print something and there is no printer.
    pub(in crate::terminal) fn media_copy(&mut self, mode: u16) {
        match mode {
            5 => self.printer = Printer { on: true, seen: 0 },
            4 => self.printer = Printer::default(),
            _ => {}
        }
    }

    /// Whether the bytes are going to the printer instead of being read.
    #[inline]
    pub(in crate::terminal) fn printing(&self) -> bool {
        self.printer.on
    }

    /// Discard what the printer would get from the start of `bytes`: everything up to and including
    /// the end sequence, or all of them when it is not there yet. Returns how many were used up; the
    /// rest is for the parser.
    pub(in crate::terminal) fn discard_printer_data(&mut self, bytes: &[u8]) -> usize {
        let c1 = self.eight_bit && !self.modes.contains(crate::terminal::modes::Modes::VT52);
        let mut seen = self.printer.seen;
        for (at, &byte) in bytes.iter().enumerate() {
            seen = match (seen, byte) {
                (3, b'i') => {
                    self.printer = Printer::default();
                    return at + 1;
                }
                (2, b'4') => 3,
                (1, b'[') => 2,
                (_, 0x1B) => 1,
                (_, 0x9B) if c1 => 2,
                _ => 0,
            };
        }
        self.printer.seen = seen;
        bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::terminal::Terminal;

    fn screen(term: &Terminal) -> String {
        term.row_text(crate::types::Line(0), true)
    }

    #[test]
    fn data_between_the_two_sequences_is_not_drawn() {
        let mut term = Terminal::new(20, 2);
        term.feed(b"a\x1b[5ihidden\x1b[1mtoo\x1b[4ib");
        assert_eq!(screen(&term), "ab");
        assert!(!term.printer.on());
    }

    #[test]
    fn the_end_sequence_is_found_however_the_input_is_cut() {
        let input = b"a\x1b[5i\x1b[5i junk \x1b\x1b[[4\x1b[4i\x1b[4ib";
        let mut whole = Terminal::new(20, 2);
        whole.feed(input);
        assert_eq!(screen(&whole), "ab");
        for cut in 1..input.len() {
            let mut term = Terminal::new(20, 2);
            term.feed(&input[..cut]);
            term.feed(&input[cut..]);
            assert_eq!(screen(&term), "ab", "cut at {cut}");
        }
        // One byte at a time.
        let mut term = Terminal::new(20, 2);
        for byte in input {
            term.feed(&[*byte]);
        }
        assert_eq!(screen(&term), "ab");
    }

    #[test]
    fn near_misses_do_not_end_it() {
        let mut term = Terminal::new(20, 2);
        term.feed(b"\x1b[5i\x1b[4;1i\x1b[?4i\x1b[04i\x1b[ 4i\x1b4ix");
        assert!(term.printer.on());
        assert_eq!(screen(&term), "");
    }

    #[test]
    fn the_c1_form_ends_it_only_with_8_bit_controls() {
        let mut term = Terminal::new(20, 2);
        term.feed(b"\x1b[5i\xc2\x9b4ix");
        assert!(term.printer.on(), "UTF-8 text is not a control");
        let mut term = Terminal::new(20, 2);
        term.feed(b"\x1b G\x1b[5i\x9b4ix");
        assert_eq!(screen(&term), "x");
    }

    #[test]
    fn nothing_is_kept_for_endless_data() {
        let mut term = Terminal::new(20, 2);
        term.feed(b"\x1b[5i");
        let junk = vec![b'x'; 1 << 20];
        for _ in 0..16 {
            term.feed(&junk);
        }
        assert!(term.printer.on());
        assert_eq!(screen(&term), "");
        assert!(term.replies().is_empty());
    }

    #[test]
    fn a_reset_ends_it() {
        let mut term = Terminal::new(20, 2);
        term.feed(b"\x1b[5i");
        term.reset();
        term.feed(b"a");
        assert_eq!(screen(&term), "a");
    }

    #[test]
    fn the_functions_that_print_something_do_nothing() {
        let mut term = Terminal::new(40, 2);
        term.feed(b"\x1b[ia\x1b[0ib\x1b[4ic\x1b[?1id\x1b[?4ie\x1b[?5if\x1b[?10ig\x1b[?11ih\x1b[10ii");
        assert_eq!(screen(&term), "abcdefghi");
        assert!(!term.printer.on());
    }
}
