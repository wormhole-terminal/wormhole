//! Text on a translucent window. A window paints its own surfaces (the
//! window, the sidebar, the right panel) at the window opacity over the system's blur
//! material, so whatever is behind the window shows through them. The rules judge text on
//! the opaque surfaces; here the text the design names (muted, faint, the branch and the
//! selected row's, and the names above them) is judged again on the worst case at the
//! current opacity: a white window behind a dark theme, a black one behind a light theme.
//!
//! This runs on finished tokens, once per opacity, outside derivation. The cached built-in
//! themes (`builtins`) and parsed rules (`rules::default_rules`) are keyed by appearance
//! alone, and stay right whatever the opacity, the blur or Reduce Transparency say.

use super::model::Appearance;
use super::resolve::READABLE_MARGIN;
use super::theme_tokens::ThemeTokens;
use crate::color::Color;

/// At or above this opacity a window counts as opaque, as the blur does.
pub const OPAQUE: f32 = 0.999;

/// Without the blur, the desktop shows sharp through every surface: below this
/// opacity no text colour is safe on it, and Settings says so.
pub const BLUR_OFF_MIN_OPACITY: f32 = 0.95;

/// The HUD material over the desktop as the design models it: #1c1c1e at 45 % in a dark
/// appearance, #f4f4f6 at 45 % in a light one. An estimate, not measured; the material
/// follows the theme's appearance (`vibrancy.rs` sets it).
const MATERIAL_DARK: Color = Color::rgba(28.0 / 255.0, 28.0 / 255.0, 30.0 / 255.0, 0.45);
const MATERIAL_LIGHT: Color = Color::rgba(244.0 / 255.0, 244.0 / 255.0, 246.0 / 255.0, 0.45);

/// How a window's own surfaces let the desktop through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Translucency {
    /// The surfaces' opacity; [`OPAQUE`] and above is opaque.
    pub opacity: f32,
    /// Whether the system's blur material lies between the surfaces and the desktop.
    pub blur: bool,
}

impl Translucency {
    /// Whether the desktop shows through at all.
    pub fn is_translucent(self) -> bool {
        self.opacity < OPAQUE
    }

    /// Translucent without the blur, below the opacity text can be judged at: the desktop
    /// shows sharp behind the text.
    pub fn unsafe_without_blur(self) -> bool {
        !self.blur && self.opacity < BLUR_OFF_MIN_OPACITY
    }
}

/// What a translucent window draws in place of a few tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TranslucentColors {
    /// `text.primary`, `text.muted`, `text.faint` and `git.branch`, readable on the surfaces
    /// and on the bands, hovered rows and branch chips over them.
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub branch: Color,
    /// `sidebar.row.selected.text` / `.muted`, readable on the sidebar's selected row and on a
    /// selected right-panel row: `state.selected` over the translucent panel, the composite
    /// the design measures as "text.muted on primary 14 %". The sidebar's row itself stays opaque.
    /// GPUI adds the alphas of fills drawn on one another (Metal's destination alpha factor is
    /// one), so a 14 % tint over the 80 % sidebar would cover 94 % of the desktop anyway and
    /// show darker than the design in a light theme; the opaque row shows what it draws.
    pub selected_text: Color,
    pub selected_muted: Color,
}

impl ThemeTokens {
    /// The colours a window at `look` draws in place of the tokens', or `None` when it is
    /// opaque and the tokens stand as they are.
    ///
    /// Each colour moves only away from the surface (lighter in a dark theme, darker in a
    /// light one) and only as far as its ratio takes it: 4.5:1 for text, 3:1 for faint
    /// glyphs, as the rules ask on opaque surfaces and the design lists per pair. The names are
    /// judged too (the design's "Terminal name · 4.5"): a theme whose names barely pass
    /// opaque would otherwise end up quieter than its muted text. Where no colour reaches
    /// the ratio (a low opacity over the wrong desktop), text stops where the text above it
    /// in rank stopped rather than turning into one more white line.
    pub fn translucent(&self, look: Translucency) -> Option<TranslucentColors> {
        if !look.is_translucent() {
            return None;
        }
        let get = |name: &str| self.get(name);
        let dark = self.appearance == Appearance::Dark;
        let (desktop, material) =
            if dark { (Color::WHITE, MATERIAL_DARK) } else { (Color::BLACK, MATERIAL_LIGHT) };
        let behind = if look.blur { material.over(desktop) } else { desktop };
        let seen = |surface: Color| surface.with_alpha(surface.a * look.opacity).over(behind);
        let opaque_window = get("surface.window")?.over(Color::BLACK);

        let window = seen(get("surface.window")?);
        let sidebar = seen(get("sidebar.background")?);
        let panel = seen(get("surface.panel")?);
        let (band, hover, chip) = (get("band")?, get("state.hover")?, get("git.branch.background")?);
        let banded = [window, sidebar, panel, band.over(sidebar), band.over(panel)];
        let hovered = [&banded[..], &[hover.over(sidebar), hover.over(panel)]].concat();
        let chips = [window, sidebar, panel, chip.over(band.over(sidebar)), chip.over(band.over(panel))];

        let selected_row = get("sidebar.row.selected")?.over(get("sidebar.background")?.over(opaque_window));
        let selected = [selected_row, get("state.selected")?.over(panel)];

        let lift = |token: &str, ratio: f32, fallback: Option<Color>, backgrounds: &[Color]| {
            let way = Way { lighter: dark, fallback: fallback.map(|c| c.to_oklab().0) };
            self.get(token)
                .map(|color| backgrounds.iter().fold(color, |color, bg| toward(color, *bg, ratio, way)))
        };
        let text = lift("text.primary", 4.5, None, &hovered)?;
        let muted = lift("text.muted", 4.5, Some(text), &banded)?;
        let selected_text = lift("sidebar.row.selected.text", 4.5, None, &selected)?;
        Some(TranslucentColors {
            text,
            muted,
            faint: lift("text.faint", 3.0, Some(muted), &hovered)?,
            branch: lift("git.branch", 4.5, Some(text), &chips)?,
            selected_text,
            selected_muted: lift("sidebar.row.selected.muted", 4.5, Some(selected_text), &selected)?,
        })
    }
}

/// Which way text moves, and where it stops when nothing reads.
#[derive(Clone, Copy)]
struct Way {
    /// Lighter (a dark theme's text) or darker (a light theme's).
    lighter: bool,
    /// The OKLab lightness to stop at when no colour that way reaches the ratio; `None`
    /// goes all the way (white or black).
    fallback: Option<f32>,
}

/// `color` moved along OKLab lightness the given way just far enough to reach `ratio` on
/// the opaque `bg`. Unlike `readable()`, the way is fixed: on a mid-grey composite
/// `readable()` would turn a dark theme's muted text dark.
fn toward(color: Color, bg: Color, ratio: f32, way: Way) -> Color {
    let ratio = ratio + READABLE_MARGIN;
    let reads = |c: Color| Color::contrast_ratio(c.over(bg), bg) >= ratio;
    let from = color.to_oklab().0;
    let span = if way.lighter { 1.0 - from } else { -from };
    if reads(color) || span.abs() < f32::EPSILON {
        return color;
    }
    let at = |t: f32| color.lighten(span * t);
    if !reads(at(1.0)) {
        return match way.fallback {
            // Only ever onward: a fallback behind the colour leaves it where it is.
            Some(stop) if (stop - from) * span > 0.0 => color.lighten(stop - from),
            Some(_) => color,
            None => at(1.0),
        };
    }
    // Contrast falls until the text crosses the background's luminance and rises after it,
    // so "reads" is false and then true along the way: bisect for the first true.
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if reads(at(mid)) { hi = mid } else { lo = mid }
    }
    at(hi)
}

#[cfg(test)]
mod tests {
    use super::{MATERIAL_DARK, MATERIAL_LIGHT, Translucency};
    use crate::theme;
    use crate::tokens::test_support::{builtin, hex};
    use crate::tokens::{Appearance, Color, builtin_tokens, builtins, derive};

    /// Equal within one step per 8-bit channel: the expected values were computed in another tool.
    fn near(got: Color, want: &str, what: &str) {
        let close = got.to_bytes().iter().zip(hex(want).to_bytes()).all(|(a, b)| a.abs_diff(b) <= 1);
        assert!(close, "{what} is {}, the design has {want}", got.to_hex());
    }

    const DESIGN: Translucency = Translucency { opacity: 0.8, blur: true };

    /// Tokyo Night at 80 % with blur over a white window.
    #[test]
    fn tokyo_night_at_80_percent_matches_the_design() {
        let t = builtin("Tokyo Night");
        let colors = t.translucent(DESIGN).expect("translucent");
        // The design's "muted" is the selected row's second line (#8990b0 on primary 14 %).
        near(colors.selected_muted, "#a3abcc", "selected muted");
        near(colors.faint, "#7c819b", "faint");
        near(colors.branch, "#c0a9f7", "branch");
        // The names read at 8.11:1 there and stay as they are; muted text gets lighter than
        // the opaque window's and stays quieter than them.
        assert_eq!(Some(colors.text), t.get("text.primary"));
        let muted = colors.muted.luminance();
        assert!(muted > t.get("text.muted").unwrap().luminance());
        assert!(muted < colors.text.luminance());
    }

    /// Catppuccin Latte at 80 % with blur over a black window: muted text on the
    /// selection tint is 3.66:1 before.
    #[test]
    fn latte_muted_on_the_tint_at_80_percent_matches_the_design() {
        let t = builtin("Catppuccin Latte");
        let colors = t.translucent(DESIGN).expect("translucent");
        near(colors.selected_muted, "#4b4d5f", "selected muted");
        // Darker, never lighter, in a light theme.
        assert!(colors.muted.luminance() < t.get("text.muted").unwrap().luminance());
    }

    /// The pass runs after derivation, so the cached built-in themes (one each, keyed by
    /// appearance alone) never see the window: whatever opacity and blur a window asks for,
    /// in whatever order, the cache still holds what a fresh derivation gives, and the pass
    /// on it gives what the pass on a fresh derivation does.
    #[test]
    fn the_cached_themes_never_see_the_window() {
        let looks = [(0.8, true), (1.0, true), (0.8, false), (0.3, true), (1.0, false), (0.8, true)];
        for (opacity, blur) in looks {
            let look = Translucency { opacity, blur };
            for (cached, palette) in builtins().iter().zip(theme::builtins()) {
                let colors = cached.translucent(look);
                let fresh = derive(palette);
                assert_eq!(*cached, fresh, "{}", cached.name);
                assert_eq!(colors, fresh.translucent(look), "{} at {opacity}, blur {blur}", cached.name);
            }
        }
    }

    #[test]
    fn an_opaque_window_keeps_the_tokens() {
        let t = builtin("Tokyo Night");
        assert_eq!(t.translucent(Translucency { opacity: 1.0, blur: true }), None);
        assert_eq!(t.translucent(Translucency { opacity: 0.9995, blur: false }), None);
    }

    /// At a low opacity over the wrong desktop nothing reads, and `readable()` would flip a
    /// dark theme's text to dark on the mid-grey it sees. Text keeps its direction, and its
    /// rank where reading allows: faint no louder than muted, muted no louder than the names.
    #[test]
    fn text_keeps_its_direction_and_rank_at_any_opacity() {
        for t in builtin_tokens() {
            let n = &t.name;
            let dark = t.appearance == Appearance::Dark;
            // How loud: lightness away from the surface.
            let loud = |c: Color| if dark { c.to_oklab().0 } else { -c.to_oklab().0 };
            for opacity in [0.3, 0.5, 0.65, 0.8, 0.95] {
                for blur in [true, false] {
                    let c = t.translucent(Translucency { opacity, blur }).expect("translucent");
                    for (name, got) in [
                        ("text.primary", c.text),
                        ("text.muted", c.muted),
                        ("text.faint", c.faint),
                        ("git.branch", c.branch),
                        ("sidebar.row.selected.text", c.selected_text),
                        ("sidebar.row.selected.muted", c.selected_muted),
                    ] {
                        let token = t.get(name).unwrap();
                        assert!(
                            loud(got) >= loud(token) - 1e-4,
                            "{n} {opacity} {blur}: {name} moved the wrong way"
                        );
                    }
                    let at = format!("{n} at {opacity}, blur {blur}");
                    assert!(loud(c.muted) <= loud(c.text) + 0.02, "{at}: muted louder than the names");
                    assert!(loud(c.faint) <= loud(c.muted) + 0.02, "{at}: faint louder than muted");
                }
            }
        }
    }

    /// With the blur on at the default opacity, every bundled theme's adjusted text reaches
    /// its ratio on the worst case it is drawn on.
    #[test]
    fn every_theme_reads_on_the_worst_case_at_the_default_opacity() {
        for t in builtin_tokens() {
            let n = &t.name;
            let c = t.translucent(DESIGN).expect("translucent");
            let dark = t.appearance == Appearance::Dark;
            let behind =
                if dark { MATERIAL_DARK.over(Color::WHITE) } else { MATERIAL_LIGHT.over(Color::BLACK) };
            let sidebar = t.get("sidebar.background").unwrap().with_alpha(0.8).over(behind);
            let band = t.get("band").unwrap().over(sidebar);
            let tint = t
                .get("state.selected")
                .unwrap()
                .over(t.get("surface.panel").unwrap().with_alpha(0.8).over(behind));
            for (what, fg, bg, min) in [
                ("names on a band", c.text, band, 4.5),
                ("muted on a band", c.muted, band, 4.5),
                ("faint on a band", c.faint, band, 3.0),
                ("branch on its chip", c.branch, t.get("git.branch.background").unwrap().over(band), 4.5),
                ("selected muted", c.selected_muted, tint, 4.5),
                ("selected name", c.selected_text, tint, 4.5),
            ] {
                let r = Color::contrast_ratio(fg, bg);
                assert!(r >= min, "{n}: {what} is {r:.2}");
            }
        }
    }

    #[test]
    fn blur_off_is_unsafe_below_95_percent() {
        assert!(Translucency { opacity: 0.8, blur: false }.unsafe_without_blur());
        assert!(!Translucency { opacity: 0.95, blur: false }.unsafe_without_blur());
        assert!(!Translucency { opacity: 0.8, blur: true }.unsafe_without_blur());
    }
}
