//! A project's colour (`project_color` in `.wormhole/config.json`):
//! the tint at the left of the terminal's title row and of its folder's band.
//!
//! The value comes from a repository, so it is read as strictly as a value can be: one of the app's
//! eight colour names (`"teal"`, exactly as written in lower case) or `#rgb` or `#rrggbb`, and
//! nothing else (no alpha, no functions), then moved into a range that reads as a colour. It is
//! only ever drawn as a fill; nothing about it is executed, fetched or trusted, which is why it
//! (and the lock below) stay out of the trust fingerprint.

use crate::folder_color::FolderColor;
use crate::tokens::{Color, ThemeTokens};

/// `config.json`'s key for the project's colour.
pub(super) const KEY: &str = "project_color";
/// `config.json`'s key that locks it: Wormhole's colour menus leave the colour alone while it is
/// `true`, so only editing the file changes it.
pub(super) const LOCK_KEY: &str = "project_color_locked";
/// The top-level keys of `config.json` that only paint or lock the paint: the trust fingerprint
/// leaves them out, so choosing a colour doesn't send a trusted project back for approval.
pub(super) const PAINT_KEYS: [&str; 2] = [KEY, LOCK_KEY];

/// What a hex `project_color` may be at the most: a hex colour is at most seven characters, so
/// anything longer is not read as one.
const MAX_LEN: usize = 7;
/// The OKLab lightness a project's colour is kept within: a black or a white says nothing as a
/// tint (it only darkens or lightens the bar), so they become the nearest colour that does.
const LIGHTNESS: (f32, f32) = (0.30, 0.88);

/// The tint's strength at the bar's left edge, and the share of the bar's width it lasts
/// (36 % at the edge, 20 % at 12 % of the width, gone by 40 %: the stops, as fractions
/// of the strongest).
pub const TINT_STOPS: [(f32, f32); 3] = [(0.0, 1.0), (0.12, 20.0 / 36.0), (0.40, 0.0)];
/// The strongest the tint is drawn: 36 % of the colour over the bar.
pub const TINT_PEAK: f32 = 0.36;
/// How much of [`ProjectColor::tint_peak`] the title row's rounded bar wears across its whole width:
/// an even fill is read everywhere at once, so it is softer than the gradient's start.
pub const BAR_TINT_SHARE: f32 = 0.5;
/// The contrast the bar's main text keeps under the strongest tint, where the bar itself allows it.
const TEXT_CONTRAST: f32 = 5.5;
/// Under Increase Contrast the tint gives way to a line, which keeps this much against the bar.
const LINE_CONTRAST: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectColor(Color);

impl ProjectColor {
    /// `value` as written in the project file, or why it can't be used.
    pub fn parse(value: &str) -> Result<Self, String> {
        let bad = || format!("{:?} is not a colour (write it as #rgb or #rrggbb)", shown(value));
        let digits = value.strip_prefix('#').ok_or_else(bad)?;
        if value.len() > MAX_LEN
            || !matches!(digits.len(), 3 | 6)
            || !digits.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(bad());
        }
        Color::from_hex(value).map(Self::from_color).map_err(|_| bad())
    }

    /// `color` as a tint: opaque and moved into the lightness range that reads as a colour. For
    /// the colours the app derives itself (a folder's, from the theme's palette), which are as
    /// well behaved as a project's after the same treatment.
    pub fn from_color(color: Color) -> Self {
        let lightness = color.to_oklab().0;
        let clamped = lightness.clamp(LIGHTNESS.0, LIGHTNESS.1);
        let color = if clamped == lightness { color } else { color.lighten(clamped - lightness) };
        Self(color.with_alpha(1.0))
    }

    pub fn color(self) -> Color {
        self.0
    }

    /// How strong the tint is at its left edge over `bar` (opaque) with `text` on it:
    /// [`TINT_PEAK`], or less where `text` would otherwise fall below 5.5:1 against the tinted bar
    /// (or 85 % of what the bar alone gives, in a theme whose text has less than that to spare).
    /// The rest of the gradient scales with it.
    pub fn tint_peak(self, bar: Color, text: Color) -> f32 {
        let base = Color::contrast_ratio(text, bar);
        let floor = TEXT_CONTRAST.min(base * 0.85);
        let reads = |alpha: f32| Color::contrast_ratio(text, self.0.with_alpha(alpha).over(bar)) >= floor;
        if reads(TINT_PEAK) {
            return TINT_PEAK;
        }
        // Contrast falls as the tint grows, so the strongest one that reads can be bisected for.
        let (mut ok, mut over) = (0.0, TINT_PEAK);
        for _ in 0..16 {
            let mid = (ok + over) / 2.0;
            if reads(mid) { ok = mid } else { over = mid }
        }
        ok
    }

    /// How strong the title row's rounded bar's even fill is over `bar` with `text` on it: half the
    /// strongest tint that reads, so the text keeps its contrast (5.5:1, never below 4.5:1 where the
    /// theme's own text allows it) everywhere on the bar.
    pub fn bar_tint(self, bar: Color, text: Color) -> f32 {
        self.tint_peak(bar, text) * BAR_TINT_SHARE
    }

    /// The colour of the line that stands in for the tint under Increase Contrast: the project's
    /// colour, moved far enough from `bar` to be seen against it.
    pub fn line(self, bar: Color) -> Color {
        self.0.readable_on(bar, LINE_CONTRAST)
    }
}

/// What `project_color` says: one of the app's colour names, which each theme draws in its own
/// way (so what it looks like is only known where the theme is, see [`Self::resolve`]), or a hex
/// colour, which looks the same everywhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectColorChoice {
    Named(FolderColor),
    Hex(ProjectColor),
}

impl ProjectColorChoice {
    /// `value` as written in the project file, or why it can't be used.
    pub fn parse(value: &str) -> Result<Self, String> {
        if let Some(name) = FolderColor::from_key(value) {
            return Ok(Self::Named(name));
        }
        ProjectColor::parse(value).map(Self::Hex).map_err(|_| {
            format!(
                "{:?} is not a colour (write a name such as \"teal\", or a hex colour as #rgb or #rrggbb)",
                shown(value)
            )
        })
    }

    /// The tint in the theme `tokens` draws: a name as the theme draws it, treated as a hex
    /// colour is.
    pub fn resolve(self, tokens: &ThemeTokens) -> ProjectColor {
        match self {
            Self::Named(name) => ProjectColor::from_color(name.color(tokens)),
            Self::Hex(color) => color,
        }
    }

    /// How it is written in a file: the name, or the hex colour as `#rrggbb`. What
    /// [`Self::parse`] reads back.
    pub fn to_value(self) -> String {
        match self {
            Self::Named(name) => name.key().to_string(),
            Self::Hex(color) => color.color().to_hex(),
        }
    }

    /// The name, for a choice that is one (a hex colour is none of the eight).
    pub fn named(self) -> Option<FolderColor> {
        match self {
            Self::Named(name) => Some(name),
            Self::Hex(_) => None,
        }
    }
}

/// `value` as a short readable text for an error: a long or odd one is cut, and what it holds
/// that a person couldn't see (a control character) is not passed on.
fn shown(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(24).collect()
}

#[cfg(test)]
mod tests {
    use super::{BAR_TINT_SHARE, ProjectColor, TINT_PEAK, TINT_STOPS};
    use crate::tokens::Color;

    fn parse(value: &str) -> Result<ProjectColor, String> {
        ProjectColor::parse(value)
    }

    #[test]
    fn a_colour_is_hex_with_three_or_six_digits() {
        assert_eq!(parse("#2bb3a3").map(|c| c.color().to_hex()), Ok("#2bb3a3".to_string()));
        assert_eq!(parse("#2ba").map(|c| c.color().to_hex()), Ok("#22bbaa".to_string()));
        assert_eq!(parse("#2BB3A3"), parse("#2bb3a3"));
    }

    #[test]
    fn nothing_else_is_a_colour() {
        for value in [
            "",
            "#",
            "2bb3a3",
            "teal",
            "#2bb3a3ff",
            "#2bb3",
            "#2bb3a",
            "#gggggg",
            " #2bb3a3",
            "#2bb3a3 ",
            "#2bb3a3\n",
            "rgb(1, 2, 3)",
            "#2bb3a3; background: url(https://example.com/x)",
            "url(file:///etc/passwd)",
            "#２ｂｂ３ａ３",
            &"#".repeat(10_000),
        ] {
            assert!(parse(value).is_err(), "{value:?} was taken for a colour");
        }
    }

    #[test]
    fn an_error_names_the_value_without_echoing_a_wall_of_text() {
        let err = parse(&format!("#{}\u{7}\u{1b}[2J", "a".repeat(5000))).expect_err("not a colour");
        assert!(err.len() < 120, "{} bytes", err.len());
        assert!(!err.contains('\u{1b}') && !err.contains('\u{7}'));
        assert!(err.contains("#rgb or #rrggbb"));
    }

    use super::{PAINT_KEYS, ProjectColorChoice};
    use crate::folder_color::FolderColor;
    use crate::tokens::builtins;

    /// `project_color` takes the app's own eight names, exactly as written, and a hex as before.
    #[test]
    fn a_choice_is_a_lower_case_name_or_a_hex_colour() {
        for name in FolderColor::ALL {
            assert_eq!(ProjectColorChoice::parse(name.key()), Ok(ProjectColorChoice::Named(name)));
            assert_eq!(ProjectColorChoice::parse(name.key()).map(|c| c.named()), Ok(Some(name)));
        }
        let hex = ProjectColorChoice::parse("#2bb3a3").expect("hex");
        assert_eq!(hex, ProjectColorChoice::Hex(parse("#2bb3a3").expect("hex")));
        assert_eq!(hex.named(), None);
        for value in [
            "",
            "Teal",
            "TEAL",
            " teal",
            "teal ",
            "teal\n",
            "tea",
            "teals",
            "brown",
            "grey",
            "#teal",
            "te\u{0430}l",
            "teal; background: url(https://example.com/x)",
            "rgb(1, 2, 3)",
            "#2bb3a3ff",
            "2bb3a3",
        ] {
            let err = ProjectColorChoice::parse(value).expect_err(value);
            assert!(err.contains("#rgb or #rrggbb") && err.contains("\"teal\""), "{err}");
            assert!(err.len() < 140 && !err.contains('\n'), "{err}");
        }
        assert!(ProjectColorChoice::parse(&"teal".repeat(5000)).is_err());
    }

    /// A name is the folder colour of the theme in force, treated as a hex colour is; a hex colour
    /// looks the same in every theme.
    #[test]
    fn a_name_is_drawn_from_the_theme_and_a_hex_colour_is_not() {
        let themes = builtins();
        let (dark, light) =
            (&themes[0], themes.iter().find(|t| t.name == "Catppuccin Latte").expect("Latte"));
        let teal = ProjectColorChoice::Named(FolderColor::Teal);
        for tokens in themes {
            let want = ProjectColor::from_color(FolderColor::Teal.color(tokens));
            assert_eq!(teal.resolve(tokens), want, "{}", tokens.name);
            assert_eq!(want.color().a, 1.0);
        }
        assert_ne!(teal.resolve(dark).color().to_hex(), teal.resolve(light).color().to_hex());
        let hex = ProjectColorChoice::parse("#2bb3a3").expect("hex");
        assert_eq!(hex.resolve(dark), hex.resolve(light));
        assert_eq!(hex.resolve(dark).color().to_hex(), "#2bb3a3");
    }

    #[test]
    fn the_keys_that_only_paint_are_the_colour_and_its_lock() {
        assert_eq!(PAINT_KEYS, ["project_color", "project_color_locked"]);
    }

    #[test]
    fn black_and_white_become_colours_that_read_as_a_tint() {
        let black = parse("#000000").expect("black").color();
        let white = parse("#ffffff").expect("white").color();
        assert!(black.to_oklab().0 > 0.25 && white.to_oklab().0 < 0.9);
        assert!(black.luminance() < white.luminance());
        // A colour in range is not touched.
        assert_eq!(parse("#2bb3a3").expect("teal").color().to_hex(), "#2bb3a3");
        // And it is always opaque.
        assert_eq!(white.a, 1.0);
    }

    /// A colour the app derives is treated as a project's is: opaque, and kept off black and white.
    #[test]
    fn a_derived_colour_is_made_a_tint_the_same_way() {
        let white = Color::from_hex("#ffffffcc").expect("hex");
        assert_eq!(ProjectColor::from_color(white), parse("#ffffff").expect("white"));
        assert_eq!(ProjectColor::from_color(white).color().a, 1.0);
        let teal = Color::from_hex("#2bb3a3").expect("hex");
        assert_eq!(ProjectColor::from_color(teal), parse("#2bb3a3").expect("teal"));
    }

    /// The stops: 36 % at the edge, 20 % at 12 % of the width, none from 40 %.
    #[test]
    fn the_tint_is_strongest_at_the_left_and_gone_by_forty_percent() {
        let at =
            |share: f32| TINT_STOPS.iter().find(|(x, _)| *x >= share).map_or(0.0, |(_, s)| *s * TINT_PEAK);
        assert_eq!(at(0.0), 0.36);
        assert!((at(0.12) - 0.20).abs() < 1e-6);
        assert_eq!(at(0.40), 0.0);
        assert!(TINT_STOPS.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 > w[1].1), "stops fall");
    }

    #[test]
    fn the_tint_keeps_the_text_legible_on_the_bar() {
        let bar = Color::from_hex("#1a1b26").expect("bar");
        let text = Color::from_hex("#c0caf5").expect("text");
        let teal = parse("#2bb3a3").expect("teal");
        // 36 % measures 5.5:1; the tint gives up a hair of it to keep that.
        let peak = teal.tint_peak(bar, text);
        assert!(peak > 0.35 && peak <= TINT_PEAK, "{peak}");
        let under = teal.color().with_alpha(peak).over(bar);
        assert!(Color::contrast_ratio(text, under) >= 5.5 - 0.01);
    }

    #[test]
    fn a_tint_that_would_swamp_the_text_is_weakened_until_it_reads() {
        let bar = Color::from_hex("#1a1b26").expect("bar");
        let text = Color::from_hex("#c0caf5").expect("text");
        // A colour as light as the text itself.
        let pale = parse("#e0e8ff").expect("pale");
        let peak = pale.tint_peak(bar, text);
        assert!(peak > 0.0 && peak < TINT_PEAK, "{peak}");
        let under = pale.color().with_alpha(peak).over(bar);
        assert!(Color::contrast_ratio(text, under) >= 5.5 - 0.05, "{}", Color::contrast_ratio(text, under));
        // A theme whose text has little to spare keeps most of what it has.
        let dull = Color::from_hex("#7b82a0").expect("dull");
        let base = Color::contrast_ratio(dull, bar);
        let peak = pale.tint_peak(bar, dull);
        let under = pale.color().with_alpha(peak).over(bar);
        assert!(Color::contrast_ratio(dull, under) >= base * 0.85 - 0.05);
    }

    #[test]
    fn the_bars_even_tint_reads_in_dark_and_light_themes() {
        for (bar, text) in
            [("#1a1b26", "#c0caf5"), ("#ffffff", "#1f2335"), ("#fdf6e3", "#586e75"), ("#000000", "#ffffff")]
        {
            let (bar, text) = (Color::from_hex(bar).expect("bar"), Color::from_hex(text).expect("text"));
            let base = Color::contrast_ratio(text, bar);
            for hex in ["#2bb3a3", "#e0e8ff", "#d94f4f", "#f0e68c", "#3b5bdb", "#1f4f4a", "#ff00ff"] {
                let tint = parse(hex).expect("colour");
                let level = tint.bar_tint(bar, text);
                assert!(level <= TINT_PEAK * BAR_TINT_SHARE + 1e-6);
                let under = tint.color().with_alpha(level).over(bar);
                let ratio = Color::contrast_ratio(text, under);
                assert!(ratio >= 4.5_f32.min(base * 0.85) - 0.05, "{hex} on {bar:?}: {ratio}");
            }
        }
    }

    #[test]
    fn the_line_under_increase_contrast_is_seen_against_the_bar() {
        let bar = Color::from_hex("#1a1b26").expect("bar");
        let dark_teal = parse("#1f4f4a").expect("dark teal");
        let line = dark_teal.line(bar);
        assert!(Color::contrast_ratio(line, bar) >= 3.0 - 0.05);
        let light = Color::from_hex("#ffffff").expect("light bar");
        assert!(Color::contrast_ratio(parse("#f0e68c").expect("khaki").line(light), light) >= 3.0 - 0.05);
    }
}
