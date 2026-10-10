//! A folder's colour (right-click its band in the terminal list, or the Project tab): one of a
//! few names (or a hex colour picked in the Project tab, see `ProjectColorChoice`). The names are the app's, and what each looks like comes from the
//! theme, so a folder marked "Teal" is as bright, as muted and as light or dark as the theme's own
//! colours and a theme file never has to know about it. What is stored is the name
//! (`Settings::folder_colors`, by the folder's path).

use crate::tokens::{Color, ThemeTokens};

/// The colours a folder can wear, in the order they are offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FolderColor {
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Blue,
    Purple,
    Pink,
}

/// The least a colour is kept from the surfaces it is drawn on, as for the status colours: the
/// 3:1 WCAG asks of a mark that isn't text.
const MARK_CONTRAST: f32 = 3.0;
/// The least chroma a folder colour has, in a theme whose palette is nearly gray: below it a "Teal"
/// and a "Green" could not be told apart.
const CHROMA_MIN: f32 = 0.08;

impl FolderColor {
    pub const ALL: [FolderColor; 8] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Teal,
        Self::Blue,
        Self::Purple,
        Self::Pink,
    ];

    /// How it is written in `config.json` and said in commands: lower case.
    pub fn key(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Teal => "teal",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
        }
    }

    /// How menus and VoiceOver say it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Teal => "Teal",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Pink => "Pink",
        }
    }

    /// The colour named by `key`, if it is one of ours. A file written by a later build may name
    /// one this build doesn't know; that is a folder without a colour here, and stays in the file.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.key() == key)
    }

    /// Where on the colour wheel (OKLCH hue, degrees) the name is. The names stay what they say
    /// in every theme: a terminal palette's own "blue" is orange in some of them (Monokai Pro),
    /// so the hue is fixed here and only the lightness and chroma come from the theme.
    fn hue(self) -> f32 {
        match self {
            Self::Red => 25.0,
            Self::Orange => 55.0,
            Self::Yellow => 95.0,
            Self::Green => 145.0,
            Self::Teal => 190.0,
            Self::Blue => 255.0,
            Self::Purple => 305.0,
            Self::Pink => 350.0,
        }
    }

    /// The colour in `tokens`' theme: the hue of its name at the lightness and chroma of the
    /// theme's own six colours (ANSI red to cyan), so a pastel theme gets pastels and a light one
    /// darker colours, then moved far enough from the window and the sidebar to be seen on both.
    pub fn color(self, tokens: &ThemeTokens) -> Color {
        let (lightness, chroma) = palette_character(tokens);
        let color = Color::from_oklch(lightness, chroma, self.hue());
        [tokens.get("surface.window"), tokens.get("surface.panel")]
            .into_iter()
            .flatten()
            .fold(color, |color, surface| color.readable_on(surface.with_alpha(1.0), MARK_CONTRAST))
            .with_alpha(1.0)
    }
}

/// The average OKLCH lightness and chroma of the palette's colours (ANSI 1 to 6, red to cyan).
fn palette_character(tokens: &ThemeTokens) -> (f32, f32) {
    let colors = &tokens.terminal.ansi[1..=6];
    let (lightness, chroma) = colors.iter().fold((0.0, 0.0), |(l, c), color| {
        let (cl, cc, _) = color.to_oklch();
        (l + cl, c + cc)
    });
    let n = colors.len() as f32;
    (lightness / n, (chroma / n).max(CHROMA_MIN))
}

#[cfg(test)]
mod tests {
    use super::{CHROMA_MIN, FolderColor, MARK_CONTRAST};
    use crate::tokens::{Color, builtins};

    /// The distance between two hues on the wheel, in degrees.
    fn apart(a: f32, b: f32) -> f32 {
        let d = (a - b).abs() % 360.0;
        d.min(360.0 - d)
    }

    #[test]
    fn a_colour_is_written_by_its_lower_case_name() {
        for color in FolderColor::ALL {
            assert_eq!(FolderColor::from_key(color.key()), Some(color));
            assert_eq!(color.key(), color.label().to_lowercase());
        }
        for key in ["", "Red", "brown", " red", "red ", "#ff0000"] {
            assert_eq!(FolderColor::from_key(key), None, "{key:?}");
        }
        assert_eq!(FolderColor::ALL.len(), 8);
    }

    /// Whatever the theme's palette calls its colours (Monokai Pro's "blue" is orange), a name is
    /// the hue it says, and the eight can be told apart.
    #[test]
    fn every_name_is_its_own_hue_in_every_theme() {
        for tokens in builtins() {
            let all = FolderColor::ALL.map(|color| (color, color.color(tokens).to_oklch()));
            for (color, (_, chroma, hue)) in all {
                assert!(
                    apart(hue, color.hue()) < 8.0 && chroma >= 0.05,
                    "{}: {} is {hue:.0} deg at {chroma:.2}",
                    tokens.name,
                    color.key()
                );
            }
            for pair in all.windows(2).chain([[all[7], all[0]].as_slice()]) {
                let (a, b) = (pair[0], pair[1]);
                assert!(
                    apart(a.1.2, b.1.2) >= 25.0,
                    "{}: {} and {} are too close",
                    tokens.name,
                    a.0.key(),
                    b.0.key()
                );
            }
        }
    }

    #[test]
    fn a_colour_is_opaque_and_seen_on_the_window_and_the_sidebar() {
        for tokens in builtins() {
            let on = |name: &str| tokens.get(name).expect(name).with_alpha(1.0);
            for color in FolderColor::ALL {
                let shown = color.color(tokens);
                assert_eq!(shown.a, 1.0);
                for surface in ["surface.window", "surface.panel"] {
                    let ratio = Color::contrast_ratio(shown, on(surface));
                    assert!(
                        ratio >= MARK_CONTRAST - 0.05,
                        "{}: {} on {surface} is {ratio:.2}",
                        tokens.name,
                        color.key()
                    );
                }
            }
        }
    }

    #[test]
    fn the_colours_follow_the_theme_and_its_light_or_dark_side() {
        let find = |name: &str| builtins().iter().find(|t| t.name == name).expect(name);
        let (dark, light) = (find("Catppuccin Mocha"), find("Catppuccin Latte"));
        let blue = |t| FolderColor::Blue.color(t);
        assert!(blue(light).luminance() < blue(dark).luminance(), "a light theme gets the darker blue");
        assert_ne!(
            blue(find("Nord")).to_hex(),
            blue(find("Dracula")).to_hex(),
            "Nord is muted, Dracula is not"
        );
        assert!(
            FolderColor::Blue.color(find("Nord")).to_oklch().1
                < FolderColor::Blue.color(find("Dracula")).to_oklch().1
        );
    }

    #[test]
    fn a_gray_palette_still_gives_colours_that_differ() {
        let mut gray = builtins()[0].clone();
        for ansi in &mut gray.terminal.ansi[1..=6] {
            *ansi = Color::from_hex("#8a8a8a").expect("hex");
        }
        let teal = FolderColor::Teal.color(&gray).to_oklch();
        let green = FolderColor::Green.color(&gray).to_oklch();
        assert!(teal.1 >= CHROMA_MIN - 0.01 && green.1 >= CHROMA_MIN - 0.01, "{teal:?} {green:?}");
        assert!(apart(teal.2, green.2) >= 25.0);
    }
}
