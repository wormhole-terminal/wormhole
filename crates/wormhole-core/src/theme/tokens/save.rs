//! Saving: turning resolved tokens back into a theme file, either complete or
//! as the minimal difference from a base theme.

use std::collections::{BTreeMap, BTreeSet};

use super::expr::Expr;
use super::file::{MetricsFile, SyntaxOverride, ThemeFile};
use super::model::{Metrics, auto_appearance};
use super::resolve::{NO_RULES, Resolver};
use super::rules::{dark_yellow_warning, default_rules, rule_flags};
use super::theme_tokens::ThemeTokens;
use crate::theme::tokens::color::Color;

impl ThemeTokens {
    /// A theme file that reproduces `self`. With `base`, it `extends` the base
    /// and holds only what differs: terminal colors, then UI/syntax colors
    /// the rules would not produce anyway (visited in dependency order, so an
    /// overridden `primary` does not also dump `border.focus`), then flags and
    /// metrics. Values are saved as hex; expressions are not recoverable from
    /// resolved tokens, so the Studio should keep editing the file itself
    /// when it has one. Exact when `base` is a built-in or itself a saved
    /// minimal file.
    pub fn to_theme_file(&self, base: Option<&ThemeTokens>) -> ThemeFile {
        let Some(base) = base else { return self.full_theme_file() };
        let mut file = ThemeFile {
            name: Some(self.name.clone()),
            extends: Some(base.name.clone()),
            ..Default::default()
        };
        // Resolution only knows an explicit appearance or the auto-detected
        // one, so be explicit whenever either could disagree with ours.
        if self.appearance != base.appearance || self.appearance != auto_appearance(&self.terminal) {
            file.appearance = Some(self.appearance);
        }
        for (name, c) in self.terminal.named() {
            let base_c = base.get(&name);
            let key = &name["terminal.".len()..];
            let explicit = match key {
                "cursor_text" => self.terminal.cursor_text.is_some(),
                "selection_foreground" => self.terminal.selection_foreground.is_some(),
                _ => true,
            };
            if explicit && base_c.map(Color::to_bytes) != Some(c.to_bytes()) {
                file.terminal.insert(key, c.to_hex());
            }
        }
        let inherited = minimal_color_overrides(base, &BTreeMap::new());
        let own = minimal_color_overrides(self, &inherited);
        for (name, c) in own {
            match name.strip_prefix("syntax.") {
                Some(key) => {
                    file.syntax.insert(key.to_string(), SyntaxOverride::Color(c.to_hex()));
                }
                None => {
                    file.ui.insert(name, c.to_hex());
                }
            }
        }
        for (key, style) in &self.syntax {
            let expected =
                base.syntax.get(key).map(|s| (s.bold, s.italic)).unwrap_or_else(|| rule_flags(key));
            if (style.bold, style.italic) == expected {
                continue;
            }
            let color = match file.syntax.remove(key) {
                Some(SyntaxOverride::Color(c)) => Some(c),
                _ => None,
            };
            let style = SyntaxOverride::Style { color, bold: Some(style.bold), italic: Some(style.italic) };
            file.syntax.insert(key.clone(), style);
        }
        file.metrics = MetricsFile::diff(&self.metrics, Some(&base.metrics));
        file
    }

    fn full_theme_file(&self) -> ThemeFile {
        let mut file = ThemeFile {
            name: Some(self.name.clone()),
            appearance: Some(self.appearance),
            ..Default::default()
        };
        for (name, c) in self.terminal.named() {
            let key = &name["terminal.".len()..];
            let skip = (key == "cursor_text" && self.terminal.cursor_text.is_none())
                || (key == "selection_foreground" && self.terminal.selection_foreground.is_none());
            if !skip {
                file.terminal.insert(key, c.to_hex());
            }
        }
        file.ui = self.ui.iter().map(|(k, c)| (k.clone(), c.to_hex())).collect();
        file.syntax = self
            .syntax
            .iter()
            .map(|(k, s)| {
                let o = SyntaxOverride::Style {
                    color: Some(s.color.to_hex()),
                    bold: Some(s.bold),
                    italic: Some(s.italic),
                };
                (k.clone(), o)
            })
            .collect();
        file.metrics = MetricsFile::diff(&self.metrics, None);
        file
    }
}

/// UI/syntax colors of `tokens` that its own rules (plus `seed` overrides)
/// would not reproduce. Nodes are visited in dependency order and each
/// mismatch is pinned before dependents are evaluated, so dependents that
/// follow from a pinned value are not reported.
fn minimal_color_overrides(tokens: &ThemeTokens, seed: &BTreeMap<String, Color>) -> BTreeMap<String, Color> {
    let mut rules = default_rules(tokens.appearance).clone();
    for key in tokens.syntax.keys() {
        let name = format!("syntax.{key}");
        rules.entry(name).or_insert_with(|| Expr::Ref("syntax.variable".into()));
    }
    for (name, c) in seed {
        rules.insert(name.clone(), Expr::Lit(*c));
    }
    // As `resolve::build` does, judged on a resolver of its own: the walk below must
    // see each value only after what it depends on is settled.
    if let Some(amber) = dark_yellow_warning(tokens.appearance)
        && !seed.contains_key("status.warning")
        && Resolver::new(&NO_RULES, rules.clone(), tokens.terminal.named()).yellow_is_dark()
    {
        rules.insert("status.warning".into(), amber.clone());
    }
    let order = topo_order(&rules);
    let mut resolver = Resolver::new(&NO_RULES, rules, tokens.terminal.named());
    let mut out = BTreeMap::new();
    for name in order {
        let Some(actual) = tokens.get(&name) else { continue };
        // Rules are built-in or literal, so resolution cannot fail; treat a
        // failure as "differs" to stay lossless.
        let expected = resolver.color(&name).ok();
        if expected.map(Color::to_bytes) != Some(actual.to_bytes()) {
            // Dependents are judged on what the file will hold: a readable() on a
            // background one bit away can land one bit away too.
            resolver.values.insert(name.clone(), actual.quantized());
            out.insert(name, actual);
        }
    }
    out
}

/// Every node with a rule, each after the nodes it refers to (depth-first, in
/// name order). Iterative, so its stack use does not grow with the chains.
fn topo_order(rules: &BTreeMap<String, Expr>) -> Vec<String> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    // Nodes whose dependencies are being visited, with the ones still to visit.
    let mut stack: Vec<(&str, std::vec::IntoIter<&str>)> = Vec::new();
    let enter = |name, seen: &mut BTreeSet<_>, stack: &mut Vec<_>| {
        if seen.insert(name)
            && let Some((name, e)) = rules.get_key_value(name)
        {
            let mut deps = Vec::new();
            e.refs(&mut deps);
            stack.push((name.as_str(), deps.into_iter()));
        }
    };
    for root in rules.keys() {
        enter(root.as_str(), &mut seen, &mut stack);
        while let Some((name, deps)) = stack.last_mut() {
            match deps.next() {
                Some(dep) => enter(dep, &mut seen, &mut stack),
                None => {
                    out.push(name.to_string());
                    stack.pop();
                }
            }
        }
    }
    out
}

impl MetricsFile {
    /// Fields of `m` that differ from `base` (all fields without a base, except
    /// a `radius_chip` that its own derivation would give anyway).
    pub(super) fn diff(m: &Metrics, base: Option<&Metrics>) -> Self {
        let pick = |v: f32, b: Option<f32>| (b != Some(v)).then_some(v);
        let b = base;
        Self {
            radius_sm: pick(m.radius_sm, b.map(|b| b.radius_sm)),
            // A standalone file re-derives chips from `radius_sm`, so it only
            // needs them when they differ.
            radius_chip: pick(m.radius_chip, b.map_or(Some(m.radius_sm), |b| Some(b.radius_chip))),
            radius: pick(m.radius, b.map(|b| b.radius)),
            radius_lg: pick(m.radius_lg, b.map(|b| b.radius_lg)),
            spacing: pick(m.spacing, b.map(|b| b.spacing)),
            ui_font_family: m
                .ui_font_family
                .clone()
                .filter(|f| b.and_then(|b| b.ui_font_family.as_ref()) != Some(f)),
            ui_font_size: pick(m.ui_font_size, b.map(|b| b.ui_font_size)),
            border_width: pick(m.border_width, b.map(|b| b.border_width)),
            // Like the chips' radius: a standalone file re-derives it from `border_width`.
            divider_width: pick(m.divider_width, b.map_or(Some(m.border_width), |b| Some(b.divider_width))),
            focus_ring_width: pick(m.focus_ring_width, b.map(|b| b.focus_ring_width)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::topo_order;
    use crate::theme::tokens::expr::{Expr, parse_expr};
    use crate::theme::tokens::rules::default_rules;
    use crate::theme::tokens::test_support::{builtin, hex, no_lookup, resolve_str, snapshot};
    use crate::theme::tokens::{
        Appearance, MetricsFile, SyntaxOverride, ThemeFile, builtin_tokens, parse_theme_file, resolve,
    };

    #[test]
    fn chip_radius_is_saved_only_when_it_differs() {
        let derived = resolve_str(r#"{"metrics": {"radius_sm": 2}}"#).unwrap();
        assert_eq!(derived.to_theme_file(None).metrics.radius_chip, None);
        let own = resolve_str(r#"{"metrics": {"radius_chip": 9}}"#).unwrap();
        let file = own.to_theme_file(None);
        assert_eq!(file.metrics.radius_chip, Some(9.0));
        let back = resolve(&parse_theme_file(&file.to_json_pretty()).unwrap(), &no_lookup).unwrap();
        assert_eq!(back.metrics, own.metrics);
        let minimal = own.to_theme_file(Some(&builtin("Tokyo Night")));
        assert_eq!(minimal.metrics, MetricsFile { radius_chip: Some(9.0), ..Default::default() });
    }

    #[test]
    fn full_save_round_trips() {
        for base in builtin_tokens() {
            let file = base.to_theme_file(None);
            assert!(file.extends.is_none());
            let json = file.to_json_pretty();
            let back = resolve(&parse_theme_file(&json).unwrap(), &no_lookup).unwrap();
            assert_eq!(snapshot(&back), snapshot(&base), "{}", base.name);
        }
    }

    #[test]
    fn minimal_save_round_trips() {
        let src = r##"{
            "name": "Mine", "extends": "Gruvbox Dark",
            "terminal": {"background": "#101010", "ansi": {"4": "#5588ff"}},
            "ui": {"primary": "#ff9e64", "sidebar.row.selected": "primary / 15%"},
            "syntax": {"keyword": {"color": "#bb9af7", "italic": true}, "string": {"bold": true}},
            "metrics": {"radius": 3, "ui_font_family": "Inter"}
        }"##;
        let mine = resolve_str(src).unwrap();
        let base = builtin("Gruvbox Dark");
        let file = mine.to_theme_file(Some(&base));
        assert_eq!(file.extends.as_deref(), Some("Gruvbox Dark"));
        assert_eq!(file.name.as_deref(), Some("Mine"));
        assert_eq!(file.appearance, None);
        // Only what was changed is saved, not the tokens derived from it.
        assert_eq!(file.ui.keys().collect::<Vec<_>>(), ["primary", "sidebar.row.selected"]);
        assert_eq!(file.ui["sidebar.row.selected"], "#ff9e6426");
        assert_eq!(file.terminal.background.as_deref(), Some("#101010"));
        assert_eq!(file.terminal.ansi.keys().collect::<Vec<_>>(), ["4"]);
        assert!(file.terminal.foreground.is_none());
        assert_eq!(file.syntax.keys().collect::<Vec<_>>(), ["keyword", "string"]);
        assert_eq!(
            file.syntax["keyword"],
            SyntaxOverride::Style { color: Some("#bb9af7".into()), bold: Some(false), italic: Some(true) }
        );
        assert_eq!(
            file.syntax["string"],
            SyntaxOverride::Style { color: None, bold: Some(true), italic: Some(false) }
        );
        assert_eq!(
            file.metrics,
            MetricsFile { radius: Some(3.0), ui_font_family: Some("Inter".into()), ..Default::default() }
        );

        let json = file.to_json_pretty();
        let back = resolve(&parse_theme_file(&json).unwrap(), &no_lookup).unwrap();
        assert_eq!(snapshot(&back), snapshot(&mine));

        // Unchanged tokens save as an (almost) empty file.
        let same = base.to_theme_file(Some(&base));
        assert_eq!(
            same,
            ThemeFile {
                name: Some(base.name.clone()),
                extends: Some(base.name.clone()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn minimal_save_on_top_of_user_base() {
        let parent_src =
            r##"{"name": "P", "extends": "Nord", "ui": {"primary": "#ff00ff"}, "metrics": {"radius": 9}}"##;
        let parent_file = parse_theme_file(parent_src).unwrap();
        let parent = resolve(&parent_file, &no_lookup).unwrap();
        let lookup = |n: &str| (n == "P").then(|| parent_file.clone());

        // Child edited in the Studio via set(): one token and one ANSI color.
        let mut child = parent.clone();
        child.name = "Kid".into();
        child.set("status.info", hex("#00ffff"));
        child.set("terminal.ansi.2", hex("#00ff00"));
        let file = child.to_theme_file(Some(&parent));
        assert_eq!(file.extends.as_deref(), Some("P"));
        // primary is inherited from P, so it is not repeated. status.success
        // was not re-derived from the new green, so it is pinned to keep the
        // round trip exact.
        assert!(!file.ui.contains_key("primary"));
        assert!(file.ui.contains_key("status.info"));
        assert!(file.metrics.is_empty());
        let back = resolve(&file, &lookup).unwrap();
        assert_eq!(snapshot(&back), snapshot(&child));

        // Light appearance on a dark background is made explicit.
        let mut odd = builtin("Nord");
        odd.appearance = Appearance::Light;
        assert_eq!(odd.to_theme_file(Some(&builtin("Nord"))).appearance, Some(Appearance::Light));
    }

    #[test]
    fn json_output_shape() {
        let file = builtin("Nord").to_theme_file(Some(&builtin("Tokyo Night")));
        let json = file.to_json_pretty();
        assert!(json.contains("\"extends\": \"Tokyo Night\""));
        assert!(json.ends_with("}\n"));
        let empty = ThemeFile::default().to_json_pretty();
        assert_eq!(empty, "{}\n");
    }

    /// The recursive version `topo_order` replaced, as the reference for its order.
    fn recursive_topo_order(rules: &BTreeMap<String, Expr>) -> Vec<String> {
        fn visit(
            n: &str,
            rules: &BTreeMap<String, Expr>,
            seen: &mut BTreeSet<String>,
            out: &mut Vec<String>,
        ) {
            if !seen.insert(n.to_string()) {
                return;
            }
            if let Some(e) = rules.get(n) {
                let mut deps = Vec::new();
                e.refs(&mut deps);
                for d in deps {
                    visit(d, rules, seen, out);
                }
                out.push(n.to_string());
            }
        }
        let (mut seen, mut out) = (BTreeSet::new(), Vec::new());
        for n in rules.keys() {
            visit(n, rules, &mut seen, &mut out);
        }
        out
    }

    #[test]
    fn topo_order_is_unchanged_and_needs_no_deep_stack() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            let mut rules = default_rules(appearance).clone();
            assert_eq!(topo_order(&rules), recursive_topo_order(&rules));
            // Unknown references, literals and a cycle are visited the same way too.
            for (name, src) in
                [("a", "mix(b, primary, 10%)"), ("b", "missing"), ("c", "#fff"), ("d", "e"), ("e", "d")]
            {
                rules.insert(name.into(), parse_expr(src).unwrap());
            }
            assert_eq!(topo_order(&rules), recursive_topo_order(&rules));
        }
        let chain: BTreeMap<String, Expr> =
            (0..100_000).map(|i| (format!("n{i:06}"), Expr::Ref(format!("n{:06}", i + 1)))).collect();
        let order = crate::theme::tokens::test_support::on_small_stack(move || topo_order(&chain));
        assert_eq!(order.len(), 100_000);
        assert_eq!(order[0], "n099999");
    }
}
