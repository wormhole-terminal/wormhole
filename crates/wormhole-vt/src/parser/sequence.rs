//! Control sequences (`ESC [`, ECMA-48 8.3.16) and the header of a device control string
//! (`ESC P`, 8.3.27), which is written the same way: private marker, parameters, intermediates,
//! final byte. `DCS` says which of the two is being read; the two differ in what a C0 control
//! does (run in a CSI, ignored in a DCS header), in what the final byte does (dispatch, or hook the
//! data that follows) and in what ESC ends (a sequence, or a string).

use super::{Handler, Parser, State};

impl Parser {
    /// Parameter bytes (0x30..=0x3F) and what follows them. Williams' CSI Entry and CSI Param.
    #[inline]
    pub(super) fn params<H: Handler, const DCS: bool>(
        &mut self,
        h: &mut H,
        bytes: &[u8],
        mut at: usize,
    ) -> usize {
        // The parameter being read lives in registers for the length of the call.
        let (mut cur, mut digits) = (self.cur, self.digits);
        while at < bytes.len() {
            let byte = bytes[at];
            at += 1;
            let digit = u32::from(byte).wrapping_sub(u32::from(b'0'));
            if digit < 10 {
                // xterm clamps a value rather than wrapping it; 65535 is as far as a u16 goes.
                cur = (cur * 10 + digit).min(0xFFFF);
                digits = true;
                continue;
            }
            match byte {
                b';' => {
                    self.csi.push(cur, digits);
                    cur = 0;
                    digits = false;
                }
                // ECMA-48 5.4.2 b: a colon joins sub-parameters into one parameter.
                b':' => {
                    self.csi.push(cur, digits);
                    self.csi.start_sub();
                    cur = 0;
                    digits = false;
                }
                0x40..=0x7E => {
                    self.csi.push(cur, digits);
                    self.csi.set_final(byte);
                    if DCS {
                        h.dcs_hook(&self.csi);
                        self.state = State::DcsData;
                    } else {
                        h.csi(&self.csi);
                        self.state = State::Ground;
                    }
                    return at;
                }
                // A private marker (ECMA-48 5.4.1 b) is valid as the first byte only.
                0x3C..=0x3F => {
                    if self.csi.at_start(digits) {
                        self.csi.set_marker(byte);
                    } else {
                        self.state = if DCS { State::StringIgnored } else { State::CsiIgnore };
                        return at;
                    }
                }
                0x20..=0x2F => {
                    self.csi.push(cur, digits);
                    self.csi.collect(byte);
                    self.state = if DCS { State::DcsIntermediate } else { State::CsiIntermediate };
                    return at;
                }
                0x18 | 0x1A => {
                    h.execute(byte);
                    self.state = State::Ground;
                    return at;
                }
                0x1B => {
                    self.state = if DCS { State::EscapeSt } else { State::Escape };
                    return at;
                }
                // A control inside a sequence runs and the sequence goes on (Williams: `execute` in
                // every CSI state); in a DCS header it is ignored.
                0x00..=0x1F if !DCS => h.execute(byte),
                0x80..=0x9F if h.eight_bit_controls() => {
                    self.c1(h, byte, DCS);
                    return at;
                }
                // DEL, and the bytes of UTF-8: not part of a sequence.
                _ => {}
            }
        }
        self.cur = cur;
        self.digits = digits;
        at
    }

    /// Intermediate bytes (0x20..=0x2F) after the parameters, up to the final byte.
    pub(super) fn intermediates<H: Handler, const DCS: bool>(
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
                0x40..=0x7E => {
                    self.csi.set_final(byte);
                    if DCS {
                        h.dcs_hook(&self.csi);
                        self.state = State::DcsData;
                    } else {
                        h.csi(&self.csi);
                        self.state = State::Ground;
                    }
                    return at;
                }
                // A parameter byte after an intermediate is malformed (Williams: CSI Intermediate).
                0x30..=0x3F => {
                    self.state = if DCS { State::StringIgnored } else { State::CsiIgnore };
                    return at;
                }
                0x18 | 0x1A => {
                    h.execute(byte);
                    self.state = State::Ground;
                    return at;
                }
                0x1B => {
                    self.state = if DCS { State::EscapeSt } else { State::Escape };
                    return at;
                }
                0x00..=0x1F if !DCS => h.execute(byte),
                0x80..=0x9F if h.eight_bit_controls() => {
                    self.c1(h, byte, DCS);
                    return at;
                }
                _ => {}
            }
        }
        at
    }

    /// A malformed CSI: skipped to its final byte. Controls still run.
    pub(super) fn csi_ignore<H: Handler>(&mut self, h: &mut H, bytes: &[u8], mut at: usize) -> usize {
        while at < bytes.len() {
            let byte = bytes[at];
            at += 1;
            match byte {
                0x40..=0x7E => {
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
