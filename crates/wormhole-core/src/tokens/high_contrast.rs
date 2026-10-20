//! Increase Contrast: the variant of a theme macOS's accessibility setting asks for.
//! Stronger borders, bands, hovers and selection tints, text lifted on the sidebar (muted
//! and faint, and the names where they sit at the 7:1 floor), and a thicker focus ring; the
//! surfaces turning opaque is the app's part.
//!
//! The variant is derived again from the theme's own recipe on the high-contrast rules
//! ([`high_contrast_rules`]), as [`ThemeTokens::with_primary`] does with a new primary:
//! the theme's own values still win, and what follows a changed token follows it. The cached
//! built-in themes (`builtins`) and the parsed default rules are never touched, so they stay
//! right whichever way the setting is.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use super::catalog::UI_TOKENS;
use super::expr::{Expr, parse_expr};
use super::model::Appearance;
use super::resolve::build_variant;
use super::rules::{default_rules, ui_rule};
use super::theme_tokens::ThemeTokens;

/// The focus ring's least width under Increase Contrast (2 → 3). A theme may draw it
/// thicker, never thinner.
pub const HIGH_CONTRAST_RING: f32 = 3.0;

impl ThemeTokens {
    /// These tokens as Increase Contrast shows them. A theme's own overrides keep their
    /// values (and expressions like `mix(border.strong, primary, 20%)` follow the stronger
    /// token); the rest is derived on the high-contrast rules. Tokens that already are the
    /// variant come back as they are.
    pub fn high_contrast(&self) -> ThemeTokens {
        if self.recipe.high_contrast {
            return self.clone();
        }
        let overrides: BTreeMap<String, (Expr, String)> =
            self.recipe.overrides.iter().map(|(n, e)| (n.clone(), (e.clone(), "theme".into()))).collect();
        let mut metrics = self.metrics.clone();
        metrics.focus_ring_width = metrics.focus_ring_width.max(HIGH_CONTRAST_RING);
        // The columns stay told apart: a theme that draws no divider gets the border's line.
        metrics.divider_width = metrics.divider_width.max(metrics.border_width);
        let rebuilt = build_variant(
            self.name.clone(),
            self.terminal.clone(),
            self.appearance,
            metrics,
            overrides,
            &self.recipe.flags,
            true,
        );
        // Only a theme whose own rules close a loop through a high-contrast rule (a sidebar
        // mixed from `text.primary`, which the variant lifts on the sidebar) can fail here:
        // it keeps its plain colors.
        rebuilt.unwrap_or_else(|e| {
            tracing::warn!("{}: no Increase Contrast variant: {e}", self.name);
            self.clone()
        })
    }
}

/// Increase Contrast: the rules that differ from [`ui_rule`]'s. Every other token
/// keeps its default rule and re-derives from these: the selected texts are judged on the
/// stronger tint, `input.border` follows `border.strong`, `checkbox.border` and
/// `agent.exited` follow `text.faint`.
fn high_contrast_rule(name: &str, dark: bool) -> Option<Cow<'static, str>> {
    let p = |d: &'static str, l: &'static str| if dark { d } else { l };
    // Text is lifted on the sidebar, where it is measured, from where its default
    // rule leaves it, so it keeps every ratio that rule gives it elsewhere.
    let lift = |ratio: &str| {
        ui_rule(name, dark).map(|rule| Cow::Owned(format!("readable({rule}, sidebar.background, {ratio})")))
    };
    Some(Cow::Borrowed(match name {
        // 2.3:1 on Tokyo Night's sidebar. The same alpha of a light theme's dark text shows
        // less on its light sidebar (hence the defaults' 12 % and 14 %): 40 % puts Catppuccin
        // Latte and GitHub Light at 2.1 and 2.4.
        "border" => p("text.primary / 32%", "text.primary / 40%"),
        // 3.5:1 on Tokyo Night's sidebar, and in any theme at least 3:1, the minimum for a
        // control's outline, on every surface a control sits on. Light themes take 58 %, at
        // which Catppuccin Latte needs no push (3.2 on its sidebar, GitHub Light 3.9).
        "border.strong" => p(
            "readable(text.primary / 48%, surface.window, surface.panel, surface.elevated, \
             sidebar.background, 3)",
            "readable(text.primary / 58%, surface.window, surface.panel, surface.elevated, \
             sidebar.background, 3)",
        ),
        // Hover (10 %) now sits above the band (8 %); the steps above it move up to stay
        // steps, hover < band.hover < active, as 5 < 8 < 12 do by default.
        "band" => "text.primary / 8%",
        "card" => "text.primary / 6%",
        "card.edge" => "text.primary / 28%",
        "state.hover" => "text.primary / 10%",
        "band.hover" => "text.primary / 13%",
        "state.active" => "text.primary / 16%",
        "state.selected" => "primary / 24%",
        "sidebar.row.selected" => "mix(sidebar.background, primary, 24%)",
        "list.selected" => "mix(surface.elevated, primary, 24%)",
        // Drawn on the tint (a count badge on the current row), so any alpha shows stronger
        // than it; 28 % over 24 % keeps the default's step (24 % over 14 %): about 21 points
        // more primary. A larger one would take more from the tint's text drawn on it.
        "state.selected.inset" => "primary / 28%",
        // Muted text goes to 7:1, which is where a soft theme's names stop (Catppuccin Latte,
        // Solarized, One Dark and GitHub Dark derive them at the 7:1 floor): the names move
        // up to 10:1 so that muted text stays a step below them.
        "text.primary" => return lift("10"),
        "text.muted" => return lift("7"),
        "text.faint" => return lift("4.5"),
        _ => return None,
    }))
}

/// [`default_rules`] with Increase Contrast's replacements parsed over them, for one
/// appearance. Cached on their own, so the default rules, and the built-in themes derived
/// from them, never see the variant.
pub(super) fn high_contrast_rules(appearance: Appearance) -> &'static BTreeMap<String, Expr> {
    static DARK: LazyLock<BTreeMap<String, Expr>> =
        LazyLock::new(|| parse_high_contrast_rules(Appearance::Dark));
    static LIGHT: LazyLock<BTreeMap<String, Expr>> =
        LazyLock::new(|| parse_high_contrast_rules(Appearance::Light));
    match appearance {
        Appearance::Dark => &DARK,
        Appearance::Light => &LIGHT,
    }
}

fn parse_high_contrast_rules(appearance: Appearance) -> BTreeMap<String, Expr> {
    let mut rules = default_rules(appearance).clone();
    let dark = appearance == Appearance::Dark;
    for t in UI_TOKENS {
        if let Some(src) = high_contrast_rule(t.name, dark) {
            // Static like the default rules, and every entry is exercised by tests.
            let expr = parse_expr(&src).unwrap_or_else(|e| panic!("high-contrast rule for {}: {e}", t.name));
            rules.insert(t.name.to_string(), expr);
        }
    }
    rules
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{HIGH_CONTRAST_RING, high_contrast_rule, high_contrast_rules};
    use crate::theme;
    use crate::tokens::expr::parse_expr;
    use crate::tokens::resolve::MAX_REF_DEPTH;
    use crate::tokens::rules::{default_rule, default_rules};
    use crate::tokens::test_support::{builtin, chain_len, hex, ratio, resolve_str};
    use crate::tokens::{Appearance, ThemeTokens, builds_on_this_thread, builtin_tokens, builtins, derive};

    /// Within `tolerance` steps per 8-bit channel: the expected values were computed in another tool.
    fn near(t: &ThemeTokens, name: &str, want: &str, tolerance: u8) {
        let got = t.get(name).unwrap_or_else(|| panic!("{}: no {name}", t.name));
        let close = got.to_bytes().iter().zip(hex(want).to_bytes()).all(|(a, b)| a.abs_diff(b) <= tolerance);
        assert!(close, "{}: {name} is {}, the design has {want}", t.name, got.to_hex());
    }

    fn alpha(t: &ThemeTokens, name: &str) -> f32 {
        t.get(name).unwrap().a
    }

    /// The table, measured on Tokyo Night's sidebar.
    #[test]
    fn tokyo_night_high_contrast_matches_the_design() {
        let plain = builtin("Tokyo Night");
        let t = plain.high_contrast();
        let get = |name: &str| t.get(name).unwrap();
        // Tokyo Night's names are well above 10:1, so everything is made from the same text.
        let text = get("text.primary");
        assert_eq!(Some(text), plain.get("text.primary"));
        assert_eq!(get("border"), text.with_alpha(0.32));
        assert_eq!(get("border.strong"), text.with_alpha(0.48));
        let border = ratio(&t, "border", "sidebar.background");
        assert!((2.2..2.4).contains(&border), "border at {border:.2}, the design has 2.3");
        let strong = ratio(&t, "border.strong", "sidebar.background");
        assert!((3.4..3.6).contains(&strong), "border.strong at {strong:.2}, the design has 3.5");
        for bg in ["surface.window", "surface.panel", "surface.elevated"] {
            assert!(ratio(&t, "border.strong", bg) >= 3.0, "border.strong on {bg}");
        }
        assert_eq!(get("band"), text.with_alpha(0.08));
        assert_eq!(get("state.hover"), text.with_alpha(0.10));
        assert_eq!(get("sidebar.row.selected"), get("sidebar.background").mix(get("primary"), 0.24));
        assert_eq!(get("state.selected"), get("primary").with_alpha(0.24));
        let name = ratio(&t, "sidebar.row.selected.text", "sidebar.row.selected");
        assert!((4.5..4.7).contains(&name), "the selected name at {name:.2}, the design has 4.6");
        near(&t, "text.muted", "#9aa1c0", 3);
        let muted = ratio(&t, "text.muted", "sidebar.background");
        assert!((7.0..7.2).contains(&muted), "muted at {muted:.2}, the design has 7.1");
        near(&t, "text.faint", "#7a809b", 3);
        assert!(ratio(&t, "text.faint", "sidebar.background") >= 4.5);
        assert_eq!(t.metrics.focus_ring_width, 3.0);
        // The default stays as it was.
        assert_eq!(plain.get("border"), Some(text.with_alpha(0.12)));
        assert_eq!(plain.metrics.focus_ring_width, 2.0);
    }

    /// Every bundled theme, dark and light: what is asked of the variant, with every pair the
    /// contrast check knows still readable.
    #[test]
    fn every_bundled_theme_keeps_its_guarantees() {
        for plain in builtin_tokens() {
            let t = plain.high_contrast();
            let n = &t.name;
            let warnings: Vec<_> =
                t.contrast_warnings().into_iter().filter(|w| !w.0.starts_with("terminal.")).collect();
            assert!(warnings.is_empty(), "{n}: {warnings:?}");
            assert!(ratio(&t, "text.muted", "sidebar.background") >= 7.0, "{n}: muted");
            assert!(ratio(&t, "text.faint", "sidebar.background") >= 4.5, "{n}: faint");
            // Muted text still reads as secondary and faint as quieter still, on the sidebar
            // where they were lifted and on the window.
            for bg in ["sidebar.background", "surface.window"] {
                let (text, muted, faint) =
                    (ratio(&t, "text.primary", bg), ratio(&t, "text.muted", bg), ratio(&t, "text.faint", bg));
                assert!(text > 1.3 * muted, "{n} on {bg}: names {text:.2}, muted {muted:.2}");
                assert!(muted >= faint, "{n} on {bg}: muted {muted:.2}, faint {faint:.2}");
            }
            for bg in ["surface.window", "surface.panel", "sidebar.background", "surface.elevated"] {
                let strong = ratio(&t, "border.strong", bg);
                assert!(strong >= 3.0, "{n}: border.strong on {bg} is {strong:.2}");
                assert!(strong > ratio(&plain, "border.strong", bg), "{n}: border.strong on {bg}");
                assert!(ratio(&t, "border", bg) > ratio(&plain, "border", bg), "{n}: border on {bg}");
            }
            // The overlays keep their order: band < hover < band.hover < active.
            let steps = ["band", "state.hover", "band.hover", "state.active"].map(|s| alpha(&t, s));
            assert!(steps.windows(2).all(|w| w[0] < w[1]), "{n}: {steps:?}");
            assert!(alpha(&t, "state.selected.inset") > alpha(&t, "state.selected"), "{n}");
            assert_eq!(t.metrics.focus_ring_width, HIGH_CONTRAST_RING, "{n}");
            // What it leaves alone stays: the palette, surfaces, primary, status colors, code.
            assert_eq!(t.terminal, plain.terminal, "{n}");
            assert_eq!(t.syntax, plain.syntax, "{n}");
            for same in [
                "surface.window",
                "surface.panel",
                "surface.elevated",
                "sidebar.background",
                "primary",
                "status.success",
                "status.warning",
                "status.danger",
                "status.info",
            ] {
                assert_eq!(t.get(same), plain.get(same), "{n}: {same}");
            }
        }
    }

    /// Light themes get the variant the same way, darker where a dark theme's goes lighter.
    #[test]
    fn light_themes_get_the_variant_too() {
        for name in ["Catppuccin Latte", "GitHub Light"] {
            let plain = builtin(name);
            assert_eq!(plain.appearance, Appearance::Light);
            let t = plain.high_contrast();
            let text = t.get("text.primary").unwrap();
            assert_eq!(t.get("border"), Some(text.with_alpha(0.40)), "{name}");
            assert_eq!(t.get("border.strong"), Some(text.with_alpha(0.58)), "{name}");
            let border = ratio(&t, "border", "sidebar.background");
            assert!((2.0..2.6).contains(&border), "{name}: border at {border:.2}");
            for token in ["text.primary", "text.muted", "text.faint"] {
                let (before, after) = (plain.get(token).unwrap(), t.get(token).unwrap());
                assert!(after.luminance() <= before.luminance(), "{name}: {token} got lighter");
            }
        }
    }

    /// The variant is derived beside the caches, never into them: the built-in themes (one
    /// each, keyed by appearance alone) and the parsed default rules stay what a fresh
    /// derivation gives, however often and in whatever order the variant is asked for, and
    /// the variant of a cached theme is the variant of a fresh one.
    #[test]
    fn the_cached_themes_never_see_the_variant() {
        let pairs: Vec<_> = builtins().iter().zip(theme::builtins()).collect();
        let orders = [pairs.clone(), pairs.iter().rev().copied().collect(), pairs.clone()];
        for order in orders {
            for (cached, palette) in order {
                let variant = cached.high_contrast().high_contrast();
                let _ = cached.with_primary(hex("#ff6600")).high_contrast();
                let fresh = derive(palette);
                assert_eq!(*cached, fresh, "{}", cached.name);
                assert_eq!(variant, fresh.high_contrast(), "{}", cached.name);
                assert_ne!(variant, fresh, "{}", cached.name);
                assert_eq!(cached.metrics.focus_ring_width, 2.0, "{}", cached.name);
            }
        }
    }

    /// "A theme can override any of it": the theme's own values stay, and what it wrote
    /// relative to a token follows that token's high-contrast value.
    #[test]
    fn a_themes_own_values_win() {
        let themed = resolve_str(
            r##"{"extends": "Tokyo Night", "ui": {
                "border": "#123456", "text.muted": "#8a8f9e",
                "scrollbar.thumb": "border.strong", "diff.filler": "mix(band.hover, primary, 20%)"
            }, "metrics": {"focus_ring_width": 4}}"##,
        )
        .unwrap();
        let t = themed.high_contrast();
        assert_eq!(t.get("border"), Some(hex("#123456")));
        assert_eq!(t.get("text.muted"), Some(hex("#8a8f9e")));
        // What follows muted text on a surface the variant leaves alone follows the theme's.
        assert_eq!(t.get("tab.inactive.text"), themed.get("tab.inactive.text"));
        assert_eq!(t.get("scrollbar.thumb"), t.get("border.strong"));
        assert_ne!(t.get("scrollbar.thumb"), themed.get("scrollbar.thumb"));
        let filler = t.get("band.hover").unwrap().mix(t.get("primary").unwrap(), 0.2);
        assert_eq!(t.get("diff.filler"), Some(filler));
        // Not overridden: the variant's.
        assert_eq!(t.get("border.strong"), builtin("Tokyo Night").high_contrast().get("border.strong"));
        // A thicker ring stays; a thinner one is raised to the minimum.
        assert_eq!(t.metrics.focus_ring_width, 4.0);
        let thin = resolve_str(r#"{"metrics": {"focus_ring_width": 1}}"#).unwrap().high_contrast();
        assert_eq!(thin.metrics.focus_ring_width, HIGH_CONTRAST_RING);
        // A value set on the tokens themselves counts as the theme's too.
        let mut set = builtin("Dracula");
        assert!(set.set("border.strong", hex("#ff79c6")));
        assert_eq!(set.high_contrast().get("border.strong"), Some(hex("#ff79c6")));
    }

    #[test]
    fn a_new_primary_keeps_the_variant() {
        let orange = hex("#ff6600");
        for name in ["Tokyo Night", "Catppuccin Latte", "GitHub Light"] {
            let plain = builtin(name);
            let t = plain.high_contrast().with_primary(orange);
            assert_eq!(t, plain.with_primary(orange).high_contrast(), "{name}");
            let row = t.get("sidebar.background").unwrap().mix(orange, 0.24);
            assert_eq!(t.get("sidebar.row.selected"), Some(row), "{name}");
            assert_eq!(t.metrics.focus_ring_width, HIGH_CONTRAST_RING, "{name}");
        }
    }

    /// Asking again changes nothing and derives nothing.
    #[test]
    fn the_variant_of_the_variant_is_itself() {
        let t = builtin("Nord").high_contrast();
        let before = builds_on_this_thread();
        assert_eq!(t.high_contrast(), t);
        assert_eq!(builds_on_this_thread(), before);
    }

    /// A light palette whose yellow is darker than its muted text gets the amber warning
    /// (`rules::dark_yellow_warning`). The variant's muted text is darker still; the choice
    /// is the default rules', so the warning does not turn back into the brown on the switch.
    #[test]
    fn the_warning_keeps_its_hue() {
        let src = r##"{"extends": "Catppuccin Latte", "terminal": {"ansi": {"3": "#7a5c00"}}}"##;
        let plain = resolve_str(src).unwrap();
        let t = plain.high_contrast();
        let yellow = hex("#7a5c00").luminance();
        assert!(yellow < plain.get("text.muted").unwrap().luminance());
        assert!(yellow > t.get("text.muted").unwrap().luminance(), "the case this covers");
        assert_ne!(plain.get("status.warning"), Some(hex("#7a5c00")));
        assert_eq!(t.get("status.warning"), plain.get("status.warning"));
    }

    /// A theme whose own rules only close a loop through a high-contrast rule (its sidebar
    /// mixed from the names, which the variant lifts on the sidebar) keeps its plain colors.
    #[test]
    fn a_theme_the_variant_cannot_derive_stays_as_it_is() {
        let plain =
            resolve_str(r#"{"ui": {"sidebar.background": "mix(surface.panel, text.primary, 5%)"}}"#).unwrap();
        assert_eq!(plain.high_contrast(), plain);
    }

    /// Increase Contrast replaces a few tokens and the steps that must stay above them, and
    /// keeps every other rule; its rules (one level deeper where they lift text) stay far
    /// inside the limits too.
    #[test]
    fn high_contrast_rules_replace_only_their_tokens() {
        let replaced = [
            "band",
            "band.hover",
            "border",
            "border.strong",
            "card",
            "card.edge",
            "list.selected",
            "sidebar.row.selected",
            "state.active",
            "state.hover",
            "state.selected",
            "state.selected.inset",
            "text.faint",
            "text.muted",
            "text.primary",
        ];
        for app in [Appearance::Dark, Appearance::Light] {
            let dark = app == Appearance::Dark;
            let (plain, rules) = (default_rules(app), high_contrast_rules(app));
            assert_eq!(rules.len(), plain.len());
            for (name, expr) in rules {
                let own = high_contrast_rule(name, dark);
                assert_eq!(own.is_some(), replaced.contains(&name.as_str()), "{app:?}: {name}");
                assert_eq!(own.is_some(), expr != &plain[name], "{app:?}: {name}");
                // The defaults are parsed on their own: the variant never changes them.
                if let Some(src) = default_rule(name, app) {
                    assert_eq!(plain[name], parse_expr(&src).unwrap(), "{app:?}: {name}");
                }
                if let Some(src) = own {
                    let deep_enough = format!("{}{src}{}", "(".repeat(24), ")".repeat(24));
                    assert!(parse_expr(&deep_enough).is_ok(), "{name}: rule nests more than 8 deep");
                }
            }
            let mut memo = BTreeMap::new();
            let longest = rules.keys().map(|n| chain_len(n, rules, &mut memo)).max().unwrap_or(0);
            assert!(longest < MAX_REF_DEPTH / 4, "{app:?}: chain of {longest}");
        }
    }

    /// The column dividers follow the border, and Increase Contrast never leaves the columns
    /// without a line: a theme that draws none gets the border's width.
    #[test]
    fn the_dividers_follow_the_border_and_the_variant_keeps_them_drawn() {
        let plain = builtin("Tokyo Night");
        assert_eq!(plain.get("divider"), plain.get("border"));
        assert_eq!(plain.metrics.divider_width, plain.metrics.border_width);
        let high = plain.high_contrast();
        assert_eq!(high.get("divider"), high.get("border"), "stronger with the border");
        assert_ne!(high.get("divider"), plain.get("divider"));
        let hidden = resolve_str(r#"{"metrics": {"divider_width": 0}}"#).unwrap();
        assert_eq!(hidden.metrics.divider_width, 0.0);
        assert_eq!(hidden.high_contrast().metrics.divider_width, hidden.metrics.border_width);
        let thick = resolve_str(r#"{"metrics": {"divider_width": 3}}"#).unwrap();
        assert_eq!(thick.high_contrast().metrics.divider_width, 3.0, "a thicker line stays");
        let own = resolve_str(r##"{"extends": "Tokyo Night", "ui": {"divider": "#123456"}}"##).unwrap();
        assert_eq!(own.high_contrast().get("divider"), Some(hex("#123456")));
    }
}
