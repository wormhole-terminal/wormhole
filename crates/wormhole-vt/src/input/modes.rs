//! The terminal modes the encoders read: [`InputModes`].
//!
//! The encoders are pure functions of an event and these modes, so they work for a host that keeps
//! its own mode state as well as for one that reads them off a [`Terminal`](crate::Terminal).
//! The one coupling to the engine's mode type is `From<Modes>` below.

pub use crate::types::{KittyFlags, Modes, ModifyOtherKeys, MouseEncoding, MouseTracking};

/// The terminal state that decides how input is encoded.
///
/// Start from [`InputModes::default`] (every mode off, as a terminal starts) and set what the
/// program switched on, or build it from the engine's [`Modes`] with `InputModes::from`. The
/// struct grows with the protocols, so it cannot be written as a literal outside this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct InputModes {
    /// DECCKM (1): unmodified cursor keys send `ESC O x` instead of `ESC [ x`.
    pub app_cursor: bool,
    /// DECKPAM / DECNKM (66): the keypad sends application sequences instead of its characters.
    pub app_keypad: bool,
    /// DECBKM (67): Backspace sends BS (0x08) and Control+Backspace DEL; reset (the default),
    /// DEL and BS, as xterm's `backarrowKey` does.
    pub backarrow_bs: bool,
    /// The kitty keyboard flags of the screen showing. Any flag set switches the key encoding to
    /// the kitty protocol.
    pub kitty: KittyFlags,
    /// xterm's modifyOtherKeys level (`CSI > 4 ; Pv m`): with the legacy encoding, keys that
    /// have no function-key form are sent as `CSI 27 ; modifiers ; code ~` when a modifier is
    /// held. The kitty flags take precedence when set.
    pub modify_other_keys: ModifyOtherKeys,
    /// The terminal is in VT52 mode (DECANM reset): cursor keys send `ESC A` to `ESC D` whatever
    /// DECCKM says, the keypad `ESC ? x` in application mode, and nothing is sent that needs a
    /// `CSI` (mouse, focus and paste reports, function keys but F1 to F4 as PF1 to PF4).
    pub vt52: bool,
    /// 2004: pastes are wrapped in `CSI 200 ~` / `CSI 201 ~`.
    pub bracketed_paste: bool,
    /// 1004: the program wants focus in and out reports.
    pub focus_events: bool,
    /// The mouse reporting level.
    pub mouse: MouseTracking,
    /// The mouse report format.
    pub mouse_encoding: MouseEncoding,
}

impl InputModes {
    /// Whether a program asked for mouse reports at all.
    pub fn reports_mouse(&self) -> bool {
        self.mouse != MouseTracking::Off
    }
}

/// What the engine's modes say about input: a host that reads
/// [`Terminal::modes`](crate::Terminal::modes) turns them into the encoders' input with this.
impl From<Modes> for InputModes {
    fn from(modes: Modes) -> InputModes {
        InputModes {
            app_cursor: modes.contains(Modes::APP_CURSOR),
            app_keypad: modes.contains(Modes::APP_KEYPAD),
            backarrow_bs: modes.contains(Modes::BACKARROW_BS),
            kitty: modes.kitty_flags(),
            modify_other_keys: modes.modify_other_keys(),
            vt52: modes.contains(Modes::VT52),
            bracketed_paste: modes.contains(Modes::BRACKETED_PASTE),
            focus_events: modes.contains(Modes::FOCUS_REPORT),
            mouse: modes.mouse_tracking(),
            mouse_encoding: modes.mouse_encoding(),
        }
    }
}

impl From<&Modes> for InputModes {
    fn from(modes: &Modes) -> InputModes {
        InputModes::from(*modes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_lands_in_its_field() {
        let modes = InputModes::from(
            Modes::APP_CURSOR
                | Modes::APP_KEYPAD
                | Modes::BACKARROW_BS
                | Modes::BRACKETED_PASTE
                | Modes::FOCUS_REPORT
                | Modes::MOUSE_DRAG
                | Modes::MOUSE_UTF8
                | Modes::KEYS_DISAMBIGUATE
                | Modes::KEYS_ASSOCIATED_TEXT
                | Modes::VT52
                | Modes::MODIFY_OTHER_KEYS
                | Modes::MODIFY_OTHER_KEYS_ALL,
        );
        assert!(modes.vt52);
        assert_eq!(modes.modify_other_keys, ModifyOtherKeys::Level2);
        assert!(modes.app_cursor && modes.app_keypad && modes.backarrow_bs);
        assert!(modes.bracketed_paste && modes.focus_events);
        assert_eq!(modes.mouse, MouseTracking::Drag);
        assert_eq!(modes.mouse_encoding, MouseEncoding::Utf8);
        assert_eq!(modes.kitty, KittyFlags::DISAMBIGUATE | KittyFlags::REPORT_ASSOCIATED_TEXT);
        assert_eq!(InputModes::from(Modes::empty()), InputModes::default());
    }

    #[test]
    fn the_starting_modes_encode_like_the_default() {
        assert_eq!(InputModes::from(Modes::INITIAL), InputModes::default());
        assert_eq!(InputModes::from(&Modes::INITIAL), InputModes::default());
    }

    #[test]
    fn the_modes_that_were_out_of_reach_land_too() {
        let modes = InputModes::from(Modes::MOUSE_X10 | Modes::MOUSE_PIXELS);
        assert_eq!((modes.mouse, modes.mouse_encoding), (MouseTracking::X10, MouseEncoding::SgrPixels));
        assert_eq!(InputModes::from(Modes::MOUSE_URXVT).mouse_encoding, MouseEncoding::Urxvt);
    }
}
