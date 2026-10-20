//! The ground state: text, C0 controls and the start of everything else.

use super::scan::printable_run;
use super::utf8::{Decoded, REPLACEMENT, decode};
use super::{Handler, Parser, State};

impl Parser {
    /// Read from `at` until the input ends or a sequence begins; returns the index reached.
    #[inline]
    pub(super) fn ground<H: Handler>(&mut self, h: &mut H, bytes: &[u8], mut at: usize) -> usize {
        while at < bytes.len() {
            let byte = bytes[at];
            // Printable ASCII goes to the handler as one slice, however long.
            if (0x20..0x7F).contains(&byte) {
                let run = printable_run(&bytes[at..]);
                h.print_ascii(&bytes[at..at + run]);
                at += run;
                continue;
            }
            // Text that is not ASCII is next: a run of characters one after the other, where the
            // tests for the controls below would only be in the way.
            if byte >= 0x80 {
                // A byte of 0x80..=0x9F cannot begin a character: in 8-bit mode it is a control.
                if byte < 0xA0 && h.eight_bit_controls() {
                    at += 1;
                    self.c1(h, byte, false);
                    if self.state != State::Ground {
                        return at;
                    }
                    continue;
                }
                match decode(&bytes[at..]) {
                    Decoded::Char(c, len) => {
                        h.print(c);
                        at += len;
                    }
                    Decoded::Invalid(len) => {
                        h.print(REPLACEMENT);
                        at += len;
                    }
                    Decoded::Incomplete => {
                        // The input ends inside a character: keep what there is for the next call.
                        let rest = &bytes[at..];
                        self.pending[..rest.len()].copy_from_slice(rest);
                        self.pending_len = rest.len() as u8;
                        self.state = State::Utf8;
                        return bytes.len();
                    }
                }
                continue;
            }
            match byte {
                0x1B => {
                    // `ESC [` and `ESC ]` are the commonest starts, and what follows the ESC is
                    // nearly always in the input too: read them without a trip round the state
                    // machine, and carry on here if the sequence ends within the input.
                    match bytes.get(at + 1) {
                        Some(b'[') if !h.vt52_mode() => {
                            self.begin();
                            self.state = State::CsiParam;
                            at = self.params::<H, false>(h, bytes, at + 2);
                        }
                        Some(b']') if !h.vt52_mode() => {
                            self.state = State::Osc;
                            h.osc_start();
                            at = self.osc(h, bytes, at + 2);
                        }
                        _ => {
                            self.state = State::Escape;
                            return at + 1;
                        }
                    }
                    if self.state != State::Ground {
                        return at;
                    }
                }
                // ECMA-35 6.2.1: DEL carries no information, unless the set in GL has a character
                // there (a 96-character set): then it is that one.
                0x7F => {
                    if h.del_is_graphic() {
                        h.print_ascii(&bytes[at..=at]);
                    }
                    at += 1;
                }
                // What is left is C0.
                _ => {
                    h.execute(byte);
                    at += 1;
                }
            }
        }
        at
    }

    /// Carry on a character that the previous input ended in the middle of.
    #[cold]
    pub(super) fn resume_utf8<H: Handler>(&mut self, h: &mut H, bytes: &[u8], at: usize) -> usize {
        let kept = self.pending_len as usize;
        // A character is at most four bytes, so what is kept plus a few new bytes settles it.
        let take = (4 - kept).min(bytes.len() - at);
        let mut buf = [0u8; 4];
        buf[..kept].copy_from_slice(&self.pending[..kept]);
        buf[kept..kept + take].copy_from_slice(&bytes[at..at + take]);
        let have = kept + take;
        match decode(&buf[..have]) {
            Decoded::Char(c, len) => {
                self.state = State::Ground;
                h.print(c);
                at + len.saturating_sub(kept)
            }
            Decoded::Invalid(len) => {
                // The kept bytes were a valid beginning, so the break is in the new ones: the
                // replacement covers them and the bytes after it are read afresh.
                self.state = State::Ground;
                h.print(REPLACEMENT);
                at + len.saturating_sub(kept)
            }
            Decoded::Incomplete => {
                self.pending[..have].copy_from_slice(&buf[..have]);
                self.pending_len = have as u8;
                at + take
            }
        }
    }
}
