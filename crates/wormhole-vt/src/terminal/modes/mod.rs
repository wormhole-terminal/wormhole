//! The modes: the numbers that name them in SM/RM and DECSET/DECRST, the sets the soft reset and
//! the keyboard flags are, and what DECRQM answers.
//!
//! The set itself, [`Modes`], is a value type a host reads (`types/mode.rs`, which documents each
//! flag). What a mode *does* is in the code that reads it; this file knows the numbers.

pub(super) mod apply;
pub(super) mod conformance;
pub(super) mod kept;
pub(super) mod keyboard;
pub(super) mod modkeys;

pub use crate::types::Modes;

impl Modes {
    /// What DECSTR resets: the modes of the keyboard and of how text is placed. Mouse, paste and
    /// focus modes belong to the program that set them and stay.
    pub(in crate::terminal) const SOFT_RESET: Modes = Modes::INSERT
        .union(Modes::NEW_LINE)
        .union(Modes::KEYBOARD_LOCK)
        .union(Modes::APP_CURSOR)
        .union(Modes::APP_KEYPAD)
        .union(Modes::ORIGIN)
        .union(Modes::LEFT_RIGHT_MARGINS)
        .union(Modes::GRAPHEME_CLUSTERS)
        .union(Modes::REVERSE_WRAP)
        .union(Modes::REVERSE_WRAP_EXTENDED)
        // xterm: DECSTR resets the key-modifier modes to their resource values (initial: 0).
        .union(Modes::MODIFY_OTHER_KEYS)
        .union(Modes::MODIFY_OTHER_KEYS_ALL);

    /// The kitty keyboard flags, bits 12..=16.
    pub(in crate::terminal) const KEYS: Modes = Modes::KEYS_DISAMBIGUATE
        .union(Modes::KEYS_EVENT_TYPES)
        .union(Modes::KEYS_ALTERNATE_KEYS)
        .union(Modes::KEYS_ALL_AS_ESCAPES)
        .union(Modes::KEYS_ASSOCIATED_TEXT);
}

/// Whether a mode is being set (SM, DECSET) or reset (RM, DECRST).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Switch {
    Set,
    Reset,
}

impl Switch {
    pub(in crate::terminal) fn is_set(self) -> bool {
        self == Switch::Set
    }
}

/// Which numbering a mode number is in: ANSI (SM, RM) or DEC private (DECSET, DECRST).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum ModeSet {
    Ansi,
    Private,
}

/// What DECRQM answers (`Pm` of the reply).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Status {
    NotRecognized = 0,
    Set = 1,
    Reset = 2,
    PermanentlyReset = 4,
}

impl Status {
    pub(in crate::terminal) fn of(set: bool) -> Status {
        if set { Status::Set } else { Status::Reset }
    }
}

/// The ANSI modes (SM, RM): the flag each number stands for.
pub(in crate::terminal) fn ansi_flag(n: u16) -> Option<Modes> {
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
pub(in crate::terminal) fn private_flag(n: u16) -> Option<Modes> {
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
        2048 => Modes::INBAND_RESIZE,
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

/// The numbers of the [`RECORDED`] modes whose bits are set in `bits`.
pub(in crate::terminal) fn recorded_modes(bits: u32) -> impl Iterator<Item = u16> {
    RECORDED.iter().enumerate().filter(move |(at, _)| bits & (1 << at) != 0).map(|(_, &n)| n)
}

/// The bit that remembers DEC private mode `n` if it is one of the [`RECORDED`] ones.
pub(in crate::terminal) fn recorded_bit(n: u16) -> Option<u32> {
    RECORDED.iter().position(|&mode| mode == n).map(|at| 1 << at)
}

/// ANSI modes that are permanently reset: the ones of ECMA-48 that need a terminal with a
/// guarded-area transfer, a printer or a second device (VT510, "Mode Settings": GATM, SRTM,
/// VEM, HEM, PUM, FEAM, FETM, MATM, TTM, SATM, TSM, EBM).
pub(in crate::terminal) fn ansi_fixed(n: u16) -> Option<Status> {
    matches!(n, 1 | 5 | 7 | 10 | 11 | 13..=19).then_some(Status::PermanentlyReset)
}

/// Modes DECRQM knows about that are not flags: the alternate screen and the cursor save (always
/// reported as reset, there is nothing to ask), and the ones that are permanently one way.
pub(in crate::terminal) fn fixed_status(n: u16) -> Option<Status> {
    Some(match n {
        // DECANM: asked in ANSI mode, which is the only one that can ask.
        2 => Status::Set,
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
