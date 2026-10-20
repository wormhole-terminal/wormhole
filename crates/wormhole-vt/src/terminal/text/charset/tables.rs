//! The tables behind the character sets: DEC Technical, DEC Supplemental and the national
//! replacement sets.
//!
//! Sources: DEC Technical is the VT510 manual's technical character set (94 positions
//! 0x21..=0x7E) with the Unicode characters Kermit's table gives it (U+23B7 radical symbol bottom
//! to U+2193); the unused positions show a blank. DEC Supplemental is the upper half of the Multinational Character
//! Set: ISO 8859-1 except for five letters and the unused positions. The national sets are the VT510
//! manual's NRCS tables: twelve positions of ASCII are replaced.

/// DEC Technical, positions 0x21..=0x7E.
pub(in crate::terminal) const TECHNICAL: [char; 94] = [
    '\u{23b7}', '\u{250c}', '\u{2500}', '\u{2320}', '\u{2321}', '\u{2502}', '\u{23a1}', '\u{23a3}',
    '\u{23a4}', '\u{23a6}', '\u{239b}', '\u{239d}', '\u{239e}', '\u{23a0}', '\u{23a8}', '\u{23ac}',
    '\u{23b2}', '\u{23b3}', '\u{2572}', '\u{2571}', '\u{231d}', '\u{231f}', '\u{27e9}', ' ', ' ', ' ', ' ',
    '\u{2264}', '\u{2260}', '\u{2265}', '\u{222b}', '\u{2234}', '\u{221d}', '\u{221e}', '\u{00f7}',
    '\u{0394}', '\u{2207}', '\u{03a6}', '\u{0393}', '\u{223c}', '\u{2243}', '\u{0398}', '\u{00d7}',
    '\u{039b}', '\u{21d4}', '\u{21d2}', '\u{2261}', '\u{03a0}', '\u{03a8}', ' ', '\u{03a3}', ' ', ' ',
    '\u{221a}', '\u{03a9}', '\u{039e}', '\u{03a5}', '\u{2282}', '\u{2283}', '\u{2229}', '\u{222a}',
    '\u{2227}', '\u{2228}', '\u{00ac}', '\u{03b1}', '\u{03b2}', '\u{03c7}', '\u{03b4}', '\u{03b5}',
    '\u{03c6}', '\u{03b3}', '\u{03b7}', '\u{03b9}', '\u{03b8}', '\u{03ba}', '\u{03bb}', ' ', '\u{03bd}',
    '\u{2202}', '\u{03c0}', '\u{03c8}', '\u{03c1}', '\u{03c3}', '\u{03c4}', ' ', '\u{0192}', '\u{03c9}',
    '\u{03be}', '\u{03c5}', '\u{03b6}', '\u{2190}', '\u{2191}', '\u{2192}', '\u{2193}',
];

/// The positions an NRCS replaces (ASCII's `# @ [ \ ] ^ _ ` { | } ~`), in the order of
/// [`National::row`].
const REPLACED: [u8; 12] = *b"#@[\\]^_`{|}~";

/// A national replacement character set (VT510 SCS): ASCII with up to twelve positions replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum National {
    Dutch,
    Finnish,
    French,
    FrenchCanadian,
    German,
    Italian,
    NorwegianDanish,
    Portuguese,
    Spanish,
    Swedish,
    Swiss,
}

impl National {
    /// What stands at the twelve positions of [`REPLACED`]; NUL keeps ASCII's.
    const fn row(self) -> [char; 12] {
        const KEEP: char = '\0';
        match self {
            //          #    @    [    \    ]    ^    _    `    {    |    }    ~
            National::Dutch => [
                '\u{a3}', '\u{be}', '\u{133}', '\u{bd}', '|', KEEP, KEEP, KEEP, '\u{a8}', '\u{192}',
                '\u{bc}', '\u{b4}',
            ],
            National::Finnish => [
                KEEP, KEEP, '\u{c4}', '\u{d6}', '\u{c5}', '\u{dc}', KEEP, '\u{e9}', '\u{e4}', '\u{f6}',
                '\u{e5}', '\u{fc}',
            ],
            National::French => [
                '\u{a3}', '\u{e0}', '\u{b0}', '\u{e7}', '\u{a7}', KEEP, KEEP, KEEP, '\u{e9}', '\u{f9}',
                '\u{e8}', '\u{a8}',
            ],
            National::FrenchCanadian => [
                KEEP, '\u{e0}', '\u{e2}', '\u{e7}', '\u{ea}', '\u{ee}', KEEP, '\u{f4}', '\u{e9}', '\u{f9}',
                '\u{e8}', '\u{fb}',
            ],
            National::German => [
                KEEP, '\u{a7}', '\u{c4}', '\u{d6}', '\u{dc}', KEEP, KEEP, KEEP, '\u{e4}', '\u{f6}', '\u{fc}',
                '\u{df}',
            ],
            National::Italian => [
                '\u{a3}', '\u{a7}', '\u{b0}', '\u{e7}', '\u{e9}', KEEP, KEEP, '\u{f9}', '\u{e0}', '\u{f2}',
                '\u{e8}', '\u{ec}',
            ],
            National::NorwegianDanish => [
                KEEP, '\u{c4}', '\u{c6}', '\u{d8}', '\u{c5}', '\u{dc}', KEEP, '\u{e4}', '\u{e6}', '\u{f8}',
                '\u{e5}', '\u{fc}',
            ],
            National::Portuguese => [
                KEEP, KEEP, '\u{c3}', '\u{c7}', '\u{d5}', KEEP, KEEP, KEEP, '\u{e3}', '\u{e7}', '\u{f5}',
                KEEP,
            ],
            National::Spanish => [
                '\u{a3}', '\u{a7}', '\u{a1}', '\u{d1}', '\u{bf}', KEEP, KEEP, KEEP, '\u{b0}', '\u{f1}',
                '\u{e7}', KEEP,
            ],
            National::Swedish => [
                KEEP, '\u{c9}', '\u{c4}', '\u{d6}', '\u{c5}', '\u{dc}', KEEP, '\u{e9}', '\u{e4}', '\u{f6}',
                '\u{e5}', '\u{fc}',
            ],
            National::Swiss => [
                '\u{f9}', '\u{e0}', '\u{e9}', '\u{e7}', '\u{ea}', '\u{ee}', '\u{e8}', '\u{f4}', '\u{e4}',
                '\u{f6}', '\u{fc}', '\u{fb}',
            ],
        }
    }

    /// The final characters of the set's SCS sequence.
    pub(in crate::terminal) fn designator(self) -> &'static str {
        match self {
            National::Dutch => "4",
            National::Finnish => "5",
            National::French => "R",
            National::FrenchCanadian => "9",
            National::German => "K",
            National::Italian => "Y",
            National::NorwegianDanish => "E",
            National::Portuguese => "%6",
            National::Spanish => "Z",
            National::Swedish => "7",
            National::Swiss => "=",
        }
    }

    /// The character this set shows for the ASCII byte `byte`.
    pub(in crate::terminal) fn map(self, byte: u8) -> char {
        match REPLACED.iter().position(|&at| at == byte) {
            Some(i) => match self.row()[i] {
                '\0' => char::from(byte),
                replaced => replaced,
            },
            None => char::from(byte),
        }
    }
}

/// The positions of DEC Supplemental (the Multinational set's right half, 0xA0..=0xFF) that hold
/// no character.
const SUPPLEMENTAL_UNUSED: [u8; 15] =
    [0xa0, 0xa4, 0xa6, 0xac, 0xad, 0xae, 0xaf, 0xb4, 0xb8, 0xbe, 0xd0, 0xde, 0xf0, 0xfe, 0xff];

/// The character at `code` (0xA0..=0xFF) of the DEC Multinational Character Set's right half: the
/// ISO 8859-1 letter, except `0xA8` (the currency sign), `0xD7` (OE), `0xDD` (Y diaeresis), `0xF7`
/// (oe) and `0xFD` (y diaeresis). Unused positions show a blank.
pub(in crate::terminal) fn supplemental(code: u8) -> char {
    match code {
        0xa8 => '\u{a4}',
        0xd7 => '\u{152}',
        0xdd => '\u{178}',
        0xf7 => '\u{153}',
        0xfd => '\u{ff}',
        code if SUPPLEMENTAL_UNUSED.contains(&code) => ' ',
        code => char::from(code),
    }
}
