//! A cell's look: colors and SGR attributes. The page interns them in a table (`style_table.rs`).

use crate::types::{Color, Flags, NamedColor};

/// How a cell looks beyond its character: colors and SGR attributes.
///
/// `attrs` holds only attribute bits (bold, dim, italic, the underline styles, blink, inverse,
/// hidden, strikeout, overline); where a cell sits in a row (wide, spacer, wrap) is the cell's own
/// flags. Kept in the form a renderer draws, so the snapshot copy moves it without converting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    /// SGR 58, when it is not the text's color.
    pub underline_color: Option<Color>,
    pub attrs: Flags,
}

/// The bits of [`Flags`] that are layout, not look.
pub(super) const LAYOUT_FLAGS: Flags = Flags::WIDE_CHAR
    .union(Flags::WIDE_CHAR_SPACER)
    .union(Flags::LEADING_WIDE_CHAR_SPACER)
    .union(Flags::WRAPLINE);

impl Style {
    /// The default colors and no attributes; id 0 in every page.
    pub const DEFAULT: Style = Style {
        fg: Color::Named(NamedColor::Foreground),
        bg: Color::Named(NamedColor::Background),
        underline_color: None,
        attrs: Flags::empty(),
    };

    /// What an erased cell looks like (background-color erase): the pen's background and nothing
    /// else.
    #[inline]
    pub fn erased(bg: Color) -> Style {
        Style { bg, ..Style::DEFAULT }
    }

    #[inline]
    pub fn is_default(&self) -> bool {
        // Field by field, the cheapest first: the derived comparison with the constant is a call.
        self.attrs.is_empty()
            && self.underline_color.is_none()
            && self.fg == Style::DEFAULT.fg
            && self.bg == Style::DEFAULT.bg
    }

    /// `self == other`, inlined where the table compares: attributes first, which differ most.
    #[inline(always)]
    pub(super) fn same(&self, other: &Style) -> bool {
        self.attrs == other.attrs
            && self.fg == other.fg
            && self.bg == other.bg
            && self.underline_color == other.underline_color
    }

    /// A hash of all four fields, for the table's index, keyed with the table's `seed`.
    ///
    /// The key is what keeps a program from choosing colors that all land in one slot of the
    /// index (linear probing would then walk the whole run for every lookup): it does not know the
    /// seed, which is drawn at random when the table is made. Each field goes through a folded
    /// multiply (the 128-bit product's two halves xored), whose high half lands in the low bits a
    /// mask keeps; a plain multiply, as this was, passes the low bits of its input on unchanged
    /// and so lets a key that is xored in cancel out of the collisions.
    pub(super) fn table_hash(&self, seed: u64) -> u64 {
        fn bits(color: Color) -> u64 {
            match color {
                Color::Named(named) => 1 << 32 | named as u16 as u64,
                Color::Indexed(ix) => 2 << 32 | ix as u64,
                Color::Rgb(rgb) => 3 << 32 | (rgb.r as u64) << 16 | (rgb.g as u64) << 8 | rgb.b as u64,
            }
        }
        #[inline(always)]
        fn fold(h: u64, word: u64, k: u64) -> u64 {
            let product = u128::from(h ^ word).wrapping_mul(u128::from(k));
            (product as u64) ^ (product >> 64) as u64
        }
        const A: u64 = 0x9E37_79B9_7F4A_7C15;
        const B: u64 = 0xC2B2_AE3D_27D4_EB4F;
        let mut h = fold(seed, bits(self.fg), A);
        h = fold(h, bits(self.bg), B);
        h = fold(h, self.underline_color.map_or(0, bits), A);
        fold(h, self.attrs.bits() as u64, B)
    }
}

impl Default for Style {
    fn default() -> Style {
        Style::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::Style;
    use crate::types::{Color, NamedColor, Rgb};

    const SEED: u64 = 0x1234_5678_9ABC_DEF0;

    fn hash_with_underline(color: Option<Color>) -> u64 {
        Style { underline_color: color, ..Style::DEFAULT }.table_hash(SEED)
    }

    #[test]
    fn the_first_color_of_each_kind_hashes_differently_from_no_color_and_from_the_others() {
        // All of them have the number 0, so only their kind tells them apart.
        let hashes = [
            None,
            Some(Color::Named(NamedColor::Black)),
            Some(Color::Indexed(0)),
            Some(Color::Rgb(Rgb { r: 0, g: 0, b: 0 })),
        ]
        .map(hash_with_underline);
        for (i, a) in hashes.iter().enumerate() {
            for b in &hashes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn colors_of_one_kind_hash_by_their_number() {
        let named = |color| hash_with_underline(Some(Color::Named(color)));
        assert_ne!(named(NamedColor::Red), named(NamedColor::Green));
        let indexed = |n| hash_with_underline(Some(Color::Indexed(n)));
        assert_ne!(indexed(1), indexed(2));
    }

    #[test]
    fn about_half_the_bits_of_a_hash_are_set() {
        // A folded product has half its bits set on average. The two halves ORed, as a slip of the
        // operator would make it, have three quarters.
        let colors = 2000_u32;
        let set: u32 = (0..colors)
            .map(|n| {
                let fg = Color::Rgb(Rgb { r: n as u8, g: (n >> 8) as u8, b: (n * 7) as u8 });
                Style { fg, ..Style::DEFAULT }.table_hash(SEED).count_ones()
            })
            .sum();
        assert!((28 * colors..36 * colors).contains(&set), "{set} bits set in {colors} hashes");
    }
}
