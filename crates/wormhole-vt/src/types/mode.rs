//! The terminal modes the app reads.

use bitflags::bitflags;

bitflags! {
    /// The modes programs switch (DEC private modes, ctlseqs' "DEC Private Mode Set") and the
    /// kitty keyboard protocol's flags that change what the app sends or how it draws.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct TermMode: u32 {
        /// DECTCEM (25): the cursor is shown.
        const SHOW_CURSOR      = 1;
        /// DECAWM (7): a character past the last column wraps to the next row.
        const LINE_WRAP        = 1 << 1;
        /// The alternate screen (47, 1047, 1049): a full-screen program has it, and it has no
        /// history.
        const ALT_SCREEN       = 1 << 2;
        /// 1007: the wheel sends arrow keys on the alternate screen.
        const ALTERNATE_SCROLL = 1 << 3;
        /// 2004.
        const BRACKETED_PASTE  = 1 << 4;
        /// 1004: report the window gaining and losing focus.
        const FOCUS_IN_OUT     = 1 << 5;
        /// DECCKM (1): the arrow keys send `ESC O` instead of `ESC [`.
        const APP_CURSOR       = 1 << 6;

        /// 1000: report button presses.
        const MOUSE_REPORT_CLICK = 1 << 7;
        /// 1002: and motion while a button is down.
        const MOUSE_DRAG         = 1 << 8;
        /// 1003: and all motion.
        const MOUSE_MOTION       = 1 << 9;
        /// 1006: reports as `CSI <`.
        const SGR_MOUSE          = 1 << 10;
        /// 1005: reports with UTF-8 coordinates.
        const UTF8_MOUSE         = 1 << 11;

        /// The kitty keyboard protocol's flags, as the program pushed them.
        const DISAMBIGUATE_ESC_CODES = 1 << 12;
        const REPORT_EVENT_TYPES     = 1 << 13;
        const REPORT_ALTERNATE_KEYS  = 1 << 14;
        const REPORT_ALL_KEYS_AS_ESC = 1 << 15;
        const REPORT_ASSOCIATED_TEXT = 1 << 16;

        /// A program turned the DEC locator on (DECELR): the pointer goes to it as locator reports
        /// (`Terminal::locator_moved`, `locator_button`), not as mouse tracking.
        const LOCATOR = 1 << 17;

        /// Any of the three mouse reporting modes.
        const MOUSE_MODE = Self::MOUSE_REPORT_CLICK.bits() | Self::MOUSE_DRAG.bits() | Self::MOUSE_MOTION.bits();
    }
}

#[cfg(test)]
mod tests {
    use super::TermMode;

    #[test]
    fn mouse_reporting_is_any_of_its_three_modes() {
        for mode in [TermMode::MOUSE_REPORT_CLICK, TermMode::MOUSE_DRAG, TermMode::MOUSE_MOTION] {
            assert!(mode.intersects(TermMode::MOUSE_MODE));
        }
        assert!(!TermMode::SGR_MOUSE.intersects(TermMode::MOUSE_MODE), "an encoding, not a mode");
    }

    #[test]
    fn a_mode_set_starts_empty() {
        assert_eq!(TermMode::default(), TermMode::empty());
    }
}
