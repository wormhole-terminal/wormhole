//! DECRQCRA: the checksum of a rectangle of the screen, and the policy that turns it on.
//!
//! Spec: xterm's ctlseqs (`CSI Pi ; Pg ; Pt ; Pl ; Pb ; Pr * y`, answered `DCS Pi ! ~ xxxx ST`
//! with four hex digits) and the VT510 manual (DECRQCRA, the rectangle as in DECCARA).
//!
//! The sum is xterm's with the XTCHECKSUM extension switched on for bits 0 and 1: not negated
//! (the sum of the cells, not its two's complement), and without the VT100 video attributes. Each
//! cell is its code point (a combining mark adds nothing; the second half of a wide character
//! adds nothing) and a blank cell, written or erased, is a space (0x20); the sum is taken modulo
//! 65,536. That is what a program that reads the screen back cell by cell (esctest) expects.
//!
//! **Off unless the host asks**: the checksum of a single cell tells a program what is in it, so
//! any program, also one on the other end of an ssh connection, could read the screen back
//! cell by cell. While [`Policy::screen_checksum`](crate::terminal::Policy::screen_checksum) is off, the request is ignored like an unknown
//! sequence and nothing is answered.

use crate::parser::Csi;

use crate::terminal::Terminal;

impl Terminal {
    /// DECRQCRA (`CSI Pi ; Pg ; Pt ; Pl ; Pb ; Pr * y`). `Pi` is echoed; `Pg`, the page, can only
    /// be 0 or 1 (there is one page), and any other asks about a page that is not there. A
    /// rectangle that is upside down has no cells and sums to 0.
    pub(in crate::terminal) fn checksum_request(&mut self, csi: &Csi) {
        if !self.policy.screen_checksum || csi.param(1, 0) > 1 {
            return;
        }
        let id = csi.param(0, 0);
        let sum =
            self.corners(csi, 2).region(self.cols, self.rows).map_or(0, |area| self.grid.checksum(area));
        self.reply(format_args!("\x1bP{id}!~{sum:04X}\x1b\\"));
    }
}
