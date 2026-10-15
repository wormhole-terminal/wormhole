//! The modes: one bitset, and the numbers that name them in SM/RM and DECSET/DECRST.
//!
//! The low 17 bits are [`TermMode`](crate::types::TermMode)'s, bit for bit (a test checks it), so
//! the snapshot's mode set is a mask. Everything else is state a program can switch and ask for
//! (DECRQM) that the app reads through an accessor or not at all. What a mode *does* is in the
//! code that reads it; this file knows the numbers, the defaults and what DECRQM answers.

use bitflags::bitflags;

bitflags! {
    /// Every mode the terminal keeps.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Modes: u64 {
        /// DECTCEM (25).
        const SHOW_CURSOR      = 1;
        /// DECAWM (7).
        const AUTOWRAP         = 1 << 1;
        /// Whether the alternate screen shows (47, 1047, 1049); not a mode a program sets
        /// directly, kept in step by the screen switch.
        const ALT_SCREEN       = 1 << 2;
        /// 1007.
        const ALTERNATE_SCROLL = 1 << 3;
        /// 2004.
        const BRACKETED_PASTE  = 1 << 4;
        /// 1004.
        const FOCUS_REPORT     = 1 << 5;
        /// DECCKM (1).
        const APP_CURSOR       = 1 << 6;
        /// 1000, 1002, 1003.
        const MOUSE_CLICK      = 1 << 7;
        /// 1002.
        const MOUSE_DRAG       = 1 << 8;
        /// 1003.
        const MOUSE_MOTION     = 1 << 9;
        /// 1006.
        const MOUSE_SGR        = 1 << 10;
        /// 1005.
        const MOUSE_UTF8       = 1 << 11;
        /// The kitty keyboard flags of the screen showing (bits 12..=16, see `KeyboardFlags`).
        /// Kitty keyboard flag 1: disambiguate escape codes.
        const KEYS_DISAMBIGUATE   = 1 << 12;
        /// Flag 2: report event types (press, repeat, release).
        const KEYS_EVENT_TYPES    = 1 << 13;
        /// Flag 4: report alternate keys.
        const KEYS_ALTERNATE_KEYS = 1 << 14;
        /// Flag 8: report all keys as escape codes.
        const KEYS_ALL_AS_ESCAPES = 1 << 15;
        /// Flag 16: report associated text.
        const KEYS_ASSOCIATED_TEXT = 1 << 16;

        /// IRM (ANSI 4).
        const INSERT           = 1 << 17;
        /// LNM (ANSI 20): LF also returns the carriage.
        const NEW_LINE         = 1 << 18;
        /// KAM (ANSI 2).
        const KEYBOARD_LOCK    = 1 << 19;
        /// SRM (ANSI 12): set means no local echo, the usual state.
        const SEND_RECEIVE     = 1 << 20;
        /// DECCOLM (3): 132 columns.
        const COLUMNS_132      = 1 << 21;
        /// DECSCLM (4), recorded only.
        const SMOOTH_SCROLL    = 1 << 22;
        /// DECSCNM (5): the screen is drawn in reverse video.
        const REVERSE_VIDEO    = 1 << 23;
        /// DECOM (6): positions count from the scrolling region.
        const ORIGIN           = 1 << 24;
        /// DECARM (8), recorded only.
        const AUTO_REPEAT      = 1 << 25;
        /// 9: X10 mouse reporting, recorded only.
        const MOUSE_X10        = 1 << 26;
        /// 40: DECCOLM may change the width.
        const ALLOW_COLUMNS    = 1 << 28;
        /// 45: BS at the left edge goes to the end of the line above.
        const REVERSE_WRAP     = 1 << 29;
        /// DECNKM (66), also ESC = and ESC >: the keypad sends application sequences.
        const APP_KEYPAD       = 1 << 30;
        /// DECBKM (67): the backarrow key sends BS (set) or DEL.
        const BACKARROW_BS     = 1 << 31;
        /// DECLRMM (69): left and right margins.
        const LEFT_RIGHT_MARGINS = 1 << 32;
        /// DECSDM (80): sixel display mode. Set, a Sixel is drawn at the top left of the screen and the
        /// cursor does not move; reset (the default), it starts at the cursor and the screen scrolls.
        const SIXEL_DISPLAY    = 1 << 33;
        /// DECNCSM (95): DECCOLM does not clear the screen.
        const NO_CLEAR_ON_COLUMNS = 1 << 34;
        /// 1015.
        const MOUSE_URXVT      = 1 << 35;
        /// 1016.
        const MOUSE_PIXELS     = 1 << 36;
        /// 2026: synchronized output.
        const SYNC_OUTPUT      = 1 << 37;
        /// 2027: grapheme clusters.
        const GRAPHEME_CLUSTERS = 1 << 38;
        /// 1070: private color registers for each Sixel, recorded only (every picture starts with the
        /// VT340's colors).
        const SIXEL_PRIVATE_PALETTE = 1 << 39;
        /// 8452: a Sixel leaves the cursor right of the picture's last row, not under it.
        const SIXEL_CURSOR_RIGHT = 1 << 40;
        /// 2031: report light/dark changes, recorded only.
        const THEME_REPORT     = 1 << 41;
        /// 1010, recorded only.
        const SCROLL_ON_OUTPUT = 1 << 42;
        /// 1011, recorded only.
        const SCROLL_ON_KEY    = 1 << 43;
        /// 1036, recorded only.
        const META_SENDS_ESC   = 1 << 44;
        /// 1039, recorded only.
        const ALT_SENDS_ESC    = 1 << 45;
        /// 1042, recorded only.
        const BELL_URGENCY     = 1 << 46;
        /// 1045: BS and CUB at the left edge go to the line above even when it did not wrap, and
        /// from the top margin to the bottom one.
        const REVERSE_WRAP_EXTENDED = 1 << 47;
        /// 41: a tab with the wrap pending wraps first (xterm's fix for curses programs that draw
        /// to the end of a row and then send a tab).
        const MORE_FIX = 1 << 48;
    }
}

impl Modes {
    /// The modes a terminal starts with (and RIS restores): the cursor shown, autowrap on,
    /// send/receive set, auto-repeat on, the wheel sending arrows on the alternate screen and
    /// grapheme clustering (2027) on: a program that sizes text by clusters (everything written
    /// since emoji sequences, Indic scripts and Hangul needed it) agrees with the terminal without
    /// asking, and one that counts code points can turn it off.
    pub const INITIAL: Modes = Modes::SHOW_CURSOR
        .union(Modes::AUTOWRAP)
        .union(Modes::GRAPHEME_CLUSTERS)
        .union(Modes::SEND_RECEIVE)
        .union(Modes::AUTO_REPEAT)
        .union(Modes::ALTERNATE_SCROLL)
        .union(Modes::BACKARROW_BS);

    /// What DECSTR resets: the modes of the keyboard and of how text is placed. Mouse, paste and
    /// focus modes belong to the program that set them and stay.
    pub(super) const SOFT_RESET: Modes = Modes::INSERT
        .union(Modes::NEW_LINE)
        .union(Modes::KEYBOARD_LOCK)
        .union(Modes::APP_CURSOR)
        .union(Modes::APP_KEYPAD)
        .union(Modes::ORIGIN)
        .union(Modes::LEFT_RIGHT_MARGINS)
        .union(Modes::GRAPHEME_CLUSTERS)
        .union(Modes::REVERSE_WRAP)
        .union(Modes::REVERSE_WRAP_EXTENDED);

    /// The kitty keyboard flags, bits 12..=16.
    pub(super) const KEYS: Modes = Modes::KEYS_DISAMBIGUATE
        .union(Modes::KEYS_EVENT_TYPES)
        .union(Modes::KEYS_ALTERNATE_KEYS)
        .union(Modes::KEYS_ALL_AS_ESCAPES)
        .union(Modes::KEYS_ASSOCIATED_TEXT);
}

/// Whether a mode is being set (SM, DECSET) or reset (RM, DECRST).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Switch {
    Set,
    Reset,
}

impl Switch {
    pub(super) fn is_set(self) -> bool {
        self == Switch::Set
    }
}

/// Which numbering a mode number is in: ANSI (SM, RM) or DEC private (DECSET, DECRST).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModeSet {
    Ansi,
    Private,
}

/// What DECRQM answers (`Pm` of the reply).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    NotRecognized = 0,
    Set = 1,
    Reset = 2,
    PermanentlySet = 3,
    PermanentlyReset = 4,
}

impl Status {
    pub(super) fn of(set: bool) -> Status {
        if set { Status::Set } else { Status::Reset }
    }
}

/// The ANSI modes (SM, RM): the flag each number stands for.
pub(super) fn ansi_flag(n: u16) -> Option<Modes> {
    Some(match n {
        2 => Modes::KEYBOARD_LOCK,
        4 => Modes::INSERT,
        12 => Modes::SEND_RECEIVE,
        20 => Modes::NEW_LINE,
        _ => return None,
    })
}

/// The DEC private modes whose whole effect is a flag the rest of the code reads. The ones with
/// an action of their own (DECCOLM, DECOM, the alternate screen, DECLRMM, …) are handled in
/// `Terminal::set_private_mode` first and only come here for the flag.
pub(super) fn private_flag(n: u16) -> Option<Modes> {
    Some(match n {
        1 => Modes::APP_CURSOR,
        3 => Modes::COLUMNS_132,
        4 => Modes::SMOOTH_SCROLL,
        5 => Modes::REVERSE_VIDEO,
        6 => Modes::ORIGIN,
        7 => Modes::AUTOWRAP,
        8 => Modes::AUTO_REPEAT,
        9 => Modes::MOUSE_X10,
        25 => Modes::SHOW_CURSOR,
        40 => Modes::ALLOW_COLUMNS,
        41 => Modes::MORE_FIX,
        45 => Modes::REVERSE_WRAP,
        66 => Modes::APP_KEYPAD,
        67 => Modes::BACKARROW_BS,
        69 => Modes::LEFT_RIGHT_MARGINS,
        80 => Modes::SIXEL_DISPLAY,
        95 => Modes::NO_CLEAR_ON_COLUMNS,
        1000 => Modes::MOUSE_CLICK,
        1002 => Modes::MOUSE_DRAG,
        1003 => Modes::MOUSE_MOTION,
        1004 => Modes::FOCUS_REPORT,
        1005 => Modes::MOUSE_UTF8,
        1006 => Modes::MOUSE_SGR,
        1007 => Modes::ALTERNATE_SCROLL,
        1010 => Modes::SCROLL_ON_OUTPUT,
        1011 => Modes::SCROLL_ON_KEY,
        1015 => Modes::MOUSE_URXVT,
        1016 => Modes::MOUSE_PIXELS,
        1036 => Modes::META_SENDS_ESC,
        1039 => Modes::ALT_SENDS_ESC,
        1042 => Modes::BELL_URGENCY,
        1045 => Modes::REVERSE_WRAP_EXTENDED,
        1070 => Modes::SIXEL_PRIVATE_PALETTE,
        2004 => Modes::BRACKETED_PASTE,
        2026 => Modes::SYNC_OUTPUT,
        2027 => Modes::GRAPHEME_CLUSTERS,
        2031 => Modes::THEME_REPORT,
        8452 => Modes::SIXEL_CURSOR_RIGHT,
        _ => return None,
    })
}

/// The DEC private modes of the VT510 that belong to hardware or set-up a terminal emulator has
/// none of (the printer, a Hebrew or Greek keyboard, the modem, the CRT saver, …). A program may
/// set and reset them and ask for them with DECRQM; they are kept, nothing else. The bit of a
/// mode in `Terminal::recorded` is its place in this list.
const RECORDED: [u16; 22] = [
    18,  // DECPFF: print form feed
    19,  // DECPEX: print extent
    34,  // DECRLM: right-to-left
    35,  // DECHEBM: Hebrew keyboard mapping
    36,  // DECHEM: Hebrew encoding
    42,  // DECNRCM: national replacement character sets
    57,  // DECNAKB: Greek/North American keyboard mapping
    61,  // DECVCCM: vertical cursor coupling
    64,  // DECPCCM: page cursor coupling
    68,  // DECKBUM: keyboard usage
    73,  // DECXRLM: transmit rate limiting
    81,  // DECKPM: key position
    96,  // DECRLCM: right-to-left copy
    97,  // DECCRTSM: CRT save
    98,  // DECARSM: auto resize
    99,  // DECMCM: modem control
    100, // DECAAM: auto answerback
    101, // DECCANSM: conceal answerback message
    102, // DECNULM: null
    103, // DECHDPXM: half-duplex
    104, // DECESKM: secondary keyboard language
    106, // DECOSCNM: overscan
];

/// The bit that remembers DEC private mode `n` if it is one of the [`RECORDED`] ones.
pub(super) fn recorded_bit(n: u16) -> Option<u32> {
    RECORDED.iter().position(|&mode| mode == n).map(|at| 1 << at)
}

/// ANSI modes that are permanently reset: the ones of ECMA-48 that need a terminal with a
/// guarded-area transfer, a printer or a second device (VT510, "Mode Settings": GATM, SRTM,
/// VEM, HEM, PUM, FEAM, FETM, MATM, TTM, SATM, TSM, EBM).
pub(super) fn ansi_fixed(n: u16) -> Option<Status> {
    matches!(n, 1 | 5 | 7 | 10 | 11 | 13..=19).then_some(Status::PermanentlyReset)
}

/// Modes DECRQM knows about that are not flags: the alternate screen and the cursor save (always
/// reported as reset, there is nothing to ask), and the ones that are permanently one way.
pub(super) fn fixed_status(n: u16) -> Option<Status> {
    Some(match n {
        // DECANM: always ANSI (VT52 mode is not supported).
        2 => Status::PermanentlySet,
        // DECHCCM (horizontal cursor coupling, a pan of a page wider than the window), hilite
        // mouse tracking, and xterm's 41 and 1034..1038: not supported.
        60 | 1001 | 1034 | 1035 | 1037 | 1038 => Status::PermanentlyReset,
        // 1048 is an action; xterm answers reset.
        1048 => Status::Reset,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{Modes, ansi_flag, private_flag};
    use crate::types::TermMode;

    #[test]
    fn the_low_bits_are_the_term_modes_bit_for_bit() {
        for (ours, theirs) in [
            (Modes::SHOW_CURSOR, TermMode::SHOW_CURSOR),
            (Modes::AUTOWRAP, TermMode::LINE_WRAP),
            (Modes::ALT_SCREEN, TermMode::ALT_SCREEN),
            (Modes::ALTERNATE_SCROLL, TermMode::ALTERNATE_SCROLL),
            (Modes::BRACKETED_PASTE, TermMode::BRACKETED_PASTE),
            (Modes::FOCUS_REPORT, TermMode::FOCUS_IN_OUT),
            (Modes::APP_CURSOR, TermMode::APP_CURSOR),
            (Modes::MOUSE_CLICK, TermMode::MOUSE_REPORT_CLICK),
            (Modes::MOUSE_DRAG, TermMode::MOUSE_DRAG),
            (Modes::MOUSE_MOTION, TermMode::MOUSE_MOTION),
            (Modes::MOUSE_SGR, TermMode::SGR_MOUSE),
            (Modes::MOUSE_UTF8, TermMode::UTF8_MOUSE),
            (Modes::KEYS_DISAMBIGUATE, TermMode::DISAMBIGUATE_ESC_CODES),
            (Modes::KEYS_EVENT_TYPES, TermMode::REPORT_EVENT_TYPES),
            (Modes::KEYS_ALTERNATE_KEYS, TermMode::REPORT_ALTERNATE_KEYS),
            (Modes::KEYS_ALL_AS_ESCAPES, TermMode::REPORT_ALL_KEYS_AS_ESC),
            (Modes::KEYS_ASSOCIATED_TEXT, TermMode::REPORT_ASSOCIATED_TEXT),
        ] {
            assert_eq!(ours.bits(), u64::from(theirs.bits()));
        }
    }

    #[test]
    fn every_number_names_one_flag() {
        let mut seen = Vec::new();
        for n in 0..=9000u16 {
            if let Some(flag) = private_flag(n) {
                assert!(!seen.contains(&flag), "{n} repeats {flag:?}");
                seen.push(flag);
            }
        }
        assert_eq!(ansi_flag(4), Some(Modes::INSERT));
        assert_eq!(ansi_flag(20), Some(Modes::NEW_LINE));
        assert_eq!(ansi_flag(99), None);
    }
}
