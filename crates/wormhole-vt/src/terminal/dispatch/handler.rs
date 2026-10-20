//! The parser's callbacks: [`Sink`] is the [`Handler`] of the terminal's own byte stream.
//!
//! The terminal does not implement `Handler` itself: that would make every callback (`csi`,
//! `osc_put`, …) a public method on `Terminal`, callable by anyone with half a sequence. The
//! adapter is private, so the only way in is [`Terminal::feed`].
//!
//! Every sequence that is not text ends the grapheme cluster being built (mode 2027): the cursor
//! may have moved, and the next character starts a cluster of its own. It also ends the character
//! REP repeats: ECMA-48's REP repeats "the preceding character" if that is a graphic one, and a
//! control function is not (xterm forgets it the same way, a control sequence or a C0 control
//! returns its parser to the ground state with no character in hand).

use crate::parser::{Csi, Handler};

use crate::terminal::modes::Modes;
use crate::terminal::screen::status_line;
use crate::terminal::{Terminal, Terminator};

/// A terminal as the parser's handler, for the length of one `feed`.
pub(in crate::terminal) struct Sink<'a>(pub(in crate::terminal) &'a mut Terminal);

impl Handler for Sink<'_> {
    #[inline]
    fn eight_bit_controls(&self) -> bool {
        // The VT52 has no C1 controls.
        self.0.eight_bit && !self.0.modes.contains(Modes::VT52)
    }

    #[inline]
    fn vt52_mode(&self) -> bool {
        self.0.modes.contains(Modes::VT52)
    }

    #[inline]
    fn wants_raw(&self) -> bool {
        self.0.printing()
    }

    fn raw(&mut self, bytes: &[u8]) -> usize {
        self.0.discard_printer_data(bytes)
    }

    #[inline]
    fn print_ascii(&mut self, run: &[u8]) {
        if self.0.on_status_line() {
            return;
        }
        // The two characters after `ESC Y` are coordinates, not text.
        if self.0.vt52.addressing() {
            let rest = self.0.vt52_take_address(run);
            if !rest.is_empty() {
                self.0.print_ascii_run(rest);
            }
            return;
        }
        self.0.print_ascii_run(run);
    }

    #[inline]
    fn print(&mut self, c: char) {
        if self.0.on_status_line() {
            return;
        }
        if self.0.vt52.addressing() {
            self.0.vt52_take_address_char();
            return;
        }
        self.0.print_char(c);
    }

    fn execute(&mut self, byte: u8) {
        self.0.end_text();
        self.0.history_just_cleared = false;
        if self.0.on_status_line() && status_line::control_acts_on_display(byte) {
            return;
        }
        self.0.control(byte);
    }

    fn csi(&mut self, csi: &Csi) {
        // `dispatch_csi` forgets the character REP repeats, after REP has had it.
        self.0.cluster.open = false;
        self.0.dispatch_csi(csi);
    }

    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.0.end_text();
        self.0.history_just_cleared = false;
        self.0.dispatch_esc(intermediates, byte);
    }

    fn osc_start(&mut self) {
        self.0.osc.start();
    }

    fn osc_put(&mut self, data: &[u8]) {
        self.0.osc.put(data);
    }

    fn osc_end(&mut self, bel_terminated: bool) {
        self.0.end_text();
        self.0.finish_osc(if bel_terminated { Terminator::Bel } else { Terminator::St });
    }

    fn osc_cancel(&mut self) {
        self.0.osc.cancel();
    }

    fn dcs_hook(&mut self, csi: &Csi) {
        self.0.end_text();
        self.0.hook_dcs(csi);
    }

    fn dcs_put(&mut self, data: &[u8]) {
        self.0.dcs.put(data);
    }

    fn dcs_unhook(&mut self) {
        self.0.unhook_dcs();
    }

    fn dcs_cancel(&mut self) {
        self.0.dcs.cancel();
    }

    fn apc_start(&mut self) {
        self.0.dcs.start_apc();
    }

    fn apc_put(&mut self, data: &[u8]) {
        self.0.put_apc(data);
    }

    fn apc_end(&mut self) {
        self.0.end_text();
        self.0.finish_apc();
    }

    fn apc_cancel(&mut self) {
        self.0.cancel_apc();
    }
}
