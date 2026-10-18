//! Default derivation rules: how every UI token, syntax color and derived
//! metric follows from the terminal palette, written in the expression
//! language theme files use.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use super::catalog::UI_TOKENS;
use super::expr::{Expr, parse_expr};
use super::model::Appearance;

/// Metrics whose default follows another metric, as `(key, source)`.
const METRIC_RULES: &[(&str, &str)] = &[("radius_chip", "radius_sm")];

/// Default rule for a UI token, `syntax.*` or `metrics.*` node, in expression
/// syntax. The Studio can show this as "derived from ..." next to each token.
pub fn default_rule(name: &str, appearance: Appearance) -> Option<String> {
    if let Some(key) = name.strip_prefix("syntax.") {
        return SYNTAX_RULES.iter().find(|r| r.0 == key).map(|r| r.1.to_string());
    }
    if let Some(key) = name.strip_prefix("metrics.") {
        return METRIC_RULES.iter().find(|r| r.0 == key).map(|r| r.1.to_string());
    }
    ui_rule(name, appearance == Appearance::Dark).map(str::to_string)
}

/// Default derivation rules. Dark and light differ mostly in how far overlays
/// go (light UIs need less alpha to read) and in which way panels move.
pub(super) fn ui_rule(name: &str, dark: bool) -> Option<&'static str> {
    let p = |d: &'static str, l: &'static str| if dark { d } else { l };
    Some(match name {
        "surface.window" => "terminal.background",
        // Sidebars sit a little darker than the terminal, as before tokens.
        "surface.panel" => p("mix(surface.window, #000, 18%)", "mix(surface.window, #000, 4%)"),
        "surface.elevated" => {
            p("mix(surface.window, terminal.foreground, 7%)", "mix(surface.window, #fff, 60%)")
        }
        "surface.overlay" => p("#000 / 50%", "#000 / 25%"),
        "titlebar.background" => "surface.panel",
        "sidebar.background" => "surface.panel",
        "sidebar.row.hover" => "state.hover",
        // Opaque, so its contrast is exact and a translucent window doesn't blend it twice.
        "sidebar.row.selected" => "mix(sidebar.background, primary, 14%)",
        // The primary itself names the current row (darker or lighter only as far as
        // the tint needs); muted text gets its own variant rather than moving every
        // muted line for one row's fill.
        "sidebar.row.selected.text" => "readable(primary, sidebar.row.selected, 4.5)",
        "sidebar.row.selected.muted" => "readable(text.muted, sidebar.row.selected, 4.5)",
        // The sidebar's cards: a faint fill and an edge that holds the card where the fill vanishes
        // (on white, say): a light theme's fill is 3% and its edge 12%, as the board's GitHub Light.
        "card" => p("text.primary / 3.5%", "text.primary / 3%"),
        "card.edge" => p("text.primary / 10%", "text.primary / 12%"),
        // Group heads and section bands, one step above a hovered row and below a pressed one.
        "band" => "text.primary / 5%",
        "band.hover" => "text.primary / 8%",
        "band.solid" => "over(band, sidebar.background)",
        // UI text gets 7:1 (AAA) even when the terminal foreground is softer:
        // UI labels are smaller and thinner than terminal text.
        "text.primary" => {
            "readable(readable(terminal.foreground, surface.window, surface.panel, 7), surface.elevated, 4.5)"
        }
        "text.muted" => {
            "readable(mix(text.primary, surface.window, 35%), surface.window, surface.panel, surface.elevated, \
             band.solid, 4.5)"
        }
        // Also on a hovered row: the idle status ring sits there.
        "text.faint" => {
            "readable(mix(text.primary, surface.window, 55%), surface.window, surface.panel, band.solid, \
             over(state.hover, surface.panel), 3)"
        }
        "text.disabled" => "mix(text.faint, surface.window, 30%)",
        "text.on_primary" => "text_on(primary)",
        // For the solid fills (a chosen segment, a primary button); tints carry their
        // own `.text` / `.muted`. Starts from the primary's text so it is black or white
        // on the default fill, but is judged against the fill itself, so a theme that
        // changes `chip.selected` still gets readable text.
        "text.on_selected" => "readable(text.on_primary, chip.selected, 5)",
        "text.on_selected.muted" => {
            "readable(mix(text.on_selected, chip.selected / 100%, 35%), chip.selected, 5)"
        }
        "text.link" => "readable(primary, surface.window, surface.panel, 4.5)",
        "border" => p("text.primary / 12%", "text.primary / 14%"),
        "border.strong" => p("text.primary / 24%", "text.primary / 28%"),
        // The primary, moved only where it would fall below 3:1 on a surface it is drawn on.
        "border.focus" => {
            "readable(primary, surface.window, surface.panel, surface.elevated, sidebar.background, \
             sidebar.row.selected, 3)"
        }
        // One hover for every list: it steps up to band (5%), band.hover (8%), state.active.
        "state.hover" => "text.primary / 5%",
        "state.active" => p("text.primary / 12%", "text.primary / 10%"),
        // A tint of the primary marks what is current, as the sidebar's row does.
        "state.selected" => "primary / 14%",
        "state.selected.inset" => "primary / 24%",
        // Large areas: a pane under a drag.
        "state.highlight" => p("primary / 20%", "primary / 14%"),
        "list.selected" => "mix(surface.elevated, primary, 14%)",
        "list.selected.text" => "readable(primary, list.selected, 4.5)",
        "list.selected.muted" => "readable(text.muted, list.selected, 4.5)",
        // 3:1 is the WCAG minimum for non-text UI (icons, focus rings, dots).
        "primary" => "readable(terminal.ansi.4, surface.window, surface.panel, 3)",
        "primary.hover" => "shade(primary, 8%)",
        "status.success" => "readable(terminal.ansi.2, surface.window, surface.panel, 3)",
        "status.warning" => "readable(terminal.ansi.3, surface.window, surface.panel, 3)",
        "status.danger" => "readable(terminal.ansi.1, surface.window, surface.panel, 3)",
        "status.info" => "readable(terminal.ansi.6, surface.window, surface.panel, 3)",
        // The status colors as text (4.5:1) where the right panel draws them: on the
        // panel and on a selected row.
        "status.success.text" => {
            "readable(status.success, surface.panel, over(state.selected, surface.panel), 4.5)"
        }
        "status.warning.text" => {
            "readable(status.warning, surface.panel, over(state.selected, surface.panel), 4.5)"
        }
        "status.danger.text" => {
            "readable(status.danger, surface.panel, over(state.selected, surface.panel), 4.5)"
        }
        "banner.info.background" => "status.info / 10%",
        "banner.info.border" => "status.info / 35%",
        "banner.warning.background" => "status.warning / 10%",
        "banner.warning.border" => "status.warning / 35%",
        "banner.danger.background" => "status.danger / 10%",
        "banner.danger.border" => "status.danger / 35%",
        "banner.success.background" => "status.success / 10%",
        "banner.success.border" => "status.success / 35%",
        // The commands a project wants to run, told apart from what runs them: the
        // info color read as text, on the warning callout's tint and in the opaque window.
        "trust.command" => {
            "readable(status.info, over(banner.warning.background, surface.panel), surface.window, 4.5)"
        }
        "agent.working" => "primary",
        "agent.done" => "status.success",
        // Also on a card and on a group's wash at its strongest (worst case: a grey as bright as the
        // text), where a light theme's thin amber would fall short of 3:1.
        "agent.attention" => {
            "readable(status.warning, over(card, sidebar.background), \
             over(text.primary / 16%, sidebar.background), 3)"
        }
        "agent.exited" => "text.faint",
        // The bell's orange (the boards' RF-12): not the amber of "needs you", so the two badges
        // don't read alike.
        "agent.bell" => "#ff9e64",
        "agent.attention.ring" => "agent.attention / 28%",
        "agent.attention.tint" => "agent.attention / 12%",
        // As text in the sidebar (second lines) and the status bar: on the sidebar and
        // on its selected row.
        "agent.attention.text" => "readable(agent.attention, sidebar.background, sidebar.row.selected, 4.5)",
        "agent.done.text" => "readable(agent.done, sidebar.background, sidebar.row.selected, 4.5)",
        // An app icon: each agent's tile in its own colors (the boards' RF-12), in both schemes,
        // since an icon looks the same on a light sidebar.
        "agent.claude" => "#d97757",
        "agent.gemini" => "#3b7bff",
        "agent.gemini.end" => "#8f6cf5",
        "tile.background" => "#121318",
        "tile.mark" => "#c9d1e4",
        "tile.mark.brand" => "#fff",
        "tile.sheen" => "#fff / 14%",
        "tile.edge" => "#fff / 10%",
        // Diff backgrounds are opaque so their text contrast is well defined. The colors
        // also count lines (+N / −N) in the panel's rows, selected ones too.
        "diff.added" => {
            "readable(terminal.ansi.2, diff.added.background, surface.panel, over(state.selected, surface.panel), 4.5)"
        }
        "diff.added.background" => {
            p("mix(surface.window, terminal.ansi.2, 14%)", "mix(surface.window, terminal.ansi.2, 12%)")
        }
        "diff.removed" => {
            "readable(terminal.ansi.1, diff.removed.background, surface.panel, over(state.selected, surface.panel), \
             4.5)"
        }
        "diff.removed.background" => {
            p("mix(surface.window, terminal.ansi.1, 14%)", "mix(surface.window, terminal.ansi.1, 12%)")
        }
        "diff.hunk.background" => "mix(surface.window, primary, 10%)",
        "diff.hunk.text" => "readable(text.muted, diff.hunk.background, 4.5)",
        // Quieter than the code, but still readable on either tint.
        "diff.line_number" => {
            "readable(text.faint, surface.window, diff.added.background, diff.removed.background, 3)"
        }
        // Thin stripes a little lighter (dark) or darker (light) than the window.
        "diff.filler" => p("text.primary / 12%", "text.primary / 16%"),
        // Magenta: blue is the primary (the working status, focus, selection) and
        // green / red / yellow / cyan already mean added / removed / warning / info.
        // Pulled toward muted text so the branch stays quieter than the session name.
        // Also judged on its own chip (`git.branch.background` over a band). The chip
        // can't be made from the result, which would refer to itself, nor from the
        // color before any adjustment, which in a light theme is far paler than the
        // result. So a first pass finds a color, aiming a little high (5:1) so that it
        // goes at least as far as the result, and the second judges that color's chip.
        "git.branch" => {
            "readable(mix(terminal.ansi.5, text.muted, 25%), surface.window, surface.panel, surface.elevated, \
             mix(band.solid, readable(mix(terminal.ansi.5, text.muted, 25%), surface.window, surface.panel, \
             surface.elevated, mix(band.solid, mix(terminal.ansi.5, text.muted, 25%), 14%), 5), 14%), 4.5)"
        }
        "git.branch.background" => "git.branch / 14%",
        // The terminal's yellow, as in most editors' find: the current match solid with black
        // text (a dark yellow, as GitHub Light's, is lightened until black reads on it), the others
        // a tint of it the terminal's text reads on.
        "find.match" => p("find.match.current / 32%", "find.match.current / 40%"),
        "find.match.current" => "readable(terminal.ansi.3, #000, 9)",
        "find.match.current.text" => "readable(#000, find.match.current, 4.5)",
        "button.background" => {
            p("mix(surface.window, text.primary, 10%)", "mix(surface.window, text.primary, 6%)")
        }
        "button.hover" => {
            p("mix(surface.window, text.primary, 16%)", "mix(surface.window, text.primary, 11%)")
        }
        "button.text" => "readable(text.primary, button.background, button.hover, 4.5)",
        "input.background" => p("mix(surface.window, #000, 22%)", "mix(surface.window, #fff, 70%)"),
        "input.border" => "border.strong",
        "input.border.focus" => "primary",
        "input.placeholder" => "readable(text.faint, input.background, 3)",
        "checkbox.background" => "input.background",
        "checkbox.checked" => "primary",
        "checkbox.border" => "text.faint",
        // Light on both the off (border) and on (primary) track, in either scheme.
        "switch.knob" => "#fff",
        // The raised neutral the title tab sits on; the window colour would vanish into the row.
        "tab.active.background" => "surface.elevated",
        "tab.inactive.text" => "readable(text.muted, titlebar.background, 4.5)",
        "chip.border" => "border.strong",
        // Choice controls stay solid: a segmented control's chosen segment, the active tab.
        "chip.selected" => "primary",
        "chip.tint" => "primary / 16%",
        "scrollbar.thumb" => p("text.primary / 25%", "text.primary / 30%"),
        "resize.handle.hover" => "primary / 50%",
        "resize.handle.active" => "primary",
        // Light themes get dark tooltips, the common macOS/web convention.
        "tooltip.background" => p("mix(surface.window, text.primary, 14%)", "text.primary"),
        "tooltip.text" => p(
            "readable(text.primary, tooltip.background, 4.5)",
            "readable(surface.window, tooltip.background, 4.5)",
        ),
        "shadow" => p("#000 / 50%", "#000 / 16%"),
        "selection.ui" => p("primary / 30%", "primary / 25%"),
        _ => return None,
    })
}

/// `readable(X, …, ratio)` on every background code is drawn on: the terminal, and
/// the added and removed lines of a diff (a keyword on a removed line must read too).
macro_rules! on_code {
    ($color:literal, $ratio:literal) => {
        concat!(
            "readable(",
            $color,
            ", terminal.background, diff.added.background, diff.removed.background, ",
            $ratio,
            ")"
        )
    };
}

/// (key, color rule, bold, italic). Sub-keys refer to their parent so that
/// overriding `keyword` also recolors `keyword.control` etc. Mapping follows
/// the common One Dark / Tokyo Night convention: keywords magenta, strings
/// green, functions blue, types yellow, numbers/constants orange.
pub(super) const SYNTAX_RULES: &[(&str, &str, bool, bool)] = &[
    ("attribute", on_code!("terminal.ansi.3", "4.5"), false, true),
    ("boolean", "syntax.constant", false, false),
    ("comment", on_code!("mix(terminal.foreground, terminal.background, 45%)", "3"), false, true),
    ("comment.doc", "syntax.comment", false, true),
    ("constant", "syntax.number", false, false),
    ("constant.builtin", "syntax.constant", false, false),
    ("constructor", "syntax.type", false, false),
    ("embedded", "syntax.variable", false, false),
    ("emphasis", "syntax.variable", false, true),
    ("emphasis.strong", "syntax.variable", true, false),
    ("enum", "syntax.type", false, false),
    ("function", on_code!("terminal.ansi.4", "4.5"), false, false),
    ("function.builtin", on_code!("terminal.ansi.6", "4.5"), false, false),
    ("function.macro", "syntax.function.builtin", false, false),
    ("function.method", "syntax.function", false, false),
    ("keyword", on_code!("terminal.ansi.5", "4.5"), false, false),
    ("keyword.control", "syntax.keyword", false, false),
    ("keyword.operator", "syntax.keyword", false, false),
    ("label", "syntax.keyword", false, false),
    ("link_text", "syntax.function", false, false),
    ("link_uri", "syntax.property", false, true),
    ("namespace", "syntax.type", false, false),
    ("number", on_code!("mix(terminal.ansi.1, terminal.ansi.3, 50%)", "4.5"), false, false),
    ("operator", on_code!("mix(terminal.ansi.6, terminal.foreground, 40%)", "4.5"), false, false),
    ("preproc", "syntax.keyword", false, false),
    ("property", on_code!("terminal.ansi.6", "4.5"), false, false),
    ("punctuation", on_code!("mix(terminal.foreground, terminal.background, 25%)", "3"), false, false),
    ("punctuation.bracket", "syntax.punctuation", false, false),
    ("punctuation.delimiter", "syntax.punctuation", false, false),
    ("punctuation.special", "syntax.operator", false, false),
    ("string", on_code!("terminal.ansi.2", "4.5"), false, false),
    ("string.escape", on_code!("terminal.ansi.6", "4.5"), false, false),
    ("string.regex", "syntax.string.escape", false, false),
    ("string.special", "syntax.string.escape", false, false),
    ("tag", on_code!("terminal.ansi.1", "4.5"), false, false),
    ("text.literal", "syntax.string", false, false),
    ("title", "syntax.function", true, false),
    ("type", on_code!("terminal.ansi.3", "4.5"), false, false),
    ("type.builtin", "syntax.type", false, false),
    ("variable", on_code!("terminal.foreground", "4.5"), false, false),
    ("variable.builtin", on_code!("terminal.ansi.1", "4.5"), false, false),
    ("variable.parameter", "syntax.variable", false, false),
    ("variable.special", "syntax.variable.builtin", false, false),
    ("variant", "syntax.constant", false, false),
];

pub(super) fn rule_flags(key: &str) -> (bool, bool) {
    SYNTAX_RULES.iter().find(|r| r.0 == key).map_or((false, false), |r| (r.2, r.3))
}

/// Tokens older theme files may still name, and what to write instead.
pub(super) fn replaced_by(name: &str) -> Option<&'static str> {
    Some(match name {
        "accent" => "use `primary`",
        "accent.hover" => "use `primary.hover`",
        "text.on_accent" => "use `text.on_primary`",
        "accent.muted" => "removed; write `primary / 20%` where a tint is needed",
        "button.primary.background" | "button.primary.hover" | "button.primary.text" => {
            "removed; primary buttons use `primary`, `primary.hover` and `text.on_primary`"
        }
        _ => return None,
    })
}

/// `status.warning` for a light palette whose ANSI yellow is darker than its muted text,
/// such as GitHub Light's #4d2d00: from the yellow, the warning would be a brown that
/// passes every contrast check and still reads as text, not as a warning. Such a palette
/// gets an amber instead. The resolver applies it unless a theme sets `status.warning`.
/// Dark palettes keep their yellow: a dim one is lifted by `readable()` and still looks
/// like a warning on a dark background (Solarized's would otherwise turn into this amber).
const DARK_YELLOW_WARNING: &str = "readable(#bf8700, surface.window, surface.panel, 3)";

pub(super) fn dark_yellow_warning(appearance: Appearance) -> Option<&'static Expr> {
    static RULE: LazyLock<Expr> = LazyLock::new(|| {
        parse_expr(DARK_YELLOW_WARNING).unwrap_or_else(|e| panic!("built-in rule for status.warning: {e}"))
    });
    (appearance == Appearance::Light).then(|| &*RULE)
}

/// Every default rule, parsed, for one appearance. Parsed once: it takes
/// longer than the rest of deriving a theme.
pub(super) fn default_rules(appearance: Appearance) -> &'static BTreeMap<String, Expr> {
    static DARK: LazyLock<BTreeMap<String, Expr>> = LazyLock::new(|| parse_rules(Appearance::Dark));
    static LIGHT: LazyLock<BTreeMap<String, Expr>> = LazyLock::new(|| parse_rules(Appearance::Light));
    match appearance {
        Appearance::Dark => &DARK,
        Appearance::Light => &LIGHT,
    }
}

fn parse_rules(appearance: Appearance) -> BTreeMap<String, Expr> {
    let mut rules = BTreeMap::new();
    let dark = appearance == Appearance::Dark;
    let sources = UI_TOKENS
        .iter()
        .map(|t| (t.name.to_string(), ui_rule(t.name, dark)))
        .chain(SYNTAX_RULES.iter().map(|r| (format!("syntax.{}", r.0), Some(r.1))));
    for (name, src) in sources {
        // The rule table is static and every entry is exercised by tests, so a
        // failure here is a programming error rather than bad input.
        let expr = src
            .ok_or_else(|| "no rule".to_string())
            .and_then(parse_expr)
            .unwrap_or_else(|e| panic!("built-in rule for {name}: {e}"));
        rules.insert(name, expr);
    }
    rules
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Appearance, SYNTAX_RULES, UI_TOKENS, default_rule, default_rules};
    use crate::tokens::expr::parse_expr;
    use crate::tokens::resolve::MAX_REF_DEPTH;
    use crate::tokens::test_support::{builtin, chain_len, ratio, resolve_str};
    use crate::tokens::{Color, Metrics, builtin_tokens, syntax_keys};

    #[test]
    fn every_rule_parses_for_both_appearances() {
        for app in [Appearance::Dark, Appearance::Light] {
            let rules = default_rules(app);
            for t in UI_TOKENS {
                assert!(rules.contains_key(t.name), "{}", t.name);
                assert!(default_rule(t.name, app).is_some());
            }
            assert_eq!(rules.len(), UI_TOKENS.len() + SYNTAX_RULES.len());
        }
        let mut names: Vec<_> = UI_TOKENS.iter().map(|t| t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), UI_TOKENS.len(), "duplicate token");
        assert_eq!(default_rule("syntax.keyword", Appearance::Dark).unwrap(), SYNTAX_RULES[15].1);
        assert!(default_rule("nope", Appearance::Dark).is_none());
    }

    #[test]
    fn builtins_are_complete_and_readable() {
        let all = builtin_tokens();
        assert_eq!(all.len(), 19);
        for t in &all {
            let n = &t.name;
            assert_eq!(t.ui_tokens().count(), UI_TOKENS.len(), "{n}");
            for info in UI_TOKENS {
                let c = t.get(info.name).unwrap_or_else(|| panic!("{n}: {}", info.name));
                assert!([c.r, c.g, c.b, c.a].iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)), "{n}");
            }
            for key in syntax_keys() {
                assert!(t.syntax.contains_key(key), "{n}: {key}");
            }
            assert!(ratio(t, "text.primary", "surface.window") >= 7.0, "{n}");
            assert!(ratio(t, "text.primary", "surface.panel") >= 7.0, "{n}");
            for bg in ["surface.window", "surface.panel", "surface.elevated"] {
                assert!(ratio(t, "text.muted", bg) >= 4.5, "{n}: muted on {bg}");
            }
            assert!(ratio(t, "text.faint", "surface.window") >= 3.0, "{n}");
            // Muted must still look secondary, faint more so.
            assert!(ratio(t, "text.muted", "surface.window") < ratio(t, "text.primary", "surface.window"));
            assert!(ratio(t, "text.faint", "surface.window") <= ratio(t, "text.muted", "surface.window"));
            // The terminal pair reflects the palette itself (Solarized's
            // famously soft 4.3:1); derivation can only fix the UI tokens.
            let warnings: Vec<_> =
                t.contrast_warnings().into_iter().filter(|w| !w.0.starts_with("terminal.")).collect();
            assert!(warnings.is_empty(), "{n}: {warnings:?}");
            // Code is drawn on the terminal and on a diff's added and removed lines.
            let backgrounds = [
                t.terminal.background,
                t.get("diff.added.background").unwrap(),
                t.get("diff.removed.background").unwrap(),
            ];
            for (key, s) in &t.syntax {
                let min =
                    if key.starts_with("comment") || key.starts_with("punctuation") { 3.0 } else { 4.5 };
                for bg in backgrounds {
                    let r = Color::contrast_ratio(s.color, bg);
                    assert!(r >= min, "{n}: syntax {key} on {} is {r:.2}", bg.to_hex());
                }
            }
            assert_eq!(t.metrics, Metrics::default());
        }
        let light: Vec<_> =
            all.iter().filter(|t| t.appearance == Appearance::Light).map(|t| &t.name).collect();
        assert_eq!(light, ["Catppuccin Latte", "GitHub Light"]);
    }

    #[test]
    fn derivation_follows_palette() {
        let t = builtin("Tokyo Night");
        assert_eq!(t.get("surface.window"), Some(t.terminal.background));
        // Tokyo Night's blue already has enough contrast, so it is used as-is.
        assert_eq!(t.get("primary"), Some(t.terminal.ansi[4]));
        assert_eq!(t.get("status.danger"), Some(t.terminal.ansi[1]));
        assert_eq!(t.get("agent.working"), t.get("primary"));
        assert_eq!(t.get("agent.exited"), t.get("text.faint"));
        assert!(t.get("surface.panel").unwrap().luminance() < t.get("surface.window").unwrap().luminance());
        assert!(
            t.get("surface.elevated").unwrap().luminance() > t.get("surface.window").unwrap().luminance()
        );
        assert!(t.get("state.hover").unwrap().a < 0.2);
        assert_eq!(t.syntax["keyword"].color, t.terminal.ansi[5]);
        assert!(t.syntax["comment"].italic);
        assert!(t.syntax["title"].bold);

        // Latte's yellow is too pale on its background, so it gets darkened.
        let latte = builtin("Catppuccin Latte");
        let warn = latte.get("status.warning").unwrap();
        assert!(warn.luminance() < latte.terminal.ansi[3].luminance());
        // Light themes get dark tooltips.
        assert!(latte.get("tooltip.background").unwrap().luminance() < 0.2);
    }

    #[test]
    fn branch_color_is_readable_quiet_and_distinct() {
        let dist =
            |a: Color, b: Color| ((a.r - b.r).powi(2) + (a.g - b.g).powi(2) + (a.b - b.b).powi(2)).sqrt();
        for t in builtin_tokens() {
            let n = &t.name;
            let branch = t.get("git.branch").unwrap_or_else(|| panic!("{n}"));
            for bg in ["sidebar.background", "surface.panel", "surface.window", "surface.elevated"] {
                assert!(ratio(&t, "git.branch", bg) >= 4.5, "{n}: branch on {bg}");
            }
            // Secondary to the session name it sits under.
            assert!(
                ratio(&t, "git.branch", "surface.panel") < ratio(&t, "text.primary", "surface.panel"),
                "{n}"
            );
            // Never mistaken for the primary (selection, working) or a status color.
            for other in ["primary", "status.success", "status.warning", "status.danger", "diff.added"] {
                let c = t.get(other).unwrap();
                assert!(
                    dist(branch, c) > 0.1,
                    "{n}: branch {} too close to {other} {}",
                    branch.to_hex(),
                    c.to_hex()
                );
            }
        }
        let t = resolve_str(r##"{"ui": {"git.branch": "terminal.ansi.4"}}"##).unwrap();
        assert_eq!(t.get("git.branch"), Some(t.terminal.ansi[4]));
    }

    #[test]
    fn built_in_rules_stay_far_inside_the_limits() {
        for app in [Appearance::Dark, Appearance::Light] {
            let rules = default_rules(app);
            let mut memo = BTreeMap::new();
            let longest = rules.keys().map(|n| chain_len(n, rules, &mut memo)).max().unwrap_or(0);
            assert!(longest < MAX_REF_DEPTH / 4, "{app:?}: chain of {longest}");
            let sources = UI_TOKENS
                .iter()
                .map(|t| t.name.to_string())
                .chain(SYNTAX_RULES.iter().map(|r| format!("syntax.{}", r.0)));
            for name in sources {
                let src = default_rule(&name, app).unwrap_or_default();
                // Parentheses use up the parser's nesting budget without adding to the tree.
                let deep_enough = format!("{}{src}{}", "(".repeat(26), ")".repeat(26));
                assert!(parse_expr(&deep_enough).is_ok(), "{name}: rule nests more than 6 deep");
            }
        }
    }
}
