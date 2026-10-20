//! One cell of the screen as a renderer draws it.

use bitflags::bitflags;

use super::color::{Color, NamedColor};

bitflags! {
    /// A cell's attributes (SGR) and how it sits in a row.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Flags: u32 {
        const BOLD             = 1;
        const DIM              = 1 << 1;
        const ITALIC           = 1 << 2;
        /// The underline styles of SGR 4 and 4:1–4:5 are one flag each, and at most one is set.
        const UNDERLINE        = 1 << 3;
        const DOUBLE_UNDERLINE = 1 << 4;
        const UNDERCURL        = 1 << 5;
        const DOTTED_UNDERLINE = 1 << 6;
        const DASHED_UNDERLINE = 1 << 7;
        /// SGR 5 and 6.
        const BLINK            = 1 << 8;
        const INVERSE          = 1 << 9;
        /// SGR 8: drawn in the background's color.
        const HIDDEN           = 1 << 10;
        const STRIKEOUT        = 1 << 11;
        /// SGR 53.
        const OVERLINE         = 1 << 12;

        /// The first of the two cells of a double-width character, which holds the character.
        const WIDE_CHAR                = 1 << 13;
        /// The second, which holds nothing.
        const WIDE_CHAR_SPACER         = 1 << 14;
        /// A blank at the end of a row where a double-width character didn't fit and moved to
        /// the next row.
        const LEADING_WIDE_CHAR_SPACER = 1 << 15;
        /// On a row's last cell: the line goes on in the next row (a soft wrap, not a newline).
        const WRAPLINE                 = 1 << 16;

        /// Any underline style.
        const ALL_UNDERLINES = Self::UNDERLINE.bits()
            | Self::DOUBLE_UNDERLINE.bits()
            | Self::UNDERCURL.bits()
            | Self::DOTTED_UNDERLINE.bits()
            | Self::DASHED_UNDERLINE.bits();
    }
}

/// One cell: a character, its two colors and its attributes. Sixteen bytes and `Copy`, so a row
/// is a flat slice that copies with a memcpy.
///
/// What is rare on a cell is not in it: the zero-width characters that combine with `c`, an
/// underline color of its own, a hyperlink. They sit beside the row ([`RowExtras`]), so the common
/// cell pays nothing for them and copying a row never clones a pointer.
///
/// [`RowExtras`]: super::RowExtras
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub c: char,
    pub fg: Color,
    pub bg: Color,
    pub flags: Flags,
}

impl Default for Cell {
    /// A blank in the default colors.
    #[inline]
    fn default() -> Cell {
        Cell {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cell, Flags};

    #[test]
    fn a_cell_is_sixteen_bytes_and_copy() {
        assert!(std::mem::size_of::<Cell>() <= 16, "{} bytes", std::mem::size_of::<Cell>());
        fn is_copy<T: Copy>() {}
        is_copy::<Cell>();
    }

    #[test]
    fn the_underline_styles_are_a_group_of_their_own() {
        for style in [
            Flags::UNDERLINE,
            Flags::DOUBLE_UNDERLINE,
            Flags::UNDERCURL,
            Flags::DOTTED_UNDERLINE,
            Flags::DASHED_UNDERLINE,
        ] {
            assert!(Flags::ALL_UNDERLINES.contains(style));
        }
        assert!(!Flags::ALL_UNDERLINES.intersects(Flags::BLINK | Flags::OVERLINE | Flags::STRIKEOUT));
    }

    #[test]
    fn a_default_cell_is_a_blank() {
        let cell = Cell::default();
        assert_eq!((cell.c, cell.flags), (' ', Flags::empty()));
    }
}
