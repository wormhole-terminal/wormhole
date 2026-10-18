//! Control strings: OSC (ECMA-48 8.3.89), DCS data (8.3.27), APC (8.3.2), and the PM (8.3.94) and
//! SOS (8.3.128) strings that nobody wants. Each is closed by ST (ESC \), and each ends early at
//! CAN or SUB (8.3.6, 8.3.148, "the data preceding it is in error"). The payload goes to the
//! handler in slices of the input: found in bulk by `scan`, never copied.

use super::c1::first_c1_in_text;
use super::scan::{first_control, string_run};
use super::{Handler, Parser, State};

/// Which string `string_data` reads.
const DCS: u8 = 0;
const APC: u8 = 1;
const IGNORED: u8 = 2;

impl Parser {
    /// An OSC's text up to BEL, ST or a cancelling control.
    #[inline]
    pub(super) fn osc<H: Handler>(&mut self, h: &mut H, bytes: &[u8], mut at: usize) -> usize {
        loop {
            let mut text = first_control(&bytes[at..]);
            if h.eight_bit_controls() {
                text = first_c1_in_text(&bytes[at..at + text], &mut self.string_utf8);
            }
            if text > 0 {
                h.osc_put(&bytes[at..at + text]);
                at += text;
            }
            let Some(&control) = bytes.get(at) else {
                return at;
            };
            // A control is never inside a character: whatever comes next starts afresh.
            self.string_utf8 = 0;
            at += 1;
            match control {
                0x80..=0x9F => {
                    h.osc_end(false);
                    self.c1(h, control, true);
                    return at;
                }
                // xterm's ctlseqs: BEL ends an OSC as well as ST, for legacy programs.
                0x07 => {
                    h.osc_end(true);
                    self.state = State::Ground;
                    return at;
                }
                0x1B => {
                    h.osc_end(false);
                    return self.after_string_esc(bytes, at);
                }
                0x18 | 0x1A => {
                    h.osc_cancel();
                    h.execute(control);
                    self.state = State::Ground;
                    return at;
                }
                // Any other C0 control is no part of the text (Williams: ignored in OSC String).
                _ => {}
            }
        }
    }

    /// The string ended at an ESC (the byte before `at`). Nearly always the next byte is the `\`
    /// that makes it the ST: take both at once instead of going round the state machine.
    #[inline(always)]
    fn after_string_esc(&mut self, bytes: &[u8], at: usize) -> usize {
        if bytes.get(at) == Some(&b'\\') {
            self.state = State::Ground;
            at + 1
        } else {
            self.state = State::EscapeSt;
            at
        }
    }

    pub(super) fn dcs_data<H: Handler>(&mut self, h: &mut H, bytes: &[u8], at: usize) -> usize {
        self.string_data::<H, DCS>(h, bytes, at)
    }

    pub(super) fn apc<H: Handler>(&mut self, h: &mut H, bytes: &[u8], at: usize) -> usize {
        self.string_data::<H, APC>(h, bytes, at)
    }

    /// A PM or SOS string, or a DCS with a malformed header: skipped to the end.
    pub(super) fn string_ignored<H: Handler>(&mut self, h: &mut H, bytes: &[u8], at: usize) -> usize {
        self.string_data::<H, IGNORED>(h, bytes, at)
    }

    /// The data of a DCS or APC, or the bytes of a string that is skipped. Everything is data but
    /// ESC (the end), CAN and SUB (a cancel) and DEL (dropped, as Williams' `put` does not take it).
    /// BEL is data: only an OSC ends at BEL.
    #[inline]
    fn string_data<H: Handler, const KIND: u8>(&mut self, h: &mut H, bytes: &[u8], mut at: usize) -> usize {
        loop {
            let mut data = string_run(&bytes[at..]);
            if h.eight_bit_controls() {
                data = first_c1_in_text(&bytes[at..at + data], &mut self.string_utf8);
            }
            if data > 0 {
                match KIND {
                    DCS => h.dcs_put(&bytes[at..at + data]),
                    APC => h.apc_put(&bytes[at..at + data]),
                    _ => {}
                }
                at += data;
            }
            let Some(&stop) = bytes.get(at) else {
                return at;
            };
            self.string_utf8 = 0;
            at += 1;
            match stop {
                0x80..=0x9F => {
                    match KIND {
                        DCS => h.dcs_unhook(),
                        APC => h.apc_end(),
                        _ => {}
                    }
                    self.c1(h, stop, true);
                    return at;
                }
                0x1B => {
                    match KIND {
                        DCS => h.dcs_unhook(),
                        APC => h.apc_end(),
                        _ => {}
                    }
                    return self.after_string_esc(bytes, at);
                }
                0x18 | 0x1A => {
                    match KIND {
                        DCS => h.dcs_cancel(),
                        APC => h.apc_cancel(),
                        _ => {}
                    }
                    h.execute(stop);
                    self.state = State::Ground;
                    return at;
                }
                _ => {} // DEL
            }
        }
    }
}
