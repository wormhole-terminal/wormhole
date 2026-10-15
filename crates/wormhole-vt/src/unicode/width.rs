//! Cell widths.

use super::tables::{self, VS_BASE, WIDTH_MASK};

/// How many terminal cells `c` takes: 0, 1 or 2.
///
/// - 0: combining marks (Mn, Me), format characters (Cf: zero width space, ZWJ, ZWNJ, bidi marks,
///   tags) except U+00AD SOFT HYPHEN and the Prepended_Concatenation_Marks (the Arabic number sign
///   U+0600 and its kin, which are drawn), the viramas (Indic_Syllabic_Category=Virama, spacing
///   marks among them), variation selectors, the Hangul medial vowels and final
///   consonants (U+1160..U+11FF, U+D7B0..U+D7FF), and control characters. A terminal draws these on
///   the cell of the character before them, or not at all.
/// - 2: East Asian Wide and Fullwidth characters and everything with Emoji_Presentation (flags'
///   regional indicators and emoji such as U+1F600 included).
/// - 1: the rest; East Asian Ambiguous characters are narrow, as in a Western terminal.
///
/// The width of a character on its own: a cluster is as wide as [`cluster_width`] says.
#[inline]
pub fn width(c: char) -> u8 {
    let cp = c as u32;
    // Printable ASCII is by far most of what is printed: no table.
    if cp.wrapping_sub(0x20) < 0x5F {
        return 1;
    }
    tables::props(cp) & WIDTH_MASK
}

/// Whether `c` has an emoji variation sequence (emoji-variation-sequences.txt): the characters
/// that U+FE0F (VS16) draws as a wide emoji and U+FE0E (VS15) as narrow text. They are the
/// text-default emoji (U+2764 HEAVY BLACK HEART, #, 0..9, ...) and some default-emoji (U+231A WATCH).
#[inline]
pub fn has_emoji_variation(c: char) -> bool {
    tables::props(c as u32) & VS_BASE != 0
}

/// The width of a cluster after `c` joined it, given the width it had (`width`) and the character
/// before `c` in it (`prev`). Start a cluster at `width(first)` and fold the rest in with this.
///
/// - VS16 (U+FE0F) after a character with an emoji variation sequence makes the cluster 2, VS15
///   (U+FE0E) after one makes it 1 (UTS #51 ED-9a and ED-8a, the emoji and the text presentation
///   sequence; UTS #51 does not size them, terminals and ucs-detect take the text one as narrow).
/// - Anything else adds its own [`width`], up to 2 cells (a cluster is never wider than a wide
///   character): a combining mark adds nothing, a spacing mark (Mc, or the Thai and Lao SARA AM)
///   a cell, a consonant joined by a virama (GB9c) a cell, and a virama itself nothing even where
///   Unicode calls it a spacing mark (Javanese pangkon, Grantha virama). So a regional-indicator pair and a ZWJ family
///   are 2, a base with combining marks keeps the base's width, and an Indic conjunct with its vowel
///   signs is 2 however long it is.
#[inline]
pub fn extend_width(width: u8, prev: char, c: char) -> u8 {
    match c {
        '\u{FE0F}' if has_emoji_variation(prev) => 2,
        '\u{FE0E}' if has_emoji_variation(prev) => 1,
        _ => (width + self::width(c)).min(2),
    }
}

/// The width in cells of a whole cluster, from its characters in order (0 for none).
pub fn cluster_width(chars: impl IntoIterator<Item = char>) -> u8 {
    let mut chars = chars.into_iter();
    let Some(first) = chars.next() else { return 0 };
    let (mut cells, mut prev) = (width(first), first);
    for c in chars {
        cells = extend_width(cells, prev, c);
        prev = c;
    }
    cells
}
