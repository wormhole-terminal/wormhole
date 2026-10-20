//! The parser's callbacks: the terminal is the [`Handler`] of its own byte stream.
//!
//! Every sequence that is not text ends the grapheme cluster being built (mode 2027): the cursor
//! may have moved, and the next character starts a cluster of its own.

use crate::parser::{Csi, Handler};

use crate::terminal::{Terminal, Terminator};

impl Handler for Terminal {
    #[inline]
    fn eight_bit_controls(&self) -> bool {
        self.eight_bit
    }

    #[inline]
    fn print_ascii(&mut self, run: &[u8]) {
        self.print_ascii_run(run);
    }

    #[inline]
    fn print(&mut self, c: char) {
        self.print_char(c);
    }

    fn execute(&mut self, byte: u8) {
        self.cluster.open = false;
        self.history_just_cleared = false;
        self.control(byte);
    }

    fn csi(&mut self, csi: &Csi) {
        self.cluster.open = false;
        self.dispatch_csi(csi);
    }

    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.cluster.open = false;
        self.history_just_cleared = false;
        self.dispatch_esc(intermediates, byte);
    }

    fn osc_start(&mut self) {
        self.osc.start();
    }

    fn osc_put(&mut self, data: &[u8]) {
        self.osc.put(data);
    }

    fn osc_end(&mut self, bel_terminated: bool) {
        self.cluster.open = false;
        self.finish_osc(if bel_terminated { Terminator::Bel } else { Terminator::St });
    }

    fn osc_cancel(&mut self) {
        self.osc.cancel();
    }

    fn dcs_hook(&mut self, csi: &Csi) {
        self.cluster.open = false;
        self.hook_dcs(csi);
    }

    fn dcs_put(&mut self, data: &[u8]) {
        self.dcs.put(data);
    }

    fn dcs_unhook(&mut self) {
        self.unhook_dcs();
    }

    fn dcs_cancel(&mut self) {
        self.dcs.cancel();
    }

    fn apc_start(&mut self) {
        self.dcs.start_apc();
    }

    fn apc_put(&mut self, data: &[u8]) {
        self.put_apc(data);
    }

    fn apc_end(&mut self) {
        self.cluster.open = false;
        self.finish_apc();
    }

    fn apc_cancel(&mut self) {
        self.cancel_apc();
    }
}
