//! The conformance level (DECSCL) and how control characters are sent and received (S7C1T,
//! S8C1T).
//!
//! Spec: VT510 DECSCL ("the terminal does a hard reset when the level changes"; levels 61..=65 are
//! VT100 mode and the VT200 to VT500 families, the second parameter picks 7-bit or 8-bit C1
//! controls and is ignored in VT100 mode) and S7C1T/S8C1T (`ESC SP F`, `ESC SP G`); xterm's ctlseqs
//! for the forms and for which level answers DA with what.
//!
//! The level starts at 5, where everything the engine can do is on, and a program may take it
//! down: a level answers DA1 and DA2 as its family did, and the features of the higher families
//! are off (DECRQM is a VT300 feature, left and right margins VT400's, DECNCSM VT500's).
//!
//! 8-bit controls: the parser reads a C1 byte only in this mode, and the replies use the one-byte
//! C1 controls (CSI is 0x9B) instead of `ESC [`. A terminal does not enter it by itself, but
//! DECSCL for a level above VT100 does as the VT510 manual says: without its second parameter, or
//! with 0 or 2, the controls are 8-bit; only 1 keeps them 7-bit. (The bytes of an 8-bit reply are
//! not UTF-8: a program that asks for that has said it reads them.)

use crate::parser::Csi;

use crate::terminal::Terminal;

/// The conformance level: which DEC family the terminal says it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::terminal) enum Level {
    /// VT100 (DECSCL 61).
    Vt100 = 1,
    /// VT200 (62).
    Vt200,
    /// VT300 (63): adds DECRQM.
    Vt300,
    /// VT400 (64): adds left and right margins and rectangular editing.
    Vt400,
    /// VT500 (65): adds DECNCSM.
    Vt500,
}

impl Level {
    /// The level DECSCL's first parameter (61..=65) names.
    fn from_param(ps: u16) -> Option<Level> {
        Some(match ps {
            61 => Level::Vt100,
            62 => Level::Vt200,
            63 => Level::Vt300,
            64 => Level::Vt400,
            65 => Level::Vt500,
            _ => return None,
        })
    }

    /// DECSCL's first parameter for this level.
    pub(in crate::terminal) fn param(self) -> u16 {
        59 + self as u16 + 1
    }

    /// The first parameter of the secondary device attributes reply: the terminal type that
    /// family has (xterm's ctlseqs: 0 VT100, 1 VT220, 24 VT320, 41 VT420, 64 VT520).
    pub(in crate::terminal) fn terminal_type(self) -> u16 {
        match self {
            Level::Vt100 => 0,
            Level::Vt200 => 1,
            Level::Vt300 => 24,
            Level::Vt400 => 41,
            Level::Vt500 => 64,
        }
    }
}

impl Terminal {
    /// DECSCL (`CSI Ps ; Ps " p`).
    pub(in crate::terminal) fn set_conformance_level(&mut self, csi: &Csi) {
        let Some(level) = Level::from_param(csi.param(0, 0)) else { return };
        // A change of level is a hard reset (VT510, DECSCL).
        self.reset();
        self.level = level;
        self.eight_bit = level > Level::Vt100 && matches!(csi.get(1), None | Some(0 | 2));
    }

    /// S7C1T and S8C1T.
    pub(in crate::terminal) fn set_eight_bit_controls(&mut self, on: bool) {
        // VT100 mode only has 7-bit controls.
        self.eight_bit = on && self.level > Level::Vt100;
    }
}

/// Turn the introducers in `buf[from..]` into their one-byte forms: `ESC [` CSI, `ESC P` DCS,
/// `ESC ]` OSC, `ESC \` ST and `ESC _` APC. The text of a reply is ASCII, so nothing else in it
/// is an ESC.
pub(in crate::terminal) fn eight_bit_in_place(buf: &mut Vec<u8>, from: usize) {
    let mut write = from;
    let mut read = from;
    while read < buf.len() {
        let byte = buf[read];
        let one = match (byte, buf.get(read + 1)) {
            (0x1B, Some(b'[')) => Some(0x9B),
            (0x1B, Some(b'P')) => Some(0x90),
            (0x1B, Some(b']')) => Some(0x9D),
            (0x1B, Some(b'\\')) => Some(0x9C),
            (0x1B, Some(b'_')) => Some(0x9F),
            _ => None,
        };
        if let Some(one) = one {
            buf[write] = one;
            read += 2;
        } else {
            buf[write] = byte;
            read += 1;
        }
        write += 1;
    }
    buf.truncate(write);
}

#[cfg(test)]
mod tests {
    use super::eight_bit_in_place;

    #[test]
    fn introducers_become_one_byte() {
        let mut buf = b"keep\x1b[1;2R\x1bP1$r0m\x1b\\\x1b]4;1;rgb:00/00/00\x1b\\".to_vec();
        eight_bit_in_place(&mut buf, 4);
        assert_eq!(buf, b"keep\x9b1;2R\x901$r0m\x9c\x9d4;1;rgb:00/00/00\x9c");
    }
}
