//! Resolving a theme file: walking its `extends` chain, merging the layers
//! and deriving everything else from the final palette.

use std::collections::BTreeMap;

use super::catalog::UI_TOKENS;
use super::expr::{Expr, parse_expr};
use super::file::{MetricsFile, SyntaxOverride, ThemeFile};
use super::model::{Metrics, TerminalColors, auto_appearance};
use super::own::{own, own_named};
use super::resolve::{FlagOverrides, NO_RULES, Resolver, build};
use super::rules::replaced_by;
use super::theme_tokens::ThemeTokens;
use super::variants::variants;
use crate::text::did_you_mean;
use crate::theme;

/// Longest `extends` chain accepted; deeper is almost surely a mistake.
const MAX_EXTENDS_DEPTH: usize = 16;

/// Most syntax keys a theme (with everything it extends) may style. There are
/// 44 built-in keys and editors use a few dozen more; the cap bounds the work
/// an untrusted theme file can cause.
pub(super) const MAX_SYNTAX_KEYS: usize = 1024;

fn builtin_named(name: &str) -> Option<&'static theme::Theme> {
    let all = theme::builtins();
    all.iter().find(|t| t.name == name).or_else(|| all.iter().find(|t| t.name.eq_ignore_ascii_case(name)))
}

/// Resolve a theme file into tokens.
///
/// `extends` names are looked up with `lookup` first (user themes), then among
/// the built-ins. A name already seen in the chain skips `lookup`, so a user
/// theme called "Nord" can extend the built-in "Nord"; if no built-in matches
/// either, that is reported as a cycle. Overrides are merged from the root
/// ancestor down, then terminal colors are resolved, then every UI/syntax
/// token is derived from the final palette with the merged overrides.
pub fn resolve(file: &ThemeFile, lookup: &dyn Fn(&str) -> Option<ThemeFile>) -> Result<ThemeTokens, String> {
    // A file that ships several versions is shown as its first.
    if let Some(first) = variants(file, file.name.as_deref().unwrap_or("Theme"))?.into_iter().next() {
        return resolve(&first.file, lookup);
    }
    let mut chain = vec![file.clone()];
    let mut seen: Vec<String> = file.name.iter().cloned().collect();
    let mut parent = file.extends.clone();
    let base = loop {
        let Some(name) = parent else {
            break theme::builtins().first().ok_or("no built-in themes")?;
        };
        if chain.len() > MAX_EXTENDS_DEPTH {
            return Err(format!("{}: `extends` chain is deeper than {MAX_EXTENDS_DEPTH}", file.label()));
        }
        let shadowed = seen.contains(&name);
        // The user's themes first, then Wormhole's own (theme files too), then the palettes.
        let own_file = || own_named(&name).map(|o| o.file.clone());
        match if shadowed { None } else { lookup(&name) }.or_else(own_file) {
            Some(f) => {
                seen.push(name);
                parent = f.extends.clone();
                chain.push(f);
            }
            None => match builtin_named(&name) {
                Some(t) => break t,
                None if shadowed => {
                    let path: Vec<&str> = seen.iter().map(String::as_str).chain([name.as_str()]).collect();
                    return Err(format!("`extends` cycle: {}", path.join(" -> ")));
                }
                None => {
                    let names = theme::builtins().iter().map(|t| t.name.as_str());
                    let hint = did_you_mean(&name, names.chain(own().iter().map(|o| o.name.as_str())));
                    let from = chain.last().map_or_else(String::new, |f| f.label());
                    return Err(format!("{from}: extends unknown theme {name:?}{hint}"));
                }
            },
        }
    };

    // Merge from the root ancestor down; each entry remembers its file for
    // error messages.
    let mut terminal_over: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut colors: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut flags = FlagOverrides::new();
    let mut metrics = MetricsFile::default();
    let mut appearance = None;
    let mut name = None;
    for f in chain.iter().rev() {
        let label = f.label();
        for (node, v) in f.terminal.entries().map_err(|e| format!("{label}: {e}"))? {
            terminal_over.insert(node, (v, label.clone()));
        }
        for (k, v) in &f.ui {
            if !UI_TOKENS.iter().any(|t| t.name == k) {
                if let Some(now) = replaced_by(k) {
                    return Err(format!("{label}: ui token {k:?} no longer exists: {now}"));
                }
                let hint = did_you_mean(k, UI_TOKENS.iter().map(|t| t.name));
                return Err(format!("{label}: unknown ui token {k:?}{hint}"));
            }
            colors.insert(k.clone(), (v.clone(), label.clone()));
        }
        for (k, v) in &f.syntax {
            let (color, bold, italic) = match v {
                SyntaxOverride::Color(c) => (Some(c), None, None),
                SyntaxOverride::Style { color, bold, italic } => (color.as_ref(), *bold, *italic),
            };
            if let Some(c) = color {
                colors.insert(format!("syntax.{k}"), (c.clone(), label.clone()));
            }
            let entry = flags.entry(k.clone()).or_default();
            entry.0 = bold.or(entry.0);
            entry.1 = italic.or(entry.1);
        }
        metrics.merge(&f.metrics);
        appearance = f.appearance.or(appearance);
        name = f.name.clone().or(name);
    }

    if flags.len() > MAX_SYNTAX_KEYS {
        return Err(format!("{}: styles more than {MAX_SYNTAX_KEYS} syntax keys", file.label()));
    }

    // Terminal colors first: they may only reference each other, since the
    // UI is derived from them.
    let mut rules = BTreeMap::new();
    for (node, (src, label)) in &terminal_over {
        let e = parse_expr(src).map_err(|e| format!("{label}: {node}: {e}"))?;
        rules.insert(node.clone(), e);
    }
    let mut base_terminal = TerminalColors::from_theme(base);
    let mut values = base_terminal.named();
    // Unset optional colors follow whatever they default to after overrides.
    if base_terminal.cursor_text.is_none() {
        rules.entry("terminal.cursor_text".into()).or_insert(Expr::Ref("terminal.background".into()));
    }
    if base_terminal.selection_foreground.is_none() {
        rules
            .entry("terminal.selection_foreground".into())
            .or_insert(Expr::Ref("terminal.foreground".into()));
    }
    values.retain(|(n, _)| !rules.contains_key(n));
    let mut r = Resolver::new(&NO_RULES, rules, values);
    r.scope_hint = "; terminal colors may only reference terminal.* colors";
    for (node, (_, label)) in &terminal_over {
        let c = r.color(node).map_err(|e| format!("{label}: {node}: {e}"))?;
        if c.a < 1.0 {
            return Err(format!("{label}: {node}: terminal colors must be opaque"));
        }
        base_terminal.set(&node["terminal.".len()..], c);
    }
    let terminal = base_terminal;

    let mut overrides = BTreeMap::new();
    for (node, (src, label)) in colors {
        let e = parse_expr(&src).map_err(|e| format!("{label}: {node}: {e}"))?;
        overrides.insert(node, (e, label));
    }
    let mut m = Metrics::default();
    metrics.apply(&mut m)?;
    let appearance = appearance.unwrap_or_else(|| auto_appearance(&terminal));
    let name = name.unwrap_or_else(|| base.name.clone());
    build(name, terminal, appearance, m, overrides, &flags)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::tokens::test_support::{builtin, on_small_stack, resolve_str};
    use crate::tokens::{
        Appearance, Color, MAX_SYNTAX_KEYS, MAX_THEME_BYTES, Metrics, ThemeFile, default_rule,
        parse_theme_file, resolve,
    };

    /// A theme whose syntax keys `k0`, `k1`, ... each refer to the next one,
    /// wrapped in `nesting` calls.
    fn syntax_chain(links: usize, nesting: usize) -> String {
        let entries: Vec<String> = (0..links)
            .map(|i| {
                let next = if i + 1 == links { "#123456".to_string() } else { format!("syntax.k{}", i + 1) };
                let value = format!("{}{next}{}", "lighten(".repeat(nesting), ", 1%)".repeat(nesting));
                format!("\"k{i}\": \"{value}\"")
            })
            .collect();
        format!("{{\"syntax\": {{{}}}}}", entries.join(", "))
    }

    #[test]
    fn chip_radius_follows_radius_sm_unless_set() {
        assert_eq!(Metrics::default().radius_chip, Metrics::default().radius_sm);
        // Files from before the chip radius existed still load, and their chips
        // follow their radius_sm.
        let t = resolve_str(r#"{"metrics": {"radius_sm": 2}}"#).unwrap();
        assert_eq!((t.metrics.radius_sm, t.metrics.radius_chip), (2.0, 2.0));
        let t = resolve_str(r#"{"metrics": {"radius_chip": 11}}"#).unwrap();
        assert_eq!((t.metrics.radius_sm, t.metrics.radius_chip), (4.0, 11.0));
        // A value set up the `extends` chain is an override, not a derivation.
        let parent = parse_theme_file(r#"{"name": "P", "metrics": {"radius_chip": 0}}"#).unwrap();
        let lookup = |n: &str| (n == "P").then(|| parent.clone());
        let child = parse_theme_file(r#"{"extends": "P", "metrics": {"radius_sm": 3}}"#).unwrap();
        let t = resolve(&child, &lookup).unwrap();
        assert_eq!((t.metrics.radius_sm, t.metrics.radius_chip), (3.0, 0.0));
        let e = resolve_str(r#"{"metrics": {"radius_chip": 300}}"#).unwrap_err();
        assert!(e.contains("metrics.radius_chip"), "{e}");
        assert_eq!(default_rule("metrics.radius_chip", Appearance::Dark).as_deref(), Some("radius_sm"));
        assert_eq!(default_rule("metrics.radius", Appearance::Light), None);
    }

    #[test]
    fn terminal_overrides_rederive() {
        let base = builtin("Tokyo Night");
        let t =
            resolve_str(r##"{"extends": "Tokyo Night", "terminal": {"background": "#000000"}}"##).unwrap();
        assert_eq!(t.get("surface.window").unwrap().to_hex(), "#000000");
        assert_ne!(t.get("text.muted"), base.get("text.muted"));
        assert_ne!(t.get("button.background"), base.get("button.background"));
        assert_eq!(t.terminal.foreground, base.terminal.foreground);
        assert_eq!(t.name, "Tokyo Night");

        // A light background flips the rule set when appearance is not given.
        let t = resolve_str(r##"{"terminal": {"background": "#fafafa", "foreground": "#222"}}"##).unwrap();
        assert_eq!(t.appearance, Appearance::Light);
        assert!(t.get("surface.panel").unwrap().luminance() < t.get("surface.window").unwrap().luminance());
        let forced =
            resolve_str(r##"{"appearance": "dark", "terminal": {"background": "#fafafa"}}"##).unwrap();
        assert_eq!(forced.appearance, Appearance::Dark);

        // Terminal expressions may reference other (final) terminal colors.
        let t = resolve_str(
            r##"{"extends": "Nord", "terminal": {"background": "#101010", "cursor": "mix(terminal.background, terminal.ansi.4, 50%)"}}"##,
        )
        .unwrap();
        let nord = builtin("Nord");
        assert_eq!(t.terminal.cursor, Color::from_rgb_u32(0x101010).mix(nord.terminal.ansi[4], 0.5));
    }

    #[test]
    fn extends_chains() {
        let files: BTreeMap<&str, &str> = [
            ("A", r##"{"name": "A", "extends": "Dracula", "ui": {"primary": "#ff0000"}, "metrics": {"radius": 2}}"##),
            ("B", r##"{"name": "B", "extends": "A", "ui": {"border": "primary / 50%"}, "syntax": {"keyword": {"bold": true}}}"##),
            ("C", r##"{"name": "C", "extends": "B", "terminal": {"ansi": {"red": "#00ff00"}}, "ui": {"primary": "terminal.ansi.1"}}"##),
            ("Loop1", r#"{"name": "Loop1", "extends": "Loop2"}"#),
            ("Loop2", r#"{"name": "Loop2", "extends": "Loop1"}"#),
            ("Nord", r##"{"name": "Nord", "extends": "Nord", "ui": {"primary": "#123456"}}"##),
            ("Bad", r#"{"name": "Bad", "extends": "Draculla"}"#),
        ]
        .into_iter()
        .collect();
        let lookup = |n: &str| files.get(n).map(|s| parse_theme_file(s).unwrap());
        let get = |n: &str| resolve(&lookup(n).unwrap(), &lookup);

        let c = get("C").unwrap();
        assert_eq!(c.name, "C");
        // C's primary override wins over A's and references C's own ANSI red,
        // and B's border (defined relative to primary) follows it.
        assert_eq!(c.get("primary").unwrap().to_hex(), "#00ff00");
        assert_eq!(c.get("border").unwrap().to_hex(), "#00ff0080");
        assert_eq!(c.metrics.radius, 2.0);
        assert!(c.syntax["keyword"].bold);
        assert_eq!(c.terminal.background, builtin("Dracula").terminal.background);

        let e = get("Loop1").unwrap_err();
        assert!(e.contains("`extends` cycle: Loop1 -> Loop2 -> Loop1"), "{e}");
        let e = get("Bad").unwrap_err();
        assert!(e.contains("unknown theme \"Draculla\" (did you mean `Dracula`?)"), "{e}");

        // A user theme may shadow and extend the built-in of the same name.
        let n = get("Nord").unwrap();
        assert_eq!(n.get("primary").unwrap().to_hex(), "#123456");
        assert_eq!(n.terminal, builtin("Nord").terminal);

        // Depth limit.
        let deep = |n: &str| {
            let i: usize = n.strip_prefix('D')?.parse().ok()?;
            Some(ThemeFile {
                name: Some(n.into()),
                extends: Some(format!("D{}", i + 1)),
                ..Default::default()
            })
        };
        let e = resolve(&deep("D0").unwrap(), &deep).unwrap_err();
        assert!(e.contains("deeper than 16"), "{e}");

        // No extends: the default built-in supplies the palette.
        let t = resolve_str("{}").unwrap();
        assert_eq!(t.terminal, builtin("Tokyo Night").terminal);
        // Built-in names match case-insensitively as a fallback.
        assert_eq!(resolve_str(r#"{"extends": "nord"}"#).unwrap().terminal, builtin("Nord").terminal);
    }

    #[test]
    fn syntax_overrides() {
        let t = resolve_str(
            r##"{"syntax": {"function": "#123456", "type": {"bold": true}, "markup.heading": {"italic": true},
                "custom": "syntax.keyword"}}"##,
        )
        .unwrap();
        assert_eq!(t.syntax["function"].color.to_hex(), "#123456");
        assert_eq!(t.syntax["function.method"].color.to_hex(), "#123456");
        assert!(t.syntax["type"].bold);
        let base = builtin("Tokyo Night");
        assert_eq!(t.syntax["type"].color, base.syntax["type"].color);
        assert_eq!(t.syntax["markup.heading"].color, t.syntax["variable"].color);
        assert!(t.syntax["markup.heading"].italic);
        assert_eq!(t.syntax["custom"].color, t.syntax["keyword"].color);
        assert_eq!(t.get("syntax.custom"), Some(t.syntax["keyword"].color));
    }

    #[test]
    fn long_reference_chains_are_an_error_not_a_crash() {
        let (long, deep_and_long, fine) = on_small_stack(|| {
            let long = resolve_str(&syntax_chain(1000, 0)).map(|_| ()).unwrap_err();
            // The longest chain allowed, with every link nested as deep as allowed.
            let deep_and_long = resolve_str(&syntax_chain(63, 31)).map(|_| ()).unwrap_err();
            let fine = resolve_str(&syntax_chain(63, 2)).map(|t| t.syntax["k0"].color.to_hex());
            (long, deep_and_long, fine)
        });
        assert!(long.contains("reference chain is longer than 64 (syntax.k0 -> … -> syntax.k64)"), "{long}");
        assert!(long.starts_with("theme \"(unnamed)\": syntax \"k0\""), "{long}");
        assert!(deep_and_long.contains("expressions nest deeper than 256 levels"), "{deep_and_long}");
        assert!(deep_and_long.contains("(syntax.k0 -> … -> syntax.k8)"), "{deep_and_long}");
        assert_eq!(fine.as_deref(), Ok("#ffffff"));
    }

    #[test]
    fn theme_files_have_size_and_key_limits() {
        let big = format!("{{\"name\": \"{}\"}}", "x".repeat(MAX_THEME_BYTES));
        let e = parse_theme_file(&big).unwrap_err();
        assert!(e.contains("larger than 256 KiB"), "{e}");
        let at_limit = format!("{{\"name\": \"{}\"}}", "x".repeat(MAX_THEME_BYTES - 12));
        assert!(parse_theme_file(&at_limit).is_ok());

        let keys = |n: usize| {
            let entries: Vec<String> = (0..n).map(|i| format!("\"k{i}\": \"#000\"")).collect();
            format!("{{\"syntax\": {{{}}}}}", entries.join(", "))
        };
        let e = resolve_str(&keys(MAX_SYNTAX_KEYS + 1)).map(|_| ()).unwrap_err();
        assert!(e.contains("more than 1024 syntax keys"), "{e}");
        assert_eq!(
            resolve_str(&keys(MAX_SYNTAX_KEYS)).map(|t| t.syntax.len()).ok(),
            Some(MAX_SYNTAX_KEYS + crate::tokens::syntax_keys().count())
        );
    }

    #[test]
    fn layered_over_resolves_like_the_chain() {
        let parent = parse_theme_file(
            r##"{"name": "Foo", "extends": "Nord", "appearance": "dark",
                "terminal": {"background": "#101010", "ansi": {"red": "#aa0000", "4": "#0000aa"}},
                "ui": {"primary": "#ff0000", "border": "primary / 30%"},
                "syntax": {"keyword": {"color": "#bb9af7", "italic": true}, "string": "#9ece6a", "type": {"bold": true}},
                "metrics": {"radius": 9, "radius_chip": 2}}"##,
        )
        .unwrap();
        let child = parse_theme_file(
            r##"{"name": "Foo", "extends": "Foo",
                "terminal": {"ansi": {"1": "#00aa00"}, "foreground": "#eeeeee"},
                "ui": {"border": "#00ff00"},
                "syntax": {"keyword": {"bold": true}, "string": {"italic": true}, "type": "#123456", "custom": "#654321"},
                "metrics": {"radius": 3}}"##,
        )
        .unwrap();
        let folded = child.layered_over(&parent);
        assert_eq!(folded.name.as_deref(), Some("Foo"));
        assert_eq!(folded.extends.as_deref(), Some("Nord"));
        assert_eq!(folded.terminal.ansi.keys().collect::<Vec<_>>(), ["1", "4"]);

        // The draft as the Studio resolves it: named differently, extending Foo.
        let draft = ThemeFile { name: Some("Foo Custom".into()), ..child.clone() };
        let lookup = |n: &str| (n == "Foo").then(|| parent.clone());
        let expected = resolve(&draft, &lookup).unwrap();
        let mut got = resolve(&folded, &|_| None).unwrap();
        got.name = expected.name.clone();
        assert_eq!(got, expected);
        assert_eq!(got.syntax["keyword"].color.to_hex(), "#bb9af7");
        assert!(got.syntax["keyword"].bold && got.syntax["keyword"].italic);
        assert!(got.syntax["type"].bold);
        assert_eq!((got.metrics.radius, got.metrics.radius_chip), (3.0, 2.0));
    }
}
