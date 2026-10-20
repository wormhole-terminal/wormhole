//! SM, RM, DECSET and DECRST: what switching each mode does, and what DECRQM answers.
//!
//! Most modes are a flag some other code reads. The ones that act when switched: DECCOLM (3,
//! resizes), DECOM (6, homes the cursor), the alternate screen (47, 1047, 1049), the cursor save
//! (1048), DECLRMM (69, margins), grapheme clusters (2027, ends the cluster being built) and
//! reverse video (5, redraws everything).

use crate::parser::Csi;

use crate::terminal::Terminal;
use crate::terminal::modes::conformance::Level;
use crate::terminal::modes::{
    Modes, Status, Switch, ansi_fixed, ansi_flag, fixed_status, private_flag, recorded_bit,
};

/// How many modes XTSAVE remembers.
const SAVED_MAX: usize = 64;

impl Terminal {
    /// SM and RM for ANSI mode `n`.
    pub(in crate::terminal) fn set_ansi_mode(&mut self, n: u16, switch: Switch) {
        if let Some(flag) = ansi_flag(n) {
            self.modes.set(flag, switch.is_set());
        }
    }

    /// DECSET and DECRST for DEC private mode `n`.
    pub(in crate::terminal) fn set_private_mode(&mut self, n: u16, switch: Switch) {
        let on = switch.is_set();
        // The locator and mouse tracking are alternatives for the pointer.
        if on && matches!(n, 9 | 1000 | 1002 | 1003) {
            self.locator.disable();
        }
        match n {
            3 => self.set_column_mode(switch),
            5 => {
                self.modes.set(Modes::REVERSE_VIDEO, on);
                self.grid.damage_all();
                self.other.damage_all();
            }
            6 => {
                self.modes.set(Modes::ORIGIN, on);
                self.home();
            }
            12 => self.cursor_style.blink = Some(on),
            47 | 1047 | 1049 => self.set_alt_screen_mode(n, switch),
            1048 => {
                if on {
                    self.save_cursor();
                } else {
                    self.restore_cursor();
                }
            }
            // A VT400 feature.
            69 if self.level < Level::Vt400 => {}
            69 => {
                self.modes.set(Modes::LEFT_RIGHT_MARGINS, on);
                if !on {
                    self.margins.left = 0;
                    self.margins.right = self.cols - 1;
                    // Due at the old right margin, which is no longer an edge: nothing wraps there.
                    self.cursor.pending &= self.cursor.x == self.right_limit();
                }
            }
            2027 => {
                self.modes.set(Modes::GRAPHEME_CLUSTERS, on);
                self.cluster.open = false;
            }
            // DECANM: reset is VT52 mode; set is the ANSI mode the terminal is in already.
            2 => {
                if !on {
                    self.enter_vt52();
                }
            }
            // The size is told now, whether or not the mode was set (the gist: "the terminal MUST
            // immediately report the current size if an attempt is made to enable the feature").
            2048 => {
                self.modes.set(Modes::INBAND_RESIZE, on);
                if on {
                    self.report_inband_size();
                } else {
                    self.forget_inband_size();
                }
            }
            _ => {
                if let Some(flag) = private_flag(n) {
                    self.modes.set(flag, on);
                } else if let Some(bit) = recorded_bit(n) {
                    if on {
                        self.recorded |= bit;
                    } else {
                        self.recorded &= !bit;
                    }
                }
            }
        }
    }

    /// XTSAVE (`CSI ? Pm s`): remember the state of these DEC private modes, for XTRESTORE. A
    /// full-screen program saves what it is about to change and puts it back on the way out.
    pub(in crate::terminal) fn save_private_modes(&mut self, csi: &Csi) {
        for i in 0..csi.len() {
            let Some(n) = csi.get(i) else { continue };
            let on = self.private_status(n) == Status::Set;
            self.saved_modes.retain(|&(mode, _)| mode != n);
            if self.saved_modes.len() < SAVED_MAX {
                self.saved_modes.push((n, if on { Switch::Set } else { Switch::Reset }));
            }
        }
    }

    /// XTRESTORE (`CSI ? Pm r`): set the modes back to what XTSAVE remembered.
    pub(in crate::terminal) fn restore_private_modes(&mut self, csi: &Csi) {
        for i in 0..csi.len() {
            let Some(n) = csi.get(i) else { continue };
            if let Some(&(_, switch)) = self.saved_modes.iter().find(|&&(mode, _)| mode == n) {
                self.set_private_mode(n, switch);
            }
        }
    }

    /// What DECRQM answers for ANSI mode `n`.
    pub(in crate::terminal) fn ansi_status(&self, n: u16) -> Status {
        match ansi_flag(n) {
            Some(flag) => Status::of(self.modes.contains(flag)),
            None => ansi_fixed(n).unwrap_or(Status::NotRecognized),
        }
    }

    /// What DECRQM answers for DEC private mode `n`.
    pub(in crate::terminal) fn private_status(&self, n: u16) -> Status {
        match n {
            47 | 1047 | 1049 => Status::of(self.on_alt_screen()),
            12 => Status::of(self.cursor_style.blink == Some(true)),
            _ => {
                if let Some(status) = fixed_status(n) {
                    status
                } else if let Some(flag) = private_flag(n) {
                    Status::of(self.modes.contains(flag))
                } else if let Some(bit) = recorded_bit(n) {
                    Status::of(self.recorded & bit != 0)
                } else {
                    Status::NotRecognized
                }
            }
        }
    }
}
