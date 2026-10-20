//! The nine colours a group wears, the way a browser's tab groups do: Grey Blue Red Yellow Green
//! Pink Purple Cyan Orange. Like a folder's colour (`FolderColor`) they are names, and what each
//! looks like comes from the theme; the stored word is the lower-case name.

use crate::folder_color::swatch;
use crate::tokens::{Color, ThemeTokens};

/// How a group's colour washes its card: from `WASH_PEAK` of the colour at the card's left edge down
/// to `WASH_FAR` at `WASH_SPAN` of its width, and `WASH_FAR` from there on.
pub const WASH_PEAK: f32 = 0.12;
pub const WASH_FAR: f32 = 0.03;
pub const WASH_SPAN: f32 = 0.6;

/// A group's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupColor {
    Grey,
    Blue,
    Red,
    Yellow,
    Green,
    Pink,
    Purple,
    Cyan,
    Orange,
}

impl GroupColor {
    /// In the order the group's menu offers them.
    pub const ALL: [GroupColor; 9] = [
        Self::Grey,
        Self::Blue,
        Self::Red,
        Self::Yellow,
        Self::Green,
        Self::Pink,
        Self::Purple,
        Self::Cyan,
        Self::Orange,
    ];

    /// The colours new groups take in turn, so two groups made one after the other don't look
    /// alike (grey is only ever chosen).
    pub const IN_TURN: [GroupColor; 8] = [
        Self::Blue,
        Self::Green,
        Self::Orange,
        Self::Purple,
        Self::Pink,
        Self::Cyan,
        Self::Yellow,
        Self::Red,
    ];

    /// How it is saved with the window: lower case.
    pub fn key(self) -> &'static str {
        match self {
            Self::Grey => "grey",
            Self::Blue => "blue",
            Self::Red => "red",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Pink => "pink",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::Orange => "orange",
        }
    }

    /// How menus and VoiceOver say it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Grey => "Grey",
            Self::Blue => "Blue",
            Self::Red => "Red",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Pink => "Pink",
            Self::Purple => "Purple",
            Self::Cyan => "Cyan",
            Self::Orange => "Orange",
        }
    }

    /// The colour named by `key`. Windows saved before the nine colours called Cyan "teal" and
    /// had no grey; American spelling is accepted too.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "teal" => Some(Self::Cyan),
            "gray" => Some(Self::Grey),
            _ => Self::ALL.into_iter().find(|color| color.key() == key),
        }
    }

    /// Where on the colour wheel (OKLCH hue, degrees) the name is; unused by grey.
    fn hue(self) -> f32 {
        match self {
            Self::Grey => 0.0,
            Self::Red => 25.0,
            Self::Orange => 55.0,
            Self::Yellow => 95.0,
            Self::Green => 145.0,
            Self::Cyan => 190.0,
            Self::Blue => 255.0,
            Self::Purple => 305.0,
            Self::Pink => 350.0,
        }
    }

    /// The colour in `tokens`' theme, as a folder's colour is worked out (`FolderColor::color`):
    /// the hue of its name at the lightness and chroma of the theme's own palette, seen on the
    /// window and the sidebar.
    pub fn color(self, tokens: &ThemeTokens) -> Color {
        swatch(tokens, self.hue(), self == Self::Grey)
    }
}

#[cfg(test)]
mod tests {
    use super::GroupColor;
    use crate::tokens::{Color, builtins};

    #[test]
    fn a_colour_is_written_by_its_lower_case_name() {
        for color in GroupColor::ALL {
            assert_eq!(GroupColor::from_key(color.key()), Some(color));
            assert_eq!(color.key(), color.label().to_lowercase());
        }
        assert_eq!(GroupColor::ALL.len(), 9);
        assert_eq!(GroupColor::from_key("teal"), Some(GroupColor::Cyan), "what 0.x windows saved");
        assert_eq!(GroupColor::from_key("gray"), Some(GroupColor::Grey));
        for key in ["", "Blue", "brown", " red"] {
            assert_eq!(GroupColor::from_key(key), None, "{key:?}");
        }
    }

    #[test]
    fn the_colours_in_turn_are_all_but_grey_and_each_once() {
        let turn = GroupColor::IN_TURN;
        assert!(!turn.contains(&GroupColor::Grey));
        for color in GroupColor::ALL.into_iter().filter(|&c| c != GroupColor::Grey) {
            assert_eq!(turn.iter().filter(|&&c| c == color).count(), 1, "{color:?}");
        }
    }

    #[test]
    fn every_colour_is_seen_on_the_window_and_the_sidebar_in_every_theme() {
        for tokens in builtins() {
            let on = |name: &str| tokens.get(name).expect(name).with_alpha(1.0);
            for color in GroupColor::ALL {
                let shown = color.color(tokens);
                assert_eq!(shown.a, 1.0);
                for surface in ["surface.window", "surface.panel"] {
                    let ratio = Color::contrast_ratio(shown, on(surface));
                    assert!(ratio >= 2.95, "{}: {} on {surface} is {ratio:.2}", tokens.name, color.key());
                }
            }
        }
    }

    /// Text lies on the wash at its strongest (the card's left edge): over the card's own fill, on
    /// the sidebar. The board's text colours are fixed, so the second lines may sink to 3.5 and 4:1
    /// under the strongest wash; the primary text and the amber edge keep their full ratios.
    #[test]
    fn text_on_a_groups_wash_reads_in_every_theme() {
        for tokens in builtins() {
            let get = |name: &str| tokens.get(name).expect(name);
            let column = get("sidebar.background").over(get("surface.window").over(Color::BLACK));
            let card = get("card").over(column);
            for color in GroupColor::ALL {
                let wash = color.color(tokens).with_alpha(super::WASH_PEAK).over(card);
                for (name, min) in [
                    ("text.primary", 4.5),
                    ("text.muted", 3.5),
                    ("agent.attention.text", 4.0),
                    ("agent.attention", 3.0),
                ] {
                    let ratio = Color::contrast_ratio(get(name).over(wash), wash);
                    assert!(
                        ratio >= min - 0.05,
                        "{}: {name} on a {} wash is {ratio:.2}",
                        tokens.name,
                        color.key()
                    );
                }
            }
        }
    }

    #[test]
    fn every_hue_has_colour_in_every_theme() {
        for tokens in builtins() {
            for color in GroupColor::ALL.into_iter().filter(|&c| c != GroupColor::Grey) {
                let (_, chroma, _) = color.color(tokens).to_oklch();
                assert!(chroma >= 0.05, "{}: {} has chroma {chroma:.3}", tokens.name, color.key());
            }
        }
    }

    #[test]
    fn grey_has_no_hue_to_speak_of() {
        for tokens in builtins() {
            let (_, chroma, _) = GroupColor::Grey.color(tokens).to_oklch();
            assert!(chroma < 0.03, "{}: grey has chroma {chroma:.3}", tokens.name);
        }
    }
}
