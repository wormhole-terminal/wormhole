//! ESC sequences: the ones with a final byte, and the character set designations.
//!
//! Spec: xterm's ctlseqs ("Controls beginning with ESC") and the VT510 manual: DECSC/DECRC (7, 8),
//! IND, NEL, HTS, RI, RIS, DECKPAM/DECKPNM (=, >), DECBI/DECFI (6, 9), the single and locking
//! shifts, DECALN (`# 8`), the line sizes (`# 3` to `# 6`, see `screen/line_size`), DECID (Z), and
//! `ESC ( ) * +` + final to designate G0..G3. In VT52 mode the VT52's table is read instead (`vt52`).

use crate::terminal::Terminal;
use crate::terminal::modes::{Modes, recorded_bit};
use crate::terminal::text::charset;
use crate::types::LineSize;

impl Terminal {
    pub(in crate::terminal) fn dispatch_esc(&mut self, intermediates: &[u8], byte: u8) {
        if self.modes.contains(Modes::VT52) {
            self.dispatch_vt52(byte);
            return;
        }
        match (intermediates, byte) {
            ([], b'7') => self.save_cursor(),
            ([], b'8') => self.restore_cursor(),
            ([], b'D') => self.index(),
            ([], b'E') => self.next_line(),
            ([], b'H') => self.tabs.set(self.cursor.x),
            ([], b'M') => self.reverse_index(),
            ([], b'c') => self.reset(),
            ([], b'6') => self.back_index(),
            ([], b'9') => self.forward_index(),
            ([], b'=') => self.modes.insert(Modes::APP_KEYPAD),
            ([], b'>') => self.modes.remove(Modes::APP_KEYPAD),
            ([], b'Z') => self.primary_attributes(),
            ([b' '], b'F') => self.set_eight_bit_controls(false),
            ([b' '], b'G') => self.set_eight_bit_controls(true),
            // SPA and EPA: the cells written between them are guarded (ECMA-48 8.3.129, 8.3.54).
            ([], b'V') => {
                self.pen.guarded = true;
                self.guard_used = true;
            }
            ([], b'W') => self.pen.guarded = false,
            ([], b'N') => self.charsets.single_shift(2),
            ([], b'O') => self.charsets.single_shift(3),
            ([], b'n') => self.charsets.invoke(2),
            ([], b'o') => self.charsets.invoke(3),
            // LS1R, LS2R, LS3R put a set in GR, which UTF-8 text never shows (DECCIR reports it).
            ([], b'~') => self.charsets.invoke_right(1),
            ([], b'}') => self.charsets.invoke_right(2),
            ([], b'|') => self.charsets.invoke_right(3),
            // DECDHL top and bottom, DECSWL and DECDWL: the cursor's row changes size.
            ([b'#'], b'3') => self.set_line_size(LineSize::DoubleHeightTop),
            ([b'#'], b'4') => self.set_line_size(LineSize::DoubleHeightBottom),
            ([b'#'], b'5') => self.set_line_size(LineSize::Single),
            ([b'#'], b'6') => self.set_line_size(LineSize::DoubleWidth),
            ([b'#'], b'8') => self.screen_alignment(),
            (_, final_byte) => self.designate_charset(intermediates, final_byte),
        }
    }

    /// SCS: `ESC I Dscs`. A set the conformance level does not have is not designated, and neither
    /// is a national replacement set unless DECNRCM is set (VT510 DECNRCM: the multinational mode
    /// has none): the slot keeps what it held.
    fn designate_charset(&mut self, intermediates: &[u8], final_byte: u8) {
        let Some((slot, set)) = charset::designation(intermediates, final_byte, self.preferred) else {
            return;
        };
        if self.level < set.level() || (set.is_national() && !self.national_replacement_mode()) {
            return;
        }
        self.charsets.designate(slot, set);
    }

    /// Whether DECNRCM (private mode 42) is set.
    pub(in crate::terminal) fn national_replacement_mode(&self) -> bool {
        recorded_bit(42).is_some_and(|bit| self.recorded & bit != 0)
    }
}
