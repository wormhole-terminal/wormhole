//! Character sets: G0..G3, which of them GL shows, and the single shifts.
//!
//! Wormhole's input is UTF-8, so only GL matters: a byte in 0x20..=0x7E goes through the set
//! invoked into GL, and anything decoded from a multi-byte sequence is printed as it is (xterm
//! does the same in UTF-8 mode). GR has no use: LS1R, LS2R and LS3R are accepted and do nothing,
//! because a decoded character is never a GR byte.
//!
//! The sets (VT510 SCS): ASCII, the UK set (VT100 User Guide, "the pound sign is displayed when the
//! U.K. set is selected"), DEC special graphics (VT100 User Guide, Table 3-9), the national
//! replacement sets of the VT200 family (`tables`), DEC Technical and DEC Supplemental (VT300) and
//! the 96-character ISO Latin-1 supplement. A 96-character set designated into G1..G3 shows its
//! right half (0xA0..=0xFF) at 0x20..=0x7F. The national sets exist only while DECNRCM is set
//! (VT510 DECNRCM: the multinational mode has no national sets), which the caller decides.

mod tables;

use tables::National;

use super::conformance::Level;

/// A character set a slot can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Charset {
    Ascii,
    /// `ESC ( A`: `#` is the pound sign.
    Uk,
    /// `ESC ( 0`: the line drawing and symbol set.
    Graphics,
    /// `ESC ( >`: DEC Technical, the mathematical set.
    Technical,
    /// `ESC ( % 5`, or `ESC ( <` while it is the user-preferred set: the Multinational set's right
    /// half in 0x21..=0x7E.
    Supplemental,
    /// `ESC - A`: ISO Latin-1's right half (a 96-character set).
    Latin1,
    /// `ESC ( R` and the others of the VT200 family.
    National(National),
}

/// The user-preferred supplemental set (DECAUPSS): what `<` designates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Preferred {
    /// DEC Supplemental Graphic (`DCS 0 ! u % 5 ST`), the default.
    #[default]
    Dec,
    /// ISO Latin-1 supplemental (`DCS 1 ! u A ST`).
    Latin1,
}

impl Preferred {
    fn set(self) -> Charset {
        match self {
            Preferred::Dec => Charset::Supplemental,
            Preferred::Latin1 => Charset::Latin1,
        }
    }

    /// DECAUPSS's data: `Pn` (the size of the set) and the final byte it is named by.
    pub(super) fn from_assignment(size: u16, name: &[u8]) -> Option<Preferred> {
        match (size, name) {
            (0, b"%5") => Some(Preferred::Dec),
            (1, b"A") => Some(Preferred::Latin1),
            _ => None,
        }
    }

    /// The report DECRQUPSS gets: `DCS Pn ! u Dscs ST`, without the string's frame.
    pub(super) fn report(self) -> &'static str {
        match self {
            Preferred::Dec => "0!u%5",
            Preferred::Latin1 => "1!uA",
        }
    }
}

/// A designation `ESC I Dscs`: the slot it fills and the set it puts there.
pub(super) fn designation(
    intermediates: &[u8],
    final_byte: u8,
    preferred: Preferred,
) -> Option<(usize, Charset)> {
    let (&first, name) = intermediates.split_first()?;
    // 94-character sets go to G0..G3 by `( ) * +`, 96-character sets to G1..G3 by `- . /`.
    let (slot, wide) = match first {
        b'(' => (0, false),
        b')' => (1, false),
        b'*' => (2, false),
        b'+' => (3, false),
        b'-' => (1, true),
        b'.' => (2, true),
        b'/' => (3, true),
        _ => return None,
    };
    let set = if wide {
        match (name, final_byte) {
            ([], b'A') => Charset::Latin1,
            ([], b'<') => preferred.set(),
            _ => return None,
        }
    } else {
        match (name, final_byte) {
            ([], b'B' | b'1' | b'@') => Charset::Ascii,
            ([], b'A') => Charset::Uk,
            ([], b'0' | b'2') => Charset::Graphics,
            ([], b'>') => Charset::Technical,
            ([], b'<') => preferred.set(),
            ([b'%'], b'5') => Charset::Supplemental,
            ([], b'C' | b'5') => Charset::National(National::Finnish),
            ([], b'H' | b'7') => Charset::National(National::Swedish),
            ([], b'K') => Charset::National(National::German),
            ([], b'Q' | b'9') => Charset::National(National::FrenchCanadian),
            ([], b'R' | b'f') => Charset::National(National::French),
            ([], b'Y') => Charset::National(National::Italian),
            ([], b'Z') => Charset::National(National::Spanish),
            ([], b'4') => Charset::National(National::Dutch),
            ([], b'=') => Charset::National(National::Swiss),
            ([], b'`' | b'E' | b'6') => Charset::National(National::NorwegianDanish),
            ([b'%'], b'6') => Charset::National(National::Portuguese),
            _ => return None,
        }
    };
    Some((slot, set))
}

impl Charset {
    /// Every set, for reading a designator back (DECRSPS).
    pub(super) const ALL: [Charset; 17] = [
        Charset::Ascii,
        Charset::Uk,
        Charset::Graphics,
        Charset::Technical,
        Charset::Supplemental,
        Charset::Latin1,
        Charset::National(National::Dutch),
        Charset::National(National::Finnish),
        Charset::National(National::French),
        Charset::National(National::FrenchCanadian),
        Charset::National(National::German),
        Charset::National(National::Italian),
        Charset::National(National::NorwegianDanish),
        Charset::National(National::Portuguese),
        Charset::National(National::Spanish),
        Charset::National(National::Swedish),
        Charset::National(National::Swiss),
    ];

    /// The conformance level that has this set: a terminal taken down to VT100 has three.
    pub(super) fn level(self) -> Level {
        match self {
            Charset::Ascii | Charset::Uk | Charset::Graphics => Level::Vt100,
            Charset::National(National::Portuguese)
            | Charset::Technical
            | Charset::Supplemental
            | Charset::Latin1 => Level::Vt300,
            Charset::National(_) => Level::Vt200,
        }
    }

    /// The characters of the SCS sequence that designates this set, after the intermediate that
    /// names the slot: what DECCIR reports (VT510 SCS).
    pub(super) fn designator(self) -> &'static str {
        match self {
            Charset::Ascii => "B",
            Charset::Uk => "A",
            Charset::Graphics => "0",
            Charset::Technical => ">",
            Charset::Supplemental => "%5",
            Charset::Latin1 => "A",
            Charset::National(set) => set.designator(),
        }
    }

    /// Whether the set has 96 characters (and is designated with `- . /`).
    pub(super) fn is_96(self) -> bool {
        self == Charset::Latin1
    }

    /// Whether this is a national replacement set, which DECNRCM must be set to designate.
    pub(super) fn is_national(self) -> bool {
        matches!(self, Charset::National(_))
    }

    /// The character a GL byte (0x20..=0x7E) shows in this set.
    #[inline]
    pub(super) fn map(self, byte: u8) -> char {
        match self {
            Charset::Ascii => char::from(byte),
            Charset::Uk if byte == b'#' => '\u{a3}',
            Charset::Uk => char::from(byte),
            Charset::Graphics => graphics(byte),
            Charset::Technical => match byte {
                0x21..=0x7e => tables::TECHNICAL[usize::from(byte - 0x21)],
                _ => char::from(byte),
            },
            Charset::Supplemental => match byte {
                0x21..=0x7e => tables::supplemental(byte + 0x80),
                _ => char::from(byte),
            },
            Charset::Latin1 => match byte {
                0x21..=0x7e => char::from(byte + 0x80),
                _ => char::from(byte),
            },
            Charset::National(set) => set.map(byte),
        }
    }
}

/// DEC special graphics, VT100 User Guide Table 3-9: 0x5F..=0x7E.
fn graphics(byte: u8) -> char {
    const TABLE: [char; 32] = [
        ' ', '\u{25c6}', '\u{2592}', '\u{2409}', '\u{240c}', '\u{240d}', '\u{240a}', '\u{b0}', '\u{b1}',
        '\u{2424}', '\u{240b}', '\u{2518}', '\u{2510}', '\u{250c}', '\u{2514}', '\u{253c}', '\u{23ba}',
        '\u{23bb}', '\u{2500}', '\u{23bc}', '\u{23bd}', '\u{251c}', '\u{2524}', '\u{2534}', '\u{252c}',
        '\u{2502}', '\u{2264}', '\u{2265}', '\u{3c0}', '\u{2260}', '\u{a3}', '\u{b7}',
    ];
    match byte {
        0x5f..=0x7e => TABLE[usize::from(byte - 0x5f)],
        _ => char::from(byte),
    }
}

/// The four slots and what is invoked: the state DECSC saves and RIS and DECSTR reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Charsets {
    slots: [Charset; 4],
    /// The slot GL shows (0 after SI, 1 after SO, 2 and 3 after LS2 and LS3).
    gl: u8,
    /// The slot GR shows (2 until LS1R, LS2R or LS3R say otherwise). Kept for DECCIR, which reports
    /// it; UTF-8 text has no GR byte to show it with.
    gr: u8,
    /// A slot (2 or 3) for the next character only (SS2, SS3).
    single: Option<u8>,
    /// Whether the next byte is printed as it is: GL shows ASCII and no single shift waits. The
    /// print loop tests this one flag.
    plain: bool,
}

impl Default for Charsets {
    fn default() -> Self {
        Charsets { slots: [Charset::Ascii; 4], gl: 0, gr: 2, single: None, plain: true }
    }
}

impl Charsets {
    /// Whether bytes print as they are.
    #[inline]
    pub(super) fn plain(&self) -> bool {
        self.plain
    }

    fn update(&mut self) {
        self.plain = self.single.is_none() && self.slots[usize::from(self.gl)] == Charset::Ascii;
    }

    /// `ESC ( x`, `ESC ) x`, `ESC * x`, `ESC + x`: designate `set` as G`slot`.
    pub(super) fn designate(&mut self, slot: usize, set: Charset) {
        self.slots[slot & 3] = set;
        self.update();
    }

    /// SI, SO, LS2, LS3: invoke G`slot` into GL.
    pub(super) fn invoke(&mut self, slot: u8) {
        self.gl = slot & 3;
        self.update();
    }

    /// LS1R, LS2R, LS3R: invoke G`slot` into GR.
    pub(super) fn invoke_right(&mut self, slot: u8) {
        self.gr = slot & 3;
    }

    /// The set in G`slot`.
    pub(super) fn slot(&self, slot: usize) -> Charset {
        self.slots[slot & 3]
    }

    /// The slots GL and GR show.
    pub(super) fn invoked(&self) -> (u8, u8) {
        (self.gl, self.gr)
    }

    /// Forget a pending single shift.
    pub(super) fn clear_single_shift(&mut self) {
        self.single = None;
        self.update();
    }

    /// The slot a pending single shift names.
    pub(super) fn single(&self) -> Option<u8> {
        self.single
    }

    /// SS2, SS3: G`slot` for the next character.
    pub(super) fn single_shift(&mut self, slot: u8) {
        self.single = Some(slot & 3);
        self.update();
    }

    /// The set for the byte about to be printed; a pending single shift is used up.
    #[inline]
    pub(super) fn take_active(&mut self) -> Charset {
        let slot = self.single.take().unwrap_or(self.gl);
        self.update();
        self.slots[usize::from(slot)]
    }
}

#[cfg(test)]
mod tests {
    use super::{Charset, Charsets};

    #[test]
    fn a_single_shift_is_used_up_by_one_character() {
        let mut sets = Charsets::default();
        sets.designate(2, Charset::Graphics);
        sets.single_shift(2);
        assert!(!sets.plain());
        assert_eq!(sets.take_active(), Charset::Graphics);
        assert!(sets.plain());
        assert_eq!(sets.take_active(), Charset::Ascii);
    }

    #[test]
    fn graphics_maps_the_line_drawing_range_only() {
        assert_eq!(Charset::Graphics.map(b'q'), '\u{2500}');
        assert_eq!(Charset::Graphics.map(b'A'), 'A');
        assert_eq!(Charset::Graphics.map(b'_'), ' ');
        assert_eq!(Charset::Uk.map(b'#'), '\u{a3}');
        assert_eq!(Charset::Uk.map(b'$'), '$');
    }
}
