use gpui_kit::{Hsla, rgb};
use wormhole_core::terminal::types::{COLOR_COUNT, Cell, Color, ColorScheme, Colors, Flags, NamedColor, Rgb};

/// Terminal colors, resolved from the active theme. Comparable, so a settings
/// change that leaves them alone can keep the same palette (and the terminal
/// its shaped rows).
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub ansi: [Hsla; 16],
    pub foreground: Hsla,
    pub background: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
    /// Text on selected cells, when the theme sets one (otherwise the cell's own).
    pub selection_fg: Option<Hsla>,
    /// The character under a block cursor, when the theme sets one (otherwise
    /// the cell's background, so it reads as cut out of the cursor).
    pub cursor_text: Option<Hsla>,
    /// Find's matches: every match's tint, and the current one's fill and text.
    pub find_match: Hsla,
    pub find_current: Hsla,
    pub find_current_text: Hsla,
    /// Indexed colors 16–255 (the 6×6×6 cube and the gray ramp), converted
    /// once instead of per cell and frame.
    extended: [Hsla; 240],
}

impl Palette {
    pub fn from_theme(theme: &wormhole_core::theme::Theme) -> Self {
        let color = |c: u32| -> Hsla { rgb(c).into() };
        let yellow = color(theme.palette[3]);
        Self {
            ansi: theme.palette.map(color),
            foreground: color(theme.foreground),
            background: color(theme.background),
            cursor: color(theme.cursor),
            selection: color(theme.selection_background),
            selection_fg: theme.selection_foreground.map(color),
            cursor_text: theme.cursor_text.map(color),
            // The tokens' rules say better (`with_tokens`); a palette from a bare theme uses yellow.
            find_match: yellow.opacity(0.32),
            find_current: yellow,
            find_current_text: color(0),
            extended: std::array::from_fn(|i| to_hsla(extended_rgb(i as u8 + 16))),
        }
    }

    /// Take the colors the terminal draws that are design tokens (find's matches) from `tokens`.
    pub fn with_tokens(mut self, tokens: &wormhole_core::tokens::ThemeTokens) -> Self {
        let token = |name| crate::theme_bridge::token(tokens, name);
        self.find_match = token("find.match");
        self.find_current = token("find.match.current");
        self.find_current_text = token("find.match.current.text");
        self
    }

    /// A match's background over a cell's: a tint, so the cell's own color still shows through.
    pub fn found_background(&self, bg: Hsla) -> Hsla {
        // The terminal's background isn't painted (the pane's shows through): tint that instead.
        if bg == self.background { self.find_match } else { bg.blend(self.find_match) }
    }

    /// Resolve a cell color, honoring runtime overrides set via OSC 4/10/11.
    pub fn resolve(&self, color: Color, overrides: &Colors) -> Hsla {
        match color {
            Color::Rgb(rgb) => to_hsla(rgb),
            Color::Named(named) => overrides[named].map(to_hsla).unwrap_or_else(|| self.named(named)),
            Color::Indexed(index) => {
                overrides[index as usize].map(to_hsla).unwrap_or_else(|| self.indexed(index))
            }
        }
    }

    /// A cell's text and background colors as drawn: attributes applied, and
    /// the selection's colors when `selected`.
    pub fn cell_colors(&self, cell: &Cell, selected: bool, overrides: &Colors) -> (Hsla, Hsla) {
        let mut fg = self.resolve(cell.fg, overrides);
        let mut bg = self.resolve(cell.bg, overrides);
        if cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if cell.flags.contains(Flags::DIM) {
            fg = dim(fg);
        }
        let hidden = cell.flags.contains(Flags::HIDDEN);
        if hidden {
            fg = bg;
        }
        if selected {
            bg = self.selection;
            // Without it, text in the selection color disappears (Kanagawa,
            // GitHub Light: foreground and selection are the same color).
            if let Some(selection_fg) = self.selection_fg.filter(|_| !hidden) {
                fg = selection_fg;
            }
        }
        (fg, bg)
    }

    /// What a program gets back when it asks for palette entry `index` (OSC 4,
    /// or 256/257/258 for OSC 10/11/12): the color it set, else the theme's.
    pub fn query(&self, index: usize, overrides: &Colors) -> Rgb {
        if let Some(rgb) = (index < COLOR_COUNT).then(|| overrides[index]).flatten() {
            return rgb;
        }
        to_rgb(match index {
            0..=255 => self.indexed(index as u8),
            257 => self.background,
            258 => self.cursor,
            _ => self.foreground,
        })
    }

    /// Whether the terminal's background is light or dark, for the programs that ask
    /// (`CSI ? 996 n`): by its luma, which is what a program choosing its colors cares about.
    pub fn color_scheme(&self) -> ColorScheme {
        let Rgb { r, g, b } = to_rgb(self.background);
        // Rec. 601 weights in thousandths; half of 255,000 is the middle.
        let luma = 299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b);
        if luma >= 127_500 { ColorScheme::Light } else { ColorScheme::Dark }
    }

    /// The theme's answer for every entry a program can ask about, for the engine to answer with
    /// (it keeps what a program set itself).
    pub fn engine_colors(&self) -> Box<[Rgb; COLOR_COUNT]> {
        let none = Colors::default();
        let mut colors = Box::new([Rgb { r: 0, g: 0, b: 0 }; COLOR_COUNT]);
        for (slot, color) in colors.iter_mut().enumerate() {
            *color = self.query(slot, &none);
        }
        colors
    }

    fn named(&self, named: NamedColor) -> Hsla {
        match named {
            NamedColor::Foreground => self.foreground,
            NamedColor::Background => self.background,
            NamedColor::Cursor => self.cursor,
            // The ANSI colors are the first sixteen slots, in the theme's order.
            ansi => self.ansi[ansi as usize],
        }
    }

    fn indexed(&self, index: u8) -> Hsla {
        match index {
            0..16 => self.ansi[index as usize],
            _ => self.extended[index as usize - 16],
        }
    }
}

/// The xterm color of index 16–255.
fn extended_rgb(index: u8) -> Rgb {
    match index {
        16..232 => {
            let i = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { v * 40 + 55 };
            Rgb { r: level(i / 36), g: level((i / 6) % 6), b: level(i % 6) }
        }
        _ => {
            let v = 8 + 10 * (index.max(232) - 232);
            Rgb { r: v, g: v, b: v }
        }
    }
}

pub fn dim(color: Hsla) -> Hsla {
    Hsla { a: color.a * 0.66, ..color }
}

pub fn to_hsla(c: Rgb) -> Hsla {
    rgb(((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32).into()
}

pub fn to_rgb(color: Hsla) -> Rgb {
    let c = color.to_rgb();
    Rgb { r: (c.r * 255.0).round() as u8, g: (c.g * 255.0).round() as u8, b: (c.b * 255.0).round() as u8 }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, rgb};
    use wormhole_core::terminal::types::{Cell, Color, ColorScheme, Colors, Flags, NamedColor, Rgb};
    use wormhole_core::theme;
    use wormhole_core::tokens::Color as TokenColor;

    use super::{Palette, to_rgb};

    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let token = |c: Hsla| {
            let Rgb { r, g, b } = to_rgb(c);
            TokenColor::from_rgb_u32(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
        };
        TokenColor::contrast_ratio(token(a), token(b))
    }

    #[test]
    fn selected_text_stays_readable_in_every_bundled_theme() {
        let overrides = Colors::default();
        let text = Cell { c: 'x', ..Cell::default() };
        for theme in theme::builtin() {
            let palette = Palette::from_theme(&theme);
            let (fg, bg) = palette.cell_colors(&text, true, &overrides);
            if let Some(selection_fg) = theme.selection_foreground {
                assert_eq!(to_rgb(fg), to_rgb(gpui_kit::rgb(selection_fg).into()), "{}", theme.name);
            }
            // Catppuccin Latte's own pair is 2.34:1; Kanagawa and GitHub Light were 1.00.
            let ratio = contrast(fg, bg);
            assert!(ratio >= 2.0, "{}: selected text at {ratio:.2}:1", theme.name);
        }
    }

    #[test]
    fn unselected_and_hidden_cells_keep_their_colors() {
        let overrides = Colors::default();
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let red = Cell { c: 'x', fg: Color::Named(NamedColor::Red), ..Cell::default() };
        assert_eq!(palette.cell_colors(&red, false, &overrides), (palette.ansi[1], palette.background));
        let inverse = Cell { flags: Flags::INVERSE, ..red };
        assert_eq!(palette.cell_colors(&inverse, false, &overrides), (palette.background, palette.ansi[1]));
        let hidden = Cell { flags: Flags::HIDDEN, ..red };
        let (fg, bg) = palette.cell_colors(&hidden, true, &overrides);
        assert_eq!((fg, bg), (palette.background, palette.selection), "a selection doesn't reveal it");
    }

    #[test]
    fn color_queries_answer_what_programs_set_and_the_whole_256_palette() {
        let mut overrides = Colors::default();
        let palette = Palette::from_theme(&theme::builtin()[0]);
        // 196 is pure red in the 6×6×6 cube; it used to come back as the foreground.
        assert_eq!(palette.query(196, &overrides), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(palette.query(232, &overrides), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(palette.query(1, &overrides), to_rgb(palette.ansi[1]));
        assert_eq!(palette.query(257, &overrides), to_rgb(palette.background));
        // OSC 11 set the background: that's the answer now.
        overrides[NamedColor::Background] = Some(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(palette.query(257, &overrides), Rgb { r: 1, g: 2, b: 3 });
    }

    #[test]
    fn the_engine_gets_the_themes_answer_for_every_entry() {
        let themes = theme::builtin();
        let palette = Palette::from_theme(&themes[0]);
        let colors = palette.engine_colors();
        let none = Colors::default();
        assert!(colors.iter().enumerate().all(|(slot, color)| *color == palette.query(slot, &none)));
        assert_eq!(colors[257], to_rgb(palette.background));
    }

    #[test]
    fn the_scheme_follows_the_backgrounds_luma() {
        let mut palette = Palette::from_theme(&theme::builtin()[0]);
        for (background, scheme) in [
            (0x000000, ColorScheme::Dark),
            (0x1e1e2e, ColorScheme::Dark),
            (0x0000ff, ColorScheme::Dark),
            (0xffffff, ColorScheme::Light),
            (0xfdf6e3, ColorScheme::Light),
            (0xffff00, ColorScheme::Light),
        ] {
            palette.background = rgb(background).into();
            assert_eq!(palette.color_scheme(), scheme, "{background:06x}");
        }
    }

    #[test]
    fn palettes_compare_by_their_colors() {
        let themes = theme::builtin();
        assert_eq!(Palette::from_theme(&themes[0]), Palette::from_theme(&themes[0]));
        assert_ne!(Palette::from_theme(&themes[0]), Palette::from_theme(&themes[1]));
    }

    #[test]
    fn the_extended_table_matches_xterm() {
        let palette = Palette::from_theme(&theme::builtin()[0]);
        let overrides = Colors::default();
        for (index, rgb) in [(16, (0, 0, 0)), (21, (0, 0, 255)), (59, (95, 95, 95)), (231, (255, 255, 255))] {
            let (r, g, b) = rgb;
            assert_eq!(
                to_rgb(palette.resolve(Color::Indexed(index), &overrides)),
                Rgb { r, g, b },
                "{index}"
            );
        }
        assert_eq!(to_rgb(palette.resolve(Color::Indexed(255), &overrides)), Rgb { r: 238, g: 238, b: 238 });
    }
}
