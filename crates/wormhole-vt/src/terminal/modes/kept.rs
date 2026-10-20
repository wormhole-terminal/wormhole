//! Settings of hardware a terminal emulator does not have, kept so that DECRQSS reports what a
//! program set: the local functions (DECELF), local function key control (DECLFKC) and modifier
//! key reporting (DECSMKR). DECSACE's number is kept the way it was sent as well (0 and 1 both mean a stream of cells).
//!
//! Spec: VT510 manual, the settings DECRQSS can report (`DCS $ q Pt ST`, answered as the sequence
//! that sets them) and the sequences themselves: `CSI Ps + q`, `CSI Ps * }`, `CSI Ps $ }`,
//! `CSI Ps + r`, `CSI Ps $ ~`.

use crate::parser::Csi;

use crate::terminal::Terminal;

/// What a program set, as the numbers it sent; all zero until it sends any.
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::terminal) struct Kept {
    /// DECELF.
    pub(in crate::terminal) local_functions: u16,
    /// DECLFKC.
    pub(in crate::terminal) function_key_control: u16,
    /// DECSMKR.
    pub(in crate::terminal) modifier_keys: u16,
    /// DECSACE.
    pub(in crate::terminal) attribute_extent: u16,
}

impl Terminal {
    /// The sequences that only set a number to be kept: DECELF `+q`, DECSMKR `+r` and DECLFKC `*}`;
    /// and the status line's two, DECSASD `$}` and DECSSDT `$~`, which have a module of their own
    /// (`screen::status_line`). `true` when `csi` is one of them.
    pub(in crate::terminal) fn keep_setting(&mut self, csi: &Csi) -> bool {
        let value = csi.param(0, 0);
        match (csi.intermediates(), csi.final_byte()) {
            ([b'+'], b'q') => self.kept.local_functions = value,
            ([b'+'], b'r') => self.kept.modifier_keys = value,
            ([b'*'], b'}') => self.kept.function_key_control = value,
            ([b'$'], b'}') => self.select_status_display(value),
            ([b'$'], b'~') => self.select_status_line_type(value),
            _ => return false,
        }
        true
    }
}
