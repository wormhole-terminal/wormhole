//! The nine colours a group wears, the way a browser's tab groups do: Grey Blue Red Yellow Green
//! Pink Purple Cyan Orange. Like a folder's colour (`FolderColor`) they are names, and what each
//! looks like comes from the theme; the stored word is the lower-case name.

use crate::theme::folder_color::{legible, swatch};
use crate::theme::tokens::{Color, ThemeTokens};

/// How a group's colour washes its card: `WASH_PEAK` of the colour all over it, or, with fading
/// turned on (`fade_group_colors`), from `WASH_PEAK` at the card's left edge down to `WASH_FAR` at
/// `WASH_SPAN` of its width, and `WASH_FAR` from there on. The colour is the group's `wash`, which
/// is the colour as it is seen on the head and in the menu moved just far enough in lightness for
/// the card's text to read on the fill.
pub const WASH_PEAK: f32 = 0.22;
pub const WASH_FAR: f32 = 0.03;
pub const WASH_SPAN: f32 = 0.6;

/// The text a card's wash must leave readable, and the contrast each needs: the primary text, the
/// second lines, and the amber of a terminal that needs you (its text and its edge).
const WASH_TEXT: [(&str, f32); 4] =
    [("text.primary", 4.5), ("text.muted", 3.5), ("agent.attention.text", 4.0), ("agent.attention", 3.0)];

/// The card a group's wash lies on in `tokens`' theme: the card's own fill over the sidebar.
fn wash_backdrop(tokens: &ThemeTokens) -> Option<Color> {
    let column = tokens.get("sidebar.background")?.over(tokens.get("surface.window")?.over(Color::BLACK));
    Some(tokens.get("card")?.over(column))
}

/// `mark` (a group's colour as seen on the head) as its wash is made of: itself where the card's text
/// reads on `WASH_PEAK` of it, else moved along its lightness toward the card's (hue and chroma
/// kept) as little as it takes for every text in `WASH_TEXT` to. Bright colours on a dark theme
/// and dark ones on a light theme are what it moves, since they pull the fill toward the text.
fn wash_tint(tokens: &ThemeTokens, mark: Color) -> Color {
    let Some(card) = wash_backdrop(tokens) else { return mark };
    let reads = |tint: Color| {
        let fill = tint.with_alpha(WASH_PEAK).over(card);
        WASH_TEXT.iter().all(|&(name, min)| {
            tokens.get(name).is_none_or(|text| Color::contrast_ratio(text.over(fill), fill) >= min)
        })
    };
    if reads(mark) {
        return mark;
    }
    let (lightness, chroma, hue) = mark.to_oklch();
    let toward = card.to_oklch().0;
    let at = |t: f32| Color::from_oklch(lightness + (toward - lightness) * t, chroma, hue);
    // Contrast grows as the tint nears the card's lightness, so bisect for the least move.
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..16 {
        let mid = (lo + hi) / 2.0;
        if reads(at(mid)) { hi = mid } else { lo = mid }
    }
    at(hi)
}

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

    /// The colour its card is washed with: [`GroupColor::color`] moved just far enough for the
    /// card's text to read on the fill (see `wash_tint`).
    pub fn wash(self, tokens: &ThemeTokens) -> Color {
        wash_tint(tokens, self.color(tokens))
    }
}

/// What a group is painted with: one of the nine names, or a colour of the person's own, chosen
/// in the system's colour panel and saved as `#rrggbb` (sRGB) in the place of the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupPaint {
    Named(GroupColor),
    Custom([u8; 3]),
}

impl GroupPaint {
    /// A name, or `#rrggbb`; anything else (a name a newer build saved) is nothing.
    pub fn from_key(key: &str) -> Option<Self> {
        if let Some(hex) = key.strip_prefix('#') {
            let valid = hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit());
            let value = u32::from_str_radix(hex, 16).ok().filter(|_| valid)?;
            return Some(Self::Custom([(value >> 16) as u8, (value >> 8) as u8, value as u8]));
        }
        GroupColor::from_key(key).map(Self::Named)
    }

    /// How it is saved with the window.
    pub fn key(self) -> String {
        match self {
            Self::Named(color) => color.key().to_string(),
            Self::Custom([r, g, b]) => format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }

    /// How VoiceOver says it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Named(color) => color.label(),
            Self::Custom(_) => "Custom colour",
        }
    }

    /// The colour in `tokens`' theme. A named one is worked out from the theme (`GroupColor::color`).
    /// One of a person's own is theirs as chosen, light or dark: it is moved along its lightness only
    /// where the theme's window or sidebar would hide it (the same rule that lets the named ones
    /// be seen), and the washes and tints over cards, which are translucent, take it as it comes.
    pub fn color(self, tokens: &ThemeTokens) -> Color {
        match self {
            Self::Named(color) => color.color(tokens),
            Self::Custom([r, g, b]) => {
                legible(tokens, Color::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0))
            }
        }
    }

    /// The colour its card is washed with: [`GroupPaint::color`] moved just far enough for the card's
    /// text to read on the fill. The head's dot and the menu keep the colour itself.
    pub fn wash(self, tokens: &ThemeTokens) -> Color {
        wash_tint(tokens, self.color(tokens))
    }
}

#[cfg(test)]
mod tests {
    use super::{GroupColor, GroupPaint};

    #[test]
    fn a_colour_of_ones_own_is_written_as_hex_and_read_back() {
        let paint = GroupPaint::Custom([0x12, 0xab, 0xff]);
        assert_eq!(paint.key(), "#12abff");
        assert_eq!(GroupPaint::from_key("#12abff"), Some(paint));
        assert_eq!(GroupPaint::from_key("#12ABFF"), Some(paint));
        assert_eq!(GroupPaint::from_key("blue"), Some(GroupPaint::Named(GroupColor::Blue)));
        for key in ["", "#", "#fff", "#12abfg", "#12abff00", "12abff", "+12abff"] {
            assert_eq!(GroupPaint::from_key(key), None, "{key:?}");
        }
    }

    /// Any colour a person picks is seen on the window and the sidebar of every theme, and text
    /// still reads on its wash: the extremes are the cases that would not.
    #[test]
    fn a_colour_of_ones_own_is_seen_and_its_wash_reads_in_every_theme() {
        use crate::theme::tokens::{Color, builtins};
        let extremes = [[0, 0, 0], [255, 255, 255], [128, 128, 128], [255, 255, 0], [0, 0, 255], [255, 0, 0]];
        for tokens in builtins() {
            let get = |name: &str| tokens.get(name).expect(name);
            let column = get("sidebar.background").over(get("surface.window").over(Color::BLACK));
            let card = get("card").over(column);
            for rgb in extremes {
                let shown = GroupPaint::Custom(rgb).color(tokens);
                for surface in ["surface.window", "surface.panel"] {
                    let ratio = Color::contrast_ratio(shown, get(surface).with_alpha(1.0));
                    assert!(ratio >= 2.95, "{}: {rgb:?} on {surface} is {ratio:.2}", tokens.name);
                }
                let wash = GroupPaint::Custom(rgb).wash(tokens).with_alpha(super::WASH_PEAK).over(card);
                let ratio = Color::contrast_ratio(get("text.primary").over(wash), wash);
                assert!(ratio >= 4.45, "{}: text on a {rgb:?} wash is {ratio:.2}", tokens.name);
            }
        }
    }

    use crate::theme::tokens::{Color, builtins};

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

    /// Text lies on the wash at its strongest (the even fill, or the card's left edge when fading):
    /// over the card's own fill, on the sidebar. The wash is the colour moved only as far as it takes
    /// for the primary text (4.5:1), the second lines (3.5), the amber text (4) and the amber edge
    /// (3) to read, so this holds at any `WASH_PEAK`.
    #[test]
    fn text_on_a_groups_wash_reads_in_every_theme() {
        for tokens in builtins() {
            let get = |name: &str| tokens.get(name).expect(name);
            let column = get("sidebar.background").over(get("surface.window").over(Color::BLACK));
            let card = get("card").over(column);
            for color in GroupColor::ALL {
                let wash = color.wash(tokens).with_alpha(super::WASH_PEAK).over(card);
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

    /// The wash keeps its colour (hue, and some chroma) and only gives up lightness, and a colour
    /// the text already reads on is washed with as it is.
    #[test]
    fn a_wash_is_the_colour_moved_only_as_far_as_the_text_needs() {
        let mut moved = 0;
        for tokens in builtins() {
            for color in GroupColor::ALL.into_iter().filter(|&c| c != GroupColor::Grey) {
                let (mark, wash) = (color.color(tokens), color.wash(tokens));
                if mark == wash {
                    continue;
                }
                moved += 1;
                let ((_, mark_chroma, mark_hue), (_, wash_chroma, wash_hue)) =
                    (mark.to_oklch(), wash.to_oklch());
                let apart = (mark_hue - wash_hue).abs().min(360.0 - (mark_hue - wash_hue).abs());
                assert!(apart < 12.0 && wash_chroma > 0.4 * mark_chroma, "{}: {}", tokens.name, color.key());
            }
        }
        assert!(moved > 0, "some colour on some theme is too bright or dark for its wash");
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
