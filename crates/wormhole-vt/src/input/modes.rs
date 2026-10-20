//! The terminal modes the encoders read: [`InputModes`].
//!
//! The encoders are pure functions of an event and these modes, so they work for a host that keeps
//! its own mode state as well as for one that reads them off a [`Terminal`](crate::Terminal).
//! The one coupling to the engine's mode type is `From<TermMode>` below.

use bitflags::bitflags;

use crate::types::TermMode;

bitflags! {
    /// The kitty keyboard protocol's progressive-enhancement flags, with the protocol's own bit
    /// values (<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct KittyFlags: u8 {
        /// Flag 1: disambiguate escape codes.
        const DISAMBIGUATE          = 1;
        /// Flag 2: report event types (repeat and release as well as press).
        const REPORT_EVENT_TYPES    = 1 << 1;
        /// Flag 4: report alternate keys (the shifted and base-layout key codes).
        const REPORT_ALTERNATE_KEYS = 1 << 2;
        /// Flag 8: report all keys as escape codes, text keys and modifier keys too.
        const REPORT_ALL_KEYS       = 1 << 3;
        /// Flag 16: report the text a key produces together with the escape code.
        const REPORT_ASSOCIATED_TEXT = 1 << 4;
    }
}

/// Which pointer events a program asked to be told about (the DECSET numbers in brackets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MouseTracking {
    /// No mouse reporting.
    #[default]
    Off,
    /// X10 compatibility (9): button presses only, without modifiers.
    X10,
    /// Normal tracking (1000): presses and releases.
    Click,
    /// Button-event tracking (1002): and motion while a button is down.
    Drag,
    /// Any-event tracking (1003): and all motion.
    Motion,
}

/// How a mouse report writes its numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MouseEncoding {
    /// The X10 form, one byte per number: positions past column or row 223 cannot be written.
    #[default]
    X10,
    /// UTF-8 numbers (1005): positions up to 2015.
    Utf8,
    /// URXVT's decimal numbers (1015).
    Urxvt,
    /// SGR (1006): `CSI <`, decimal numbers, and a release has its own final byte.
    Sgr,
    /// SGR with pixel positions (1016); falls back to cells for an event without a pixel.
    SgrPixels,
}

/// The terminal state that decides how input is encoded.
///
/// Start from [`InputModes::default`] (every mode off, as a terminal starts) and set what the
/// program switched on, or build it from the engine's [`TermMode`] with `InputModes::from`. The
/// struct grows with the protocols, so it cannot be written as a literal outside this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct InputModes {
    /// DECCKM (1): unmodified cursor keys send `ESC O x` instead of `ESC [ x`.
    pub app_cursor: bool,
    /// DECKPAM / DECNKM (66): the keypad sends application sequences instead of its characters.
    pub app_keypad: bool,
    /// The kitty keyboard flags of the screen showing. Any flag set switches the key encoding to
    /// the kitty protocol.
    pub kitty: KittyFlags,
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

/// The modes the engine keeps in its `TermMode` set. It does not (yet) carry the keypad mode,
/// X10 tracking, URXVT or pixel positions, so those stay off; a host that tracks them can set the
/// fields itself.
impl From<TermMode> for InputModes {
    fn from(mode: TermMode) -> InputModes {
        let kitty_bits = [
            (TermMode::DISAMBIGUATE_ESC_CODES, KittyFlags::DISAMBIGUATE),
            (TermMode::REPORT_EVENT_TYPES, KittyFlags::REPORT_EVENT_TYPES),
            (TermMode::REPORT_ALTERNATE_KEYS, KittyFlags::REPORT_ALTERNATE_KEYS),
            (TermMode::REPORT_ALL_KEYS_AS_ESC, KittyFlags::REPORT_ALL_KEYS),
            (TermMode::REPORT_ASSOCIATED_TEXT, KittyFlags::REPORT_ASSOCIATED_TEXT),
        ];
        let kitty = kitty_bits
            .into_iter()
            .filter(|(ours, _)| mode.contains(*ours))
            .fold(KittyFlags::empty(), |all, (_, flag)| all | flag);
        let mouse = if mode.contains(TermMode::MOUSE_MOTION) {
            MouseTracking::Motion
        } else if mode.contains(TermMode::MOUSE_DRAG) {
            MouseTracking::Drag
        } else if mode.contains(TermMode::MOUSE_REPORT_CLICK) {
            MouseTracking::Click
        } else {
            MouseTracking::Off
        };
        let mouse_encoding = if mode.contains(TermMode::SGR_MOUSE) {
            MouseEncoding::Sgr
        } else if mode.contains(TermMode::UTF8_MOUSE) {
            MouseEncoding::Utf8
        } else {
            MouseEncoding::X10
        };
        InputModes {
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            app_keypad: false,
            kitty,
            bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
            focus_events: mode.contains(TermMode::FOCUS_IN_OUT),
            mouse,
            mouse_encoding,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_mode_lands_in_its_field() {
        let modes = InputModes::from(
            TermMode::APP_CURSOR
                | TermMode::BRACKETED_PASTE
                | TermMode::FOCUS_IN_OUT
                | TermMode::MOUSE_DRAG
                | TermMode::UTF8_MOUSE
                | TermMode::DISAMBIGUATE_ESC_CODES
                | TermMode::REPORT_ASSOCIATED_TEXT,
        );
        assert!(modes.app_cursor && modes.bracketed_paste && modes.focus_events);
        assert_eq!(modes.mouse, MouseTracking::Drag);
        assert_eq!(modes.mouse_encoding, MouseEncoding::Utf8);
        assert_eq!(modes.kitty, KittyFlags::DISAMBIGUATE | KittyFlags::REPORT_ASSOCIATED_TEXT);
        assert_eq!(InputModes::from(TermMode::empty()), InputModes::default());
    }

    #[test]
    fn the_widest_tracking_and_the_richest_encoding_win() {
        let all = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION;
        assert_eq!(InputModes::from(all).mouse, MouseTracking::Motion);
        let both = TermMode::SGR_MOUSE | TermMode::UTF8_MOUSE;
        assert_eq!(InputModes::from(both).mouse_encoding, MouseEncoding::Sgr);
    }
}
