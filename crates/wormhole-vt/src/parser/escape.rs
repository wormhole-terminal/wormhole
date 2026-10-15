//! Escape sequences: the bytes after an ESC, up to the sequence they introduce.

use super::{Handler, Parser, State};

impl Parser {
    /// After an ESC (`Escape`), or after the ESC that ended a string (`EscapeSt`).
    #[inline]
    pub(super) fn escape<H: Handler>(&mut self, h: &mut H, bytes: &[u8], mut at: usize) -> usize {
        while at < bytes.len() {
            let byte = bytes[at];
            at += 1;
            match byte {
                b'[' => {
                    self.begin();
                    self.state = State::CsiParam;
                    return at;
                }
                b']' => {
                    self.state = State::Osc;
                    h.osc_start();
                    return at;
                }
                b'P' => {
                    self.begin();
                    self.state = State::DcsParam;
                    return at;
                }
                b'_' => {
                    self.state = State::Apc;
                    h.apc_start();
                    return at;
                }
                b'X' | b'^' => {
                    self.state = State::StringIgnored;
                    return at;
                }
                // ST (ECMA-48 8.3.143 is ESC 05/12) after a string: its end was reported already.
                b'\\' if self.state == State::EscapeSt => {
                    self.state = State::Ground;
                    return at;
                }
                // Williams' esc_dispatch range: every Fe, Fs and Fp byte but the introducers above.
                0x30..=0x7E => {
                    h.esc(&[], byte);
                    self.state = State::Ground;
                    return at;
                }
                0x20..=0x2F => {
                    self.begin();
                    self.csi.collect(byte);
                    self.state = State::EscapeIntermediate;
                    return at;
                }
                0x18 | 0x1A => {
                    h.execute(byte);
                    self.state = State::Ground;
                    return at;
                }
                // Another ESC starts over, and is no longer the end of a string.
                0x1B => self.state = State::Escape,
                0x00..=0x1F => h.execute(byte),
                0x80..=0x9F if h.eight_bit_controls() => {
                    self.c1(h, byte, false);
                    return at;
                }
                // DEL, and the bytes of UTF-8: not part of a sequence.
                _ => {}
            }
        }
        at
    }

    /// After `ESC` and one or more intermediate bytes: more intermediates, then the final byte.
    pub(super) fn escape_intermediate<H: Handler>(
        &mut self,
        h: &mut H,
        bytes: &[u8],
        mut at: usize,
    ) -> usize {
        while at < bytes.len() {
            let byte = bytes[at];
            at += 1;
            match byte {
                0x20..=0x2F => self.csi.collect(byte),
                0x30..=0x7E => {
                    // Williams' `collect` flags too many intermediates so that the dispatch does
                    // nothing: here that is the fifth.
                    if !self.csi.overflow() {
                        h.esc(self.csi.intermediates(), byte);
                    }
                    self.state = State::Ground;
                    return at;
                }
                0x18 | 0x1A => {
                    h.execute(byte);
                    self.state = State::Ground;
                    return at;
                }
                0x1B => {
                    self.state = State::Escape;
                    return at;
                }
                0x00..=0x1F => h.execute(byte),
                0x80..=0x9F if h.eight_bit_controls() => {
                    self.c1(h, byte, false);
                    return at;
                }
                _ => {}
            }
        }
        at
    }
}
