//! The terminal modes a host reads: [`Modes`], and the three values derived from its bits that
//! decide how input is encoded ([`MouseTracking`], [`MouseEncoding`], [`KittyFlags`]).
//!
//! `Modes` is the engine's one mode type. The numbers are the DEC private modes' (`CSI ? Ps h`)
//! and the ANSI modes' (`CSI Ps h`); the engine keeps every mode a program can switch, and a
//! host acts on a handful of them, marked **Host** below. The rest are the engine's own business
//! (they change how it places text) and are public because they are bits of the same set.

use bitflags::bitflags;

bitflags! {
    /// Every mode the terminal keeps, as [`Terminal::modes`](crate::Terminal::modes) returns them.
    ///
    /// Read them by name (`modes.contains(Modes::ALT_SCREEN)`) or through the accessors that fold
    /// several bits into one answer ([`mouse_tracking`](Modes::mouse_tracking),
    /// [`mouse_encoding`](Modes::mouse_encoding), [`kitty_flags`](Modes::kitty_flags)). The bit
    /// values are not part of the API and change when modes are added: never store `bits()`.
    ///
    /// The doc of each flag names its number and says what a host does with it (**Host**) or that
    /// the engine alone acts on it (**Engine**).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Modes: u64 {
        /// DECTCEM (`?25`). **Host**: draw the cursor, or not.
        const SHOW_CURSOR      = 1;
        /// DECAWM (`?7`). **Engine**: a character past the last column wraps to the next row.
        const AUTOWRAP         = 1 << 1;
        /// The alternate screen (`?47`, `?1047`, `?1049`) shows. **Host**: it has no history, so
        /// a host hides its scrollbar and may turn the wheel into arrow keys
        /// ([`ALTERNATE_SCROLL`](Modes::ALTERNATE_SCROLL)). Kept in step by the screen switch;
        /// a program does not set it directly.
        const ALT_SCREEN       = 1 << 2;
        /// `?1007`. **Host**: the wheel sends arrow keys on the alternate screen. Starts set.
        const ALTERNATE_SCROLL = 1 << 3;
        /// `?2004`. **Host**: wrap pasted text in markers, see [`input::paste`](crate::input::paste).
        const BRACKETED_PASTE  = 1 << 4;
        /// `?1004`. **Host**: tell the program when the window gains or loses focus, see
        /// [`input::focus`](crate::input::focus).
        const FOCUS_REPORT     = 1 << 5;
        /// DECCKM (`?1`). **Host**: unmodified cursor keys send `ESC O x` instead of `ESC [ x`.
        const APP_CURSOR       = 1 << 6;
        /// `?1000`, X11 mouse reporting. **Host**: report presses and releases. Read the level
        /// through [`mouse_tracking`](Modes::mouse_tracking).
        const MOUSE_CLICK      = 1 << 7;
        /// `?1002`, button-event tracking. **Host**: and motion while a button is down.
        const MOUSE_DRAG       = 1 << 8;
        /// `?1003`, any-event tracking. **Host**: and all motion.
        const MOUSE_MOTION     = 1 << 9;
        /// `?1006`, SGR mouse reports (`CSI <`). **Host**: the report format, see
        /// [`mouse_encoding`](Modes::mouse_encoding).
        const MOUSE_SGR        = 1 << 10;
        /// `?1005`, UTF-8 mouse coordinates. **Host**: the report format.
        const MOUSE_UTF8       = 1 << 11;

        /// Kitty keyboard flag 1: disambiguate escape codes. **Host**: the flags in force on the
        /// screen showing, as the program pushed them; read them together with
        /// [`kitty_flags`](Modes::kitty_flags).
        const KEYS_DISAMBIGUATE   = 1 << 12;
        /// Kitty keyboard flag 2: report event types (press, repeat, release).
        const KEYS_EVENT_TYPES    = 1 << 13;
        /// Kitty keyboard flag 4: report alternate keys.
        const KEYS_ALTERNATE_KEYS = 1 << 14;
        /// Kitty keyboard flag 8: report all keys as escape codes.
        const KEYS_ALL_AS_ESCAPES = 1 << 15;
        /// Kitty keyboard flag 16: report associated text.
        const KEYS_ASSOCIATED_TEXT = 1 << 16;

        /// IRM (ANSI `4`). **Engine**: printing inserts instead of overwriting.
        const INSERT           = 1 << 17;
        /// LNM (ANSI `20`). **Engine**: LF also returns the carriage.
        const NEW_LINE         = 1 << 18;
        /// KAM (ANSI `2`). Recorded: a host decides whether to honour a locked keyboard.
        const KEYBOARD_LOCK    = 1 << 19;
        /// SRM (ANSI `12`). Set means no local echo, the usual state. Recorded.
        const SEND_RECEIVE     = 1 << 20;
        /// DECCOLM (`?3`). **Engine**: the screen is 132 columns wide.
        const COLUMNS_132      = 1 << 21;
        /// DECSCLM (`?4`). Recorded.
        const SMOOTH_SCROLL    = 1 << 22;
        /// DECSCNM (`?5`). **Host**: draw the screen in reverse video.
        const REVERSE_VIDEO    = 1 << 23;
        /// DECOM (`?6`). **Engine**: positions count from the scrolling region.
        const ORIGIN           = 1 << 24;
        /// DECARM (`?8`). Recorded: a host may stop key repeat.
        const AUTO_REPEAT      = 1 << 25;
        /// `?9`, X10 mouse reporting. **Host**: report presses only, without modifiers, see
        /// [`mouse_tracking`](Modes::mouse_tracking).
        const MOUSE_X10        = 1 << 26;
        /// A program asked for the DEC locator (DECELR): the pointer goes to it as locator
        /// reports ([`Terminal::locator_moved`](crate::Terminal::locator_moved),
        /// [`locator_button`](crate::Terminal::locator_button)), not as mouse reports.
        /// **Host**. Not a mode with a number: it is set in the value
        /// [`Terminal::modes`](crate::Terminal::modes) returns while the locator is on.
        const LOCATOR          = 1 << 27;
        /// `?40`. **Engine**: DECCOLM may change the width.
        const ALLOW_COLUMNS    = 1 << 28;
        /// `?45`. **Engine**: BS at the left edge goes to the end of the line above.
        const REVERSE_WRAP     = 1 << 29;
        /// DECNKM (`?66`), also `ESC =` (on) and `ESC >` (off). **Host**: the keypad sends
        /// application sequences.
        const APP_KEYPAD       = 1 << 30;
        /// DECBKM (`?67`). **Host**: the backarrow key sends BS (set) or DEL (reset, the default).
        const BACKARROW_BS     = 1 << 31;
        /// DECLRMM (`?69`). **Engine**: left and right margins.
        const LEFT_RIGHT_MARGINS = 1 << 32;
        /// DECSDM (`?80`). **Engine**: set, a Sixel is drawn at the top left of the screen and the
        /// cursor does not move; reset (the default), it starts at the cursor and the screen scrolls.
        const SIXEL_DISPLAY    = 1 << 33;
        /// DECNCSM (`?95`). **Engine**: DECCOLM does not clear the screen.
        const NO_CLEAR_ON_COLUMNS = 1 << 34;
        /// `?1015`, URXVT mouse coordinates. **Host**: the report format.
        const MOUSE_URXVT      = 1 << 35;
        /// `?1016`, SGR mouse reports with pixel positions. **Host**: the report format; the
        /// host must give the pointer's pixel (see [`MouseEvent::with_pixel`](crate::input::MouseEvent::with_pixel)).
        const MOUSE_PIXELS     = 1 << 36;
        /// Synchronized output (`?2026`). **Host**: a program is in the middle of an update; the
        /// host may hold back drawing until it ends, see
        /// [`Terminal::synchronized_output`](crate::Terminal::synchronized_output).
        const SYNC_OUTPUT      = 1 << 37;
        /// `?2027`. **Engine**: widths follow grapheme clusters. Starts set.
        const GRAPHEME_CLUSTERS = 1 << 38;
        /// `?1070`, recorded only (every picture starts with the VT340's colors).
        const SIXEL_PRIVATE_PALETTE = 1 << 39;
        /// `?8452`. **Engine**: a Sixel leaves the cursor right of the picture's last row, not
        /// under it.
        const SIXEL_CURSOR_RIGHT = 1 << 40;
        /// `?2031`. **Engine**: the program wants to hear when the host changes the colour
        /// scheme; see [`Terminal::set_color_scheme`](crate::Terminal::set_color_scheme).
        const THEME_REPORT     = 1 << 41;
        /// `?1010`. Recorded: a host may scroll the view to the bottom on output.
        const SCROLL_ON_OUTPUT = 1 << 42;
        /// `?1011`. Recorded: a host may scroll the view to the bottom on a key.
        const SCROLL_ON_KEY    = 1 << 43;
        /// `?1036`. Recorded: a host may send ESC before a Meta key.
        const META_SENDS_ESC   = 1 << 44;
        /// `?1039`. Recorded: a host may send ESC before an Alt key.
        const ALT_SENDS_ESC    = 1 << 45;
        /// `?1042`. Recorded: a host may raise the window on the bell.
        const BELL_URGENCY     = 1 << 46;
        /// `?1045`. **Engine**: BS and CUB at the left edge go to the line above even when it did
        /// not wrap, and from the top margin to the bottom one.
        const REVERSE_WRAP_EXTENDED = 1 << 47;
        /// `?41`. **Engine**: a tab with the wrap pending wraps first (xterm's fix for curses
        /// programs that draw to the end of a row and then send a tab).
        const MORE_FIX = 1 << 48;
        /// `?2048`. **Engine**: the host's resizes are reported to the program in band
        /// (`CSI 48 ; rows ; cols ; height ; width t`), see
        /// [`Terminal::resize`](crate::Terminal::resize).
        const INBAND_RESIZE    = 1 << 50;
        /// xterm's modifyOtherKeys is at level 1 or 2 (`CSI > 4 ; Pv m`). **Host**: ordinary keys
        /// with modifiers are sent as `CSI 27 ; modifiers ; code ~`. Read the level through
        /// [`modify_other_keys`](Modes::modify_other_keys).
        const MODIFY_OTHER_KEYS = 1 << 51;
        /// modifyOtherKeys is at level 2 (together with [`MODIFY_OTHER_KEYS`](Modes::MODIFY_OTHER_KEYS)):
        /// all modifiers apply, also to the keys level 1 leaves alone.
        const MODIFY_OTHER_KEYS_ALL = 1 << 52;
    }
}

impl Modes {
    /// The modes a terminal starts with (and RIS restores): the cursor shown, autowrap on,
    /// send/receive set, auto-repeat on, the wheel sending arrows on the alternate screen and
    /// grapheme clustering (2027) on: a program that sizes text by clusters (everything written
    /// since emoji sequences, Indic scripts and Hangul needed it) agrees with the terminal without
    /// asking, and one that counts code points can turn it off. The backarrow key sends DEL
    /// (DECBKM reset), as every terminal a shell runs in does.
    pub const INITIAL: Modes = Modes::SHOW_CURSOR
        .union(Modes::AUTOWRAP)
        .union(Modes::GRAPHEME_CLUSTERS)
        .union(Modes::SEND_RECEIVE)
        .union(Modes::AUTO_REPEAT)
        .union(Modes::ALTERNATE_SCROLL);

    /// The mouse tracking level the program asked for: the widest of the modes set (any-event,
    /// then button-event, then X11, then X10), `Off` when none is.
    ///
    /// A program that set the DEC locator (DECELR) has the pointer as locator reports instead;
    /// check [`LOCATOR`](Modes::LOCATOR) first.
    #[must_use]
    pub fn mouse_tracking(self) -> MouseTracking {
        if self.contains(Modes::MOUSE_MOTION) {
            MouseTracking::Motion
        } else if self.contains(Modes::MOUSE_DRAG) {
            MouseTracking::Drag
        } else if self.contains(Modes::MOUSE_CLICK) {
            MouseTracking::Click
        } else if self.contains(Modes::MOUSE_X10) {
            MouseTracking::X10
        } else {
            MouseTracking::Off
        }
    }

    /// How a mouse report writes its numbers: the richest encoding the program switched on (SGR
    /// with pixels, SGR, URXVT, UTF-8), the X10 form when none is.
    ///
    /// xterm keeps one encoding and the last mode set wins; a set of bits cannot say which was
    /// last, so the richest does. Programs that switch on a richer encoding switch the
    /// others off, or do not mind.
    #[must_use]
    pub fn mouse_encoding(self) -> MouseEncoding {
        if self.contains(Modes::MOUSE_PIXELS) {
            MouseEncoding::SgrPixels
        } else if self.contains(Modes::MOUSE_SGR) {
            MouseEncoding::Sgr
        } else if self.contains(Modes::MOUSE_URXVT) {
            MouseEncoding::Urxvt
        } else if self.contains(Modes::MOUSE_UTF8) {
            MouseEncoding::Utf8
        } else {
            MouseEncoding::X10
        }
    }

    /// The level of xterm's modifyOtherKeys the program set (`CSI > 4 ; Pv m`).
    #[must_use]
    pub fn modify_other_keys(self) -> ModifyOtherKeys {
        if self.contains(Modes::MODIFY_OTHER_KEYS_ALL) {
            ModifyOtherKeys::Level2
        } else if self.contains(Modes::MODIFY_OTHER_KEYS) {
            ModifyOtherKeys::Level1
        } else {
            ModifyOtherKeys::Off
        }
    }

    /// The kitty keyboard protocol's flags in force on the screen showing; the same five bits as
    /// `KEYS_*`, with the protocol's own values.
    #[must_use]
    pub fn kitty_flags(self) -> KittyFlags {
        // Bits 12..=16 are the flags, in the protocol's order.
        KittyFlags::from_bits_truncate(((self.bits() >> 12) & 0b1_1111) as u8)
    }
}

/// A terminal that has been told nothing: the starting modes, [`Modes::INITIAL`].
impl Default for Modes {
    fn default() -> Modes {
        Modes::INITIAL
    }
}

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

/// xterm's modifyOtherKeys (`CSI > 4 ; Pv m`): whether keys that have no function-key form are
/// sent as escape sequences when a modifier is held, so that a program can tell Ctrl+I from Tab
/// and Alt+x from a typed x. The sequences are `CSI 27 ; modifiers ; code ~` (xterm's
/// `formatOtherKeys` 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ModifyOtherKeys {
    /// 0: modified keys are sent as they always were.
    #[default]
    Off,
    /// 1: modified keys are sent as escape sequences, except where the legacy bytes are
    /// unambiguous: Shift alone on a character, and Ctrl on the keys that make a control code.
    Level1,
    /// 2: every key with a modifier is sent as an escape sequence.
    Level2,
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
    /// SGR with pixel positions (1016), 1-based as xterm writes them; falls back to cells for an
    /// event without a pixel.
    SgrPixels,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_starts_with_its_initial_modes() {
        assert_eq!(Modes::default(), Modes::INITIAL);
        assert_eq!(Modes::INITIAL.mouse_tracking(), MouseTracking::Off);
        assert_eq!(Modes::INITIAL.mouse_encoding(), MouseEncoding::X10);
        assert_eq!(Modes::INITIAL.kitty_flags(), KittyFlags::empty());
        assert!(!Modes::INITIAL.contains(Modes::BACKARROW_BS), "the backarrow key sends DEL");
        assert_eq!(Modes::INITIAL.modify_other_keys(), ModifyOtherKeys::Off);
    }

    #[test]
    fn the_widest_tracking_wins() {
        let tracking = |modes| Modes::from_bits_truncate(modes).mouse_tracking();
        assert_eq!(tracking(0), MouseTracking::Off);
        assert_eq!(tracking(Modes::MOUSE_X10.bits()), MouseTracking::X10);
        assert_eq!(tracking(Modes::MOUSE_CLICK.bits()), MouseTracking::Click);
        assert_eq!(
            tracking((Modes::MOUSE_X10 | Modes::MOUSE_CLICK | Modes::MOUSE_DRAG).bits()),
            MouseTracking::Drag
        );
        assert_eq!(tracking(Modes::all().bits()), MouseTracking::Motion);
        assert_eq!(Modes::MOUSE_SGR.mouse_tracking(), MouseTracking::Off, "an encoding is no tracking");
    }

    #[test]
    fn the_richest_encoding_wins() {
        let encoding = |modes: Modes| modes.mouse_encoding();
        assert_eq!(encoding(Modes::empty()), MouseEncoding::X10);
        assert_eq!(encoding(Modes::MOUSE_UTF8), MouseEncoding::Utf8);
        assert_eq!(encoding(Modes::MOUSE_UTF8 | Modes::MOUSE_URXVT), MouseEncoding::Urxvt);
        assert_eq!(encoding(Modes::MOUSE_URXVT | Modes::MOUSE_SGR), MouseEncoding::Sgr);
        assert_eq!(encoding(Modes::MOUSE_SGR | Modes::MOUSE_PIXELS), MouseEncoding::SgrPixels);
    }

    #[test]
    fn the_keys_bits_are_the_kitty_flags_in_order() {
        for (mode, flag) in [
            (Modes::KEYS_DISAMBIGUATE, KittyFlags::DISAMBIGUATE),
            (Modes::KEYS_EVENT_TYPES, KittyFlags::REPORT_EVENT_TYPES),
            (Modes::KEYS_ALTERNATE_KEYS, KittyFlags::REPORT_ALTERNATE_KEYS),
            (Modes::KEYS_ALL_AS_ESCAPES, KittyFlags::REPORT_ALL_KEYS),
            (Modes::KEYS_ASSOCIATED_TEXT, KittyFlags::REPORT_ASSOCIATED_TEXT),
        ] {
            assert_eq!(mode.kitty_flags(), flag);
        }
        assert_eq!(Modes::all().kitty_flags(), KittyFlags::all());
        assert_eq!((Modes::SHOW_CURSOR | Modes::INSERT).kitty_flags(), KittyFlags::empty());
    }
}
