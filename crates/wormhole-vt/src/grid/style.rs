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

    /// A hash of all four fields, for the table's index.
    pub(super) fn table_hash(&self) -> u64 {
        fn bits(color: Color) -> u64 {
            match color {
                Color::Named(named) => 1 << 32 | named as u16 as u64,
                Color::Indexed(ix) => 2 << 32 | ix as u64,
                Color::Rgb(rgb) => 3 << 32 | (rgb.r as u64) << 16 | (rgb.g as u64) << 8 | rgb.b as u64,
            }
        }
        const A: u64 = 0x9E37_79B9_7F4A_7C15;
        const B: u64 = 0xC2B2_AE3D_27D4_EB4F;
        let mut h = bits(self.fg).wrapping_mul(A);
        h = (h.rotate_left(23) ^ bits(self.bg)).wrapping_mul(B);
        h = (h.rotate_left(23) ^ self.underline_color.map_or(0, bits)).wrapping_mul(A);
        h = (h.rotate_left(23) ^ self.attrs.bits() as u64).wrapping_mul(B);
        h ^ (h >> 29)
    }
}

impl Default for Style {
    fn default() -> Style {
        Style::DEFAULT
    }
}
