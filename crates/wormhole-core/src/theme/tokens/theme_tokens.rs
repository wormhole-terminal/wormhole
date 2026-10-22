//! [`ThemeTokens`]: a resolved theme, and how it changes after resolution
//! (single values, a different primary).

use std::collections::BTreeMap;

use super::catalog::UI_TOKENS;
use super::expr::Expr;
use super::model::{Appearance, Metrics, SyntaxStyle, TerminalColors};
use super::resolve::{FlagOverrides, build_variant};
use crate::theme;
use crate::theme::tokens::color::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeTokens {
    pub name: String,
    pub appearance: Appearance,
    pub terminal: TerminalColors,
    pub(super) ui: BTreeMap<String, Color>,
    pub syntax: BTreeMap<String, SyntaxStyle>,
    pub metrics: Metrics,
    pub(super) recipe: Recipe,
}

/// What a theme overrides, as the expressions it was written with. Kept next
/// to the resolved colors so [`ThemeTokens::with_primary`] can re-derive
/// everything exactly, including overrides like `primary / 15%` that hex values
/// could not follow. Ignored by equality: the same colors are the same theme.
#[derive(Clone, Debug, Default)]
pub(super) struct Recipe {
    pub(super) overrides: BTreeMap<String, Expr>,
    pub(super) flags: FlagOverrides,
    /// Derived on the Increase Contrast rules ([`ThemeTokens::high_contrast`]), so a new
    /// primary keeps the variant.
    pub(super) high_contrast: bool,
}

impl PartialEq for Recipe {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl ThemeTokens {
    /// Any node: a UI token (`primary`), `terminal.*` or `syntax.*`.
    pub fn get(&self, name: &str) -> Option<Color> {
        if let Some(key) = name.strip_prefix("terminal.") {
            return self.terminal.get(key);
        }
        if let Some(key) = name.strip_prefix("syntax.") {
            return self.syntax.get(key).map(|s| s.color);
        }
        self.ui.get(name).copied()
    }

    /// Set a node's final value (no re-derivation of dependents; edit a
    /// [`ThemeFile`](super::ThemeFile) and [`resolve`](super::resolve) for that,
    /// though [`Self::with_primary`] keeps the value). Returns false, changing
    /// nothing, for names that are not UI tokens, terminal colors or syntax
    /// keys (new syntax keys are accepted).
    pub fn set(&mut self, name: &str, color: Color) -> bool {
        if let Some(key) = name.strip_prefix("terminal.") {
            return self.terminal.set(key, color);
        }
        if let Some(key) = name.strip_prefix("syntax.") {
            let style = self.syntax.entry(key.to_string()).or_insert(SyntaxStyle {
                color,
                bold: false,
                italic: false,
            });
            style.color = color;
            self.recipe.overrides.insert(name.to_string(), Expr::Lit(color));
            return true;
        }
        if !UI_TOKENS.iter().any(|t| t.name == name) {
            return false;
        }
        self.ui.insert(name.to_string(), color);
        self.recipe.overrides.insert(name.to_string(), Expr::Lit(color));
        true
    }

    /// These tokens with `primary` replaced, and everything derived from it
    /// (focus ring, primary buttons, selection, links, the working status, …)
    /// re-derived. The theme's own overrides still apply, expressions like
    /// `primary / 15%` included, so only what followed the primary moves.
    pub fn with_primary(&self, primary: Color) -> ThemeTokens {
        let mut overrides: BTreeMap<String, (Expr, String)> =
            self.recipe.overrides.iter().map(|(n, e)| (n.clone(), (e.clone(), "theme".into()))).collect();
        let primary = Expr::Lit(primary.with_alpha(1.0));
        overrides.insert("primary".into(), (primary, "primary color".into()));
        let rebuilt = build_variant(
            self.name.clone(),
            self.terminal.clone(),
            self.appearance,
            self.metrics.clone(),
            overrides,
            &self.recipe.flags,
            self.recipe.high_contrast,
        );
        // Removing a rule cannot introduce a cycle, so this is unreachable in
        // practice; keep the current colors rather than panic.
        rebuilt.unwrap_or_else(|_| self.clone())
    }

    /// All UI tokens, sorted by name.
    pub fn ui_tokens(&self) -> impl Iterator<Item = (&str, Color)> {
        self.ui.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// The terminal palette in the renderer's format.
    pub fn to_theme(&self) -> theme::Theme {
        self.terminal.to_theme(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::tokens::test_support::{builtin, hex, ratio, resolve_str, snapshot};
    use crate::theme::tokens::{UI_TOKENS, builtin_tokens};

    #[test]
    fn with_primary_rederives_what_follows_the_primary() {
        let base = builtin("Tokyo Night");
        let orange = hex("#ff6600");
        let t = base.with_primary(orange);
        assert_eq!(t.get("primary"), Some(orange));
        for follower in ["border.focus", "checkbox.checked", "agent.working"] {
            assert_eq!(t.get(follower), Some(orange), "{follower}");
        }
        assert_eq!(t.get("text.on_primary"), Some(orange.text_on()));
        assert_ne!(t.get("primary.hover"), base.get("primary.hover"));
        // Neither the palette nor the surfaces move.
        assert_eq!(t.terminal, base.terminal);
        assert_eq!(t.get("surface.window"), base.get("surface.window"));
        assert_eq!(t.get("text.primary"), base.get("text.primary"));
    }

    /// Selection is a tint of the primary (opaque where the surface is known), the
    /// choice controls stay solid, and what sits on each stays readable whatever primary
    /// is chosen: every bundled theme's own, and picks that are hard to put text on (pale
    /// yellow and cyan, dark navy, mid grey, a green).
    #[test]
    fn selection_is_a_tint_of_the_primary_with_readable_content() {
        let picks = ["#ffff00", "#fff3a0", "#9ff4ff", "#0b1d4d", "#777777", "#808080", "#3fb950", "#9ece6a"];
        for base in builtin_tokens() {
            let themes = std::iter::once(base.clone()).chain(picks.iter().map(|p| base.with_primary(hex(p))));
            for t in themes {
                let n = format!("{} / {}", t.name, t.get("primary").unwrap().to_hex());
                let get = |name: &str| t.get(name).unwrap();
                let primary = get("primary");
                assert_eq!(get("state.selected"), primary.with_alpha(0.14), "{n}");
                assert_eq!(get("state.selected.inset"), primary.with_alpha(0.24), "{n}");
                assert_eq!(get("sidebar.row.selected"), get("sidebar.background").mix(primary, 0.14), "{n}");
                assert_eq!(get("list.selected"), get("surface.elevated").mix(primary, 0.14), "{n}");
                assert_eq!(get("sidebar.row.selected").a, 1.0, "{n}: the row tint is opaque");
                assert_eq!(get("chip.selected"), primary, "{n}: choice controls stay solid");
                let tinted = [
                    ("sidebar.row.selected.text", "sidebar.row.selected"),
                    ("sidebar.row.selected.muted", "sidebar.row.selected"),
                    ("list.selected.text", "list.selected"),
                    ("list.selected.muted", "list.selected"),
                    ("text.on_selected", "chip.selected"),
                    ("text.on_selected.muted", "chip.selected"),
                ];
                for (fg, bg) in tinted {
                    assert!(ratio(&t, fg, bg) >= 4.5, "{n}: {fg} on {bg} is {:.2}", ratio(&t, fg, bg));
                }
                let on_fill: Vec<_> = t
                    .contrast_warnings()
                    .into_iter()
                    .filter(|w| {
                        w.0.starts_with("text.on_") || w.0.contains(".selected") || w.1.contains(".selected")
                    })
                    .collect();
                assert!(on_fill.is_empty(), "{n}: {on_fill:?}");
                // Hover stays a neutral overlay, so hovering never looks like selecting.
                assert_ne!(get("state.hover").with_alpha(1.0), primary, "{n}");
                assert!(get("state.hover").a < 0.2, "{n}");
                // Drop targets over the terminal stay a tint.
                assert!(get("state.highlight").a < 0.5, "{n}");
            }
        }
    }

    /// The active title tab is a neutral raised fill, not the window colour it sits on:
    /// the fill has to show against the title row, in every theme and under any primary.
    #[test]
    fn the_active_tab_is_the_elevated_surface() {
        for base in builtin_tokens() {
            let n = base.name.clone();
            assert_eq!(base.get("tab.active.background"), base.get("surface.elevated"), "{n}");
            let t = base.with_primary(hex("#ff6600"));
            assert_eq!(t.get("tab.active.background"), t.get("surface.elevated"), "{n}");
        }
    }

    /// The text on a selection is judged on the fill, so a theme that makes the tint
    /// stronger keeps every pair readable without touching the text tokens.
    #[test]
    fn a_theme_can_make_the_selection_tint_stronger() {
        for base in ["Tokyo Night", "GitHub Light"] {
            let src = format!(
                r##"{{"extends": "{base}", "ui": {{"state.selected": "primary / 32%",
                    "sidebar.row.selected": "mix(sidebar.background, primary, 32%)",
                    "list.selected": "mix(surface.elevated, primary, 32%)"}}}}"##
            );
            let t = resolve_str(&src).unwrap();
            let stronger = t.get("sidebar.row.selected").unwrap();
            assert_ne!(Some(stronger), builtin(base).get("sidebar.row.selected"));
            let warnings: Vec<_> =
                t.contrast_warnings().into_iter().filter(|w| !w.0.starts_with("terminal.")).collect();
            assert!(warnings.is_empty(), "{base}: {warnings:?}");
        }
    }

    #[test]
    fn with_primary_ignores_the_alpha_of_the_color() {
        let t = builtin("Nord").with_primary(hex("#ff660080"));
        assert_eq!(t.get("primary"), Some(hex("#ff6600")));
    }

    #[test]
    fn with_primary_equals_a_theme_file_that_sets_the_primary() {
        for base in builtin_tokens() {
            let via_file = resolve_str(&format!(
                r##"{{"name": {:?}, "extends": {:?}, "ui": {{"primary": "#12ab89"}}}}"##,
                base.name, base.name
            ))
            .unwrap();
            assert_eq!(snapshot(&base.with_primary(hex("#12ab89"))), snapshot(&via_file), "{}", base.name);
        }
    }

    #[test]
    fn with_primary_keeps_overrides_and_follows_primary_expressions() {
        let file = r##"{
            "ui": {"primary": "#00ff00", "sidebar.row.selected": "primary / 15%", "border": "#123456"},
            "syntax": {"keyword": {"bold": true}}
        }"##;
        let themed = resolve_str(file).unwrap();
        let red = hex("#ff0000");
        let t = themed.with_primary(red);
        // The theme's own primary gives way, and what was written relative to
        // it follows.
        assert_eq!(t.get("primary"), Some(red));
        assert_eq!(t.get("sidebar.row.selected"), Some(red.with_alpha(0.15)));
        // What the theme pinned stays pinned.
        assert_eq!(t.get("border"), Some(hex("#123456")));
        assert!(t.syntax["keyword"].bold);
        assert_eq!(snapshot(&t.with_primary(hex("#00ff00"))), snapshot(&themed));
    }

    #[test]
    fn with_primary_keeps_values_changed_with_set() {
        let mut base = builtin("Dracula");
        let pink = hex("#ff79c6");
        assert!(base.set("border", pink));
        let t = base.with_primary(hex("#ff0000"));
        assert_eq!(t.get("border"), Some(pink));
        assert_eq!(t.get("primary"), Some(hex("#ff0000")));
    }

    #[test]
    fn get_set_and_iteration() {
        let mut t = builtin("Dracula");
        let red = hex("#ff0000");
        assert!(t.set("primary", red));
        assert_eq!(t.get("primary"), Some(red));
        assert!(t.set("terminal.ansi.3", red));
        assert_eq!(t.terminal.ansi[3], red);
        assert!(t.set("terminal.cursor_text", red));
        assert_eq!(t.terminal.cursor_text, Some(red));
        assert!(t.set("syntax.keyword", red));
        assert_eq!(t.syntax["keyword"].color, red);
        assert!(!t.set("not.a.token", red));
        assert!(!t.set("terminal.ansi.16", red));
        assert_eq!(t.get("not.a.token"), None);
        let names: Vec<_> = t.ui_tokens().map(|(n, _)| n).collect();
        assert_eq!(names.len(), UI_TOKENS.len());
        assert!(names.windows(2).all(|w| w[0] < w[1]));
        let theme = t.to_theme();
        assert_eq!(theme.palette[3], 0xff0000);
        assert_eq!(theme.name, "Dracula");
    }
}
