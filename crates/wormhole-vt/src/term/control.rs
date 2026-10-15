//! The C0 controls the terminal acts on.
//!
//! Spec: ECMA-48 8.3 for BEL, BS, HT, LF, VT, FF, CR, SO, SI; VT510 Table 4-1 for what each does
//! at the margins. ENQ would send the answerback message, which is empty by default; NUL, DEL,
//! CAN and SUB (the parser has already used them up) and the rest do nothing.

use super::{Event, Terminal};

impl Terminal {
    pub(super) fn control(&mut self, byte: u8) {
        match byte {
            // A flood of BELs is one bell (`push_event` folds them).
            0x07 => self.push_event(Event::Bell),
            0x08 => self.backspace(),
            0x09 => self.tab_forward(1),
            0x0a..=0x0c => self.line_feed(),
            0x0d => self.carriage_return(),
            0x0e => self.charsets.invoke(1),
            0x0f => self.charsets.invoke(0),
            _ => {}
        }
    }
}
