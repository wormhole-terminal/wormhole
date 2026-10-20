//! SGR: the rendition of the pen.
//!
//! Spec: ECMA-48 8.3.117 for the attributes and the 8 colors, xterm's ctlseqs for 21 (double
//! underline), 53 and 55 (overline), the 90..=97 and 100..=107 bright colors and the 38/48
//! extended colors (`38;5;n`, `38;2;r;g;b` and the colon forms `38:5:n`, `38:2::r:g:b` with an
//! optional color space id), and kitty's text styling for `4:0`..`4:5` (underline styles) and
//! 58/59 (the underline color).
//!
//! A parameter that is not understood is skipped on its own: the ones after it still apply
//! (ECMA-48 says the aspects are independent). A color with missing or out-of-range numbers is
//! dropped with its numbers.

use crate::grid::Style;
use crate::parser::Csi;
use crate::types::{Color, Flags, NamedColor, Rgb};

use crate::terminal::Terminal;

/// The eight base colors, named so that SGR 30..=37 and 90..=97 map by offset.
const BASE: [NamedColor; 8] = [
    NamedColor::Black,
    NamedColor::Red,
    NamedColor::Green,
    NamedColor::Yellow,
    NamedColor::Blue,
    NamedColor::Magenta,
    NamedColor::Cyan,
    NamedColor::White,
];
const BRIGHT: [NamedColor; 8] = [
    NamedColor::BrightBlack,
    NamedColor::BrightRed,
    NamedColor::BrightGreen,
    NamedColor::BrightYellow,
    NamedColor::BrightBlue,
    NamedColor::BrightMagenta,
    NamedColor::BrightCyan,
    NamedColor::BrightWhite,
];

/// Which color a 38, 48 or 58 sets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Foreground,
    Background,
    Underline,
}

impl Layer {
    fn from_code(code: u16) -> Option<Layer> {
        match code {
            38 => Some(Layer::Foreground),
            48 => Some(Layer::Background),
            58 => Some(Layer::Underline),
            _ => None,
        }
    }
}

impl Style {
    /// Apply an SGR code that has no parameters of its own. Unknown codes change nothing.
    #[inline]
    fn apply(&mut self, code: u16) {
        match code {
            0 => *self = Style::DEFAULT,
            1 => self.attrs.insert(Flags::BOLD),
            2 => self.attrs.insert(Flags::DIM),
            3 => self.attrs.insert(Flags::ITALIC),
            4 => self.set_underline(Flags::UNDERLINE),
            5 | 6 => self.attrs.insert(Flags::BLINK),
            7 => self.attrs.insert(Flags::INVERSE),
            8 => self.attrs.insert(Flags::HIDDEN),
            9 => self.attrs.insert(Flags::STRIKEOUT),
            21 => self.set_underline(Flags::DOUBLE_UNDERLINE),
            22 => self.attrs.remove(Flags::BOLD | Flags::DIM),
            23 => self.attrs.remove(Flags::ITALIC),
            24 => self.attrs.remove(Flags::ALL_UNDERLINES),
            25 => self.attrs.remove(Flags::BLINK),
            27 => self.attrs.remove(Flags::INVERSE),
            28 => self.attrs.remove(Flags::HIDDEN),
            29 => self.attrs.remove(Flags::STRIKEOUT),
            30..=37 => self.fg = Color::Named(BASE[usize::from(code - 30)]),
            39 => self.fg = Color::Named(NamedColor::Foreground),
            40..=47 => self.bg = Color::Named(BASE[usize::from(code - 40)]),
            49 => self.bg = Color::Named(NamedColor::Background),
            53 => self.attrs.insert(Flags::OVERLINE),
            55 => self.attrs.remove(Flags::OVERLINE),
            59 => self.underline_color = None,
            90..=97 => self.fg = Color::Named(BRIGHT[usize::from(code - 90)]),
            100..=107 => self.bg = Color::Named(BRIGHT[usize::from(code - 100)]),
            _ => {}
        }
    }

    /// One underline style at a time.
    fn set_underline(&mut self, style: Flags) {
        self.attrs.remove(Flags::ALL_UNDERLINES);
        self.attrs.insert(style);
    }

    fn set_color(&mut self, slot: Layer, color: Color) {
        match slot {
            Layer::Foreground => self.fg = color,
            Layer::Background => self.bg = color,
            Layer::Underline => self.underline_color = Some(color),
        }
    }

    /// `4:n`, the underline style of kitty's text styling.
    fn apply_underline_style(&mut self, n: u16) {
        match n {
            0 => self.attrs.remove(Flags::ALL_UNDERLINES),
            1 => self.set_underline(Flags::UNDERLINE),
            2 => self.set_underline(Flags::DOUBLE_UNDERLINE),
            3 => self.set_underline(Flags::UNDERCURL),
            4 => self.set_underline(Flags::DOTTED_UNDERLINE),
            5 => self.set_underline(Flags::DASHED_UNDERLINE),
            _ => {}
        }
    }
}

fn rgb(r: u16, g: u16, b: u16) -> Option<Color> {
    Some(Color::Rgb(Rgb { r: u8::try_from(r).ok()?, g: u8::try_from(g).ok()?, b: u8::try_from(b).ok()? }))
}

fn indexed(n: u16) -> Option<Color> {
    u8::try_from(n).ok().map(Color::Indexed)
}

/// The color a colon group `[38, 5, n]` or `[38, 2, (id,) r, g, b]` sets.
fn colon_color(group: &[u16]) -> Option<Color> {
    match group {
        [_, 5, n] => indexed(*n),
        [_, 2, r, g, b] | [_, 2, _, r, g, b] => rgb(*r, *g, *b),
        _ => None,
    }
}

impl Terminal {
    /// SGR (`CSI Pm m`). Nothing is allocated; this is a hot path (every colored word of a
    /// program's output).
    pub(in crate::terminal) fn sgr(&mut self, csi: &Csi) {
        let style = &mut self.pen.style;
        if csi.has_subparams() {
            sgr_groups(style, csi);
            return;
        }
        // Without colons the parameters are a flat list; 38, 48 and 58 take their numbers from the
        // ones after them.
        let values = csi.values();
        let mut i = 0;
        while i < values.len() {
            let code = values[i];
            i += 1;
            if let Some(slot) = Layer::from_code(code) {
                i += semicolon_color(style, slot, &values[i..]);
            } else {
                style.apply(code);
            }
        }
    }
}

/// `38;5;n` or `38;2;r;g;b` in `rest` (the values after the 38): apply it and return how many of
/// them it used. A color that is cut short or malformed uses what is there.
fn semicolon_color(style: &mut Style, slot: Layer, rest: &[u16]) -> usize {
    match rest {
        [5, n, ..] => {
            if let Some(color) = indexed(*n) {
                style.set_color(slot, color);
            }
            2
        }
        [2, r, g, b, ..] => {
            if let Some(color) = rgb(*r, *g, *b) {
                style.set_color(slot, color);
            }
            4
        }
        // `38;2;r;g` and `38;5` run out: use up what there is.
        [5 | 2, tail @ ..] => 1 + tail.len(),
        // Not a color selector (ECMA-48 would call this a private use): skip nothing more.
        _ => 0,
    }
}

/// SGR with colons somewhere: walk the parameters as groups.
fn sgr_groups(style: &mut Style, csi: &Csi) {
    let mut groups = csi.groups();
    while let Some(group) = groups.next() {
        let code = group[0];
        if let Some(slot) = Layer::from_code(code) {
            if group.len() > 1 {
                if let Some(color) = colon_color(group) {
                    style.set_color(slot, color);
                }
            } else {
                // The semicolon form, among parameters that use colons elsewhere: the numbers
                // are the next groups' first values.
                let selector = groups.clone().next().map(|g| g[0]);
                let take = match selector {
                    Some(5) => 2,
                    Some(2) => 4,
                    _ => 0,
                };
                let mut numbers = [0u16; 4];
                let mut got = 0;
                for _ in 0..take {
                    if let Some(next) = groups.next() {
                        numbers[got] = next[0];
                        got += 1;
                    }
                }
                let color = match (selector, got) {
                    (Some(5), 2) => indexed(numbers[1]),
                    (Some(2), 4) => rgb(numbers[1], numbers[2], numbers[3]),
                    _ => None,
                };
                if let Some(color) = color {
                    style.set_color(slot, color);
                }
            }
        } else if code == 4 && group.len() > 1 {
            style.apply_underline_style(group[1]);
        } else {
            style.apply(code);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::grid::Style;
    use crate::types::{Color, Flags, NamedColor, Rgb};

    #[test]
    fn codes_set_and_reset_attributes_and_colors() {
        let mut style = Style::DEFAULT;
        for code in [1, 3, 4, 31, 44, 53] {
            style.apply(code);
        }
        assert!(style.attrs.contains(Flags::BOLD | Flags::ITALIC | Flags::UNDERLINE | Flags::OVERLINE));
        assert_eq!(style.fg, Color::Named(NamedColor::Red));
        assert_eq!(style.bg, Color::Named(NamedColor::Blue));
        style.apply(21);
        assert!(style.attrs.contains(Flags::DOUBLE_UNDERLINE) && !style.attrs.contains(Flags::UNDERLINE));
        style.apply(0);
        assert_eq!(style, Style::DEFAULT);
    }

    #[test]
    fn extended_colors_check_their_range() {
        assert_eq!(super::rgb(1, 2, 3), Some(Color::Rgb(Rgb { r: 1, g: 2, b: 3 })));
        assert_eq!(super::rgb(256, 0, 0), None);
        assert_eq!(super::indexed(300), None);
        assert_eq!(super::colon_color(&[38, 2, 0, 1, 2, 3]), super::rgb(1, 2, 3));
        assert_eq!(super::colon_color(&[38, 2, 1, 2, 3]), super::rgb(1, 2, 3));
        assert_eq!(super::colon_color(&[38, 5, 9]), super::indexed(9));
        assert_eq!(super::colon_color(&[38, 5]), None);
    }
}
