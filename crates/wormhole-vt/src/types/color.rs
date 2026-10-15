//! Colors as a cell carries them, and the palette entries a program has overridden.
//!
//! The palette is xterm's: sixteen ANSI colors, the 6×6×6 cube and the gray ramp (indices 0–255,
//! SGR 38;5 and OSC 4), then the three dynamic colors OSC 10, 11 and 12 set, in that order:
//! foreground, background, text cursor.

use std::ops::{Index, IndexMut};

/// A true color.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// The palette entries that have a name: the ANSI colors, and the terminal's own three. The
/// discriminant is the entry's slot in [`Colors`], so it casts to `usize` for indexing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
    /// The default text color (SGR 39; OSC 10 sets it).
    Foreground = 256,
    /// The default background (SGR 49; OSC 11).
    Background = 257,
    /// The text cursor's color (OSC 12).
    Cursor = 258,
}

/// A color as set on a cell, not yet resolved against the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// One of the terminal's own: an ANSI color by name, or the defaults.
    Named(NamedColor),
    /// An entry of the 256-color palette (SGR 38;5 and 48;5).
    Indexed(u8),
    /// A true color (SGR 38;2 and 48;2).
    Rgb(Rgb),
}

/// How many slots [`Colors`] has: the 256 indexed entries, then foreground, background and
/// cursor.
pub const COLOR_COUNT: usize = 259;

/// The palette entries programs have overridden at run time (OSC 4, 10, 11, 12), by slot; `None`
/// is "whatever the theme says". Slots 0–255 are the indexed colors, 256–258 the [`NamedColor`]s
/// that follow them.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Colors([Option<Rgb>; COLOR_COUNT]);

impl Default for Colors {
    fn default() -> Self {
        Self([None; COLOR_COUNT])
    }
}

impl std::fmt::Debug for Colors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only what is set: 259 `None`s would bury it.
        let set = self.0.iter().enumerate().filter_map(|(slot, color)| Some((slot, (*color)?)));
        f.debug_map().entries(set).finish()
    }
}

impl Colors {
    /// The override at `slot`; `None` when there is none, and also for a slot past the palette
    /// (a program can name any number), where indexing would panic.
    #[inline]
    pub fn get(&self, slot: usize) -> Option<Rgb> {
        self.0.get(slot).copied().flatten()
    }

    /// The entry at `slot` to set or clear, `None` for a slot past the palette.
    #[inline]
    pub fn get_mut(&mut self, slot: usize) -> Option<&mut Option<Rgb>> {
        self.0.get_mut(slot)
    }
}

impl Index<usize> for Colors {
    type Output = Option<Rgb>;

    #[inline]
    fn index(&self, slot: usize) -> &Option<Rgb> {
        &self.0[slot]
    }
}

impl IndexMut<usize> for Colors {
    #[inline]
    fn index_mut(&mut self, slot: usize) -> &mut Option<Rgb> {
        &mut self.0[slot]
    }
}

impl Index<NamedColor> for Colors {
    type Output = Option<Rgb>;

    #[inline]
    fn index(&self, named: NamedColor) -> &Option<Rgb> {
        &self.0[named as usize]
    }
}

impl IndexMut<NamedColor> for Colors {
    #[inline]
    fn index_mut(&mut self, named: NamedColor) -> &mut Option<Rgb> {
        &mut self.0[named as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::{COLOR_COUNT, Color, Colors, NamedColor, Rgb};

    #[test]
    fn the_dynamic_colors_follow_the_palette_in_osc_order() {
        assert_eq!(NamedColor::Red as usize, 1);
        assert_eq!(NamedColor::BrightWhite as usize, 15);
        assert_eq!(NamedColor::Foreground as usize, 256);
        assert_eq!(NamedColor::Background as usize, 257);
        assert_eq!(NamedColor::Cursor as usize, COLOR_COUNT - 1);
    }

    #[test]
    fn overrides_are_set_by_slot_or_by_name_and_start_empty() {
        let mut colors = Colors::default();
        assert!((0..COLOR_COUNT).all(|slot| colors[slot].is_none()));
        colors[NamedColor::Background] = Some(Rgb { r: 1, g: 2, b: 3 });
        colors[4] = Some(Rgb { r: 9, g: 8, b: 7 });
        assert_eq!(colors[257], Some(Rgb { r: 1, g: 2, b: 3 }));
        assert_eq!(colors[NamedColor::Blue], Some(Rgb { r: 9, g: 8, b: 7 }));
        assert_eq!(format!("{colors:?}"), "{4: Rgb { r: 9, g: 8, b: 7 }, 257: Rgb { r: 1, g: 2, b: 3 }}");
    }

    #[test]
    fn a_slot_past_the_palette_is_no_panic() {
        let mut colors = Colors::default();
        let red = Rgb { r: 255, g: 0, b: 0 };
        assert_eq!(colors.get(1), None);
        if let Some(entry) = colors.get_mut(1) {
            *entry = Some(red);
        }
        assert_eq!(colors.get(1), Some(red));
        assert_eq!(colors.get(COLOR_COUNT - 1), None);
        assert_eq!((colors.get(COLOR_COUNT), colors.get(usize::MAX)), (None, None));
        assert!(colors.get_mut(COLOR_COUNT).is_none());
    }

    #[test]
    fn a_color_is_four_bytes() {
        // Two of them and a character are most of a cell's sixteen.
        assert_eq!(std::mem::size_of::<Color>(), 4);
        assert_eq!(std::mem::size_of::<Option<Color>>(), 4, "an underline color costs no more");
    }
}
