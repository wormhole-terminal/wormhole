//! XTMODKEYS and XTQMODKEYS: the key-modifier options, of which the terminal keeps modifyOtherKeys.
//!
//! Spec: xterm's ctlseqs. `CSI > Pp ; Pv m` sets the resource `Pp` names to `Pv` and, with `Pv`
//! left out, resets it to its initial value; with no parameters it resets them all. `CSI ? Pp m`
//! asks, and the answer is `CSI > Pp ; Pv m`, a sequence that sets the value back. `Pp` 4 is
//! modifyOtherKeys, which has levels 0 (initial), 1 and 2 here (xterm's 3 and its modifier mask
//! `CSI > 4 : Pm m` are not read); the other resources (modifyCursorKeys, modifyFunctionKeys, …)
//! change how the cursor and function keys are written, which the host's key encoder has no
//! switch for, so they are neither kept nor answered. The level is part of [`Modes`], where the
//! host's key encoder reads it ([`Modes::modify_other_keys`]); DECSTR and RIS reset it, as xterm's do.

use crate::parser::Csi;

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// The resource number of modifyOtherKeys.
const MODIFY_OTHER_KEYS: u16 = 4;

impl Terminal {
    /// XTMODKEYS (`CSI > Pp ; Pv m`).
    pub(in crate::terminal) fn set_key_modifier_options(&mut self, csi: &Csi) {
        // A colon in the parameters is the modifier mask, which is not read.
        if csi.has_subparams() {
            return;
        }
        let level = if csi.len() <= 1 && csi.get(0).is_none() {
            // `CSI > m`: every resource goes back to its initial value.
            Some(0)
        } else if csi.get(0) == Some(MODIFY_OTHER_KEYS) {
            // The value left out is the initial one.
            match csi.get(1) {
                None => Some(0),
                Some(level @ 0..=2) => Some(level),
                Some(_) => None,
            }
        } else {
            None
        };
        if let Some(level) = level {
            self.modes.remove(Modes::MODIFY_OTHER_KEYS | Modes::MODIFY_OTHER_KEYS_ALL);
            if level >= 1 {
                self.modes.insert(Modes::MODIFY_OTHER_KEYS);
            }
            if level == 2 {
                self.modes.insert(Modes::MODIFY_OTHER_KEYS_ALL);
            }
        }
    }

    /// XTQMODKEYS (`CSI ? Pp m`).
    pub(in crate::terminal) fn report_key_modifier_options(&mut self, resource: u16) {
        if resource != MODIFY_OTHER_KEYS {
            return;
        }
        let level = if self.modes.contains(Modes::MODIFY_OTHER_KEYS_ALL) {
            2
        } else {
            u8::from(self.modes.contains(Modes::MODIFY_OTHER_KEYS))
        };
        self.reply(format_args!("\x1b[>{MODIFY_OTHER_KEYS};{level}m"));
    }
}
