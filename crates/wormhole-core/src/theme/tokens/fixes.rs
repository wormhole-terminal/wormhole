//! Contrast as the Theme Studio shows it: every pair a token is part of with its ratio,
//! and for each pair that falls short a fix written as an expression, which the person
//! applies or not (the Studio never changes a value by itself).

use std::collections::{BTreeMap, BTreeSet};

use super::contrast::{CONTRAST_PAIRS, ContrastPair};
use super::expr::{Expr, parse_expr};
use super::resolve::READABLE_MARGIN;
use super::rules::{default_rule, default_rules};
use super::theme_tokens::ThemeTokens;
use crate::color::Color;

/// One text-on-background pair of [`CONTRAST_PAIRS`], measured.
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastCheck {
    pub fg: &'static str,
    pub bg: &'static str,
    /// What a translucent `bg` is judged over.
    pub underlay: &'static str,
    pub ratio: f32,
    pub min: f32,
}

impl ContrastCheck {
    pub fn passes(&self) -> bool {
        self.ratio >= self.min
    }
}

/// A suggested new value for a token: `expression` is what the Studio would put in its
/// field, `label` what the button says ("Lighten to 3:1").
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastFix {
    pub token: String,
    pub expression: String,
    pub label: String,
}

/// A pair that falls short, and what to do about it.
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastIssue {
    pub check: ContrastCheck,
    /// The fix, unless the pair is the terminal palette's own or `follows` covers it.
    pub fix: Option<ContrastFix>,
    /// The failing token this pair's text follows (its rule is a plain reference to it):
    /// that token's fix clears this pair too.
    pub follows: Option<&'static str>,
}

impl ThemeTokens {
    fn measure(&self, pair: &ContrastPair) -> Option<ContrastCheck> {
        let bg = self.opaque(pair.bg, pair.underlay)?;
        let fg = self.get(pair.fg)?.over(bg);
        Some(ContrastCheck {
            fg: pair.fg,
            bg: pair.bg,
            underlay: pair.underlay,
            ratio: Color::contrast_ratio(fg, bg),
            min: pair.min,
        })
    }

    /// Every pair of [`CONTRAST_PAIRS`] that `name` is part of, as the text or as the
    /// background, measured as [`ThemeTokens::contrast_warnings`] measures them.
    pub fn contrast_checks_of(&self, name: &str) -> Vec<ContrastCheck> {
        CONTRAST_PAIRS
            .iter()
            .filter(|p| p.fg == name || p.bg == name)
            .filter_map(|p| self.measure(p))
            .collect()
    }
}

/// What a token is made of: the value typed for it, the theme's own override, or its rule.
fn source(tokens: &ThemeTokens, typed: &BTreeMap<String, String>, name: &str) -> Option<String> {
    typed
        .get(name)
        .cloned()
        .or_else(|| tokens.recipe.overrides.get(name).map(Expr::to_string))
        .or_else(|| default_rule(name, tokens.appearance))
}

/// Whether `name`'s value is computed from `target`, directly or through other tokens.
fn depends_on(tokens: &ThemeTokens, name: &str, target: &str) -> bool {
    let rules = default_rules(tokens.appearance);
    let mut seen = BTreeSet::new();
    let mut todo = vec![name.to_string()];
    while let Some(node) = todo.pop() {
        if node == target {
            return true;
        }
        if !seen.insert(node.clone()) {
            continue;
        }
        let Some(rule) = tokens.recipe.overrides.get(&node).or_else(|| rules.get(&node)) else { continue };
        let mut refs = Vec::new();
        rule.refs(&mut refs);
        todo.extend(refs.into_iter().map(str::to_string));
    }
    false
}

/// A contrast minimum as the Studio writes it: 3:1, 4.5:1.
fn ratio_text(min: f32) -> String {
    format!("{min}:1")
}

/// Cuts a long expression for a button's label.
fn shortened(src: &str) -> String {
    const LIMIT: usize = 24;
    match src.char_indices().nth(LIMIT - 1) {
        Some((cut, _)) => format!("{}…", &src[..cut]),
        None => src.to_string(),
    }
}

/// Pairs that fall short, in [`CONTRAST_PAIRS`] order, each with a fix written as an
/// expression. `typed` holds the draft's own UI values exactly as typed (its
/// `ThemeFile::ui`), so a fix wraps what the person wrote rather than a resolved hex.
///
/// The fix raises the pair's text: `readable(<what it is now>, <backgrounds>, <min>)`,
/// over every background it falls short on. Text that only names another failing token
/// (`agent.working` is `primary`) and needs no more contrast than that token is left to
/// it: that token's fix takes its backgrounds too, so one change clears both. The
/// terminal palette's own pair gets no fix: it is the palette, not the interface.
pub fn contrast_issues(tokens: &ThemeTokens, typed: &BTreeMap<String, String>) -> Vec<ContrastIssue> {
    let failing: Vec<ContrastCheck> =
        CONTRAST_PAIRS.iter().filter_map(|p| tokens.measure(p)).filter(|c| !c.passes()).collect();
    let fixable: Vec<&'static str> = {
        let mut names: Vec<&'static str> =
            failing.iter().map(|c| c.fg).filter(|fg| !fg.starts_with("terminal.")).collect();
        names.dedup();
        names
    };
    let own_min = |name: &str| failing.iter().filter(|c| c.fg == name).map(|c| c.min).fold(0.0, f32::max);
    // The failing token each one names outright, when that one can take its pairs.
    let named = |name: &str, blocked: &BTreeSet<&str>| -> Option<&'static str> {
        let Ok(Expr::Ref(target)) = parse_expr(&source(tokens, typed, name)?) else { return None };
        let target = fixable.iter().copied().find(|f| *f == target && !blocked.contains(f))?;
        (target != name && own_min(name) <= own_min(target)).then_some(target)
    };
    let leader = |name: &'static str, blocked: &BTreeSet<&str>| {
        let mut at = name;
        for _ in 0..fixable.len() {
            match named(at, blocked) {
                Some(next) => at = next,
                None => break,
            }
        }
        at
    };

    // A token that can't be fixed (each background is made from it) can't take its
    // followers' pairs either: they get fixes of their own.
    let mut blocked = BTreeSet::new();
    let fixes = loop {
        let mut fixes: BTreeMap<&str, Option<ContrastFix>> = BTreeMap::new();
        for &name in &fixable {
            let root = leader(name, &blocked);
            if !fixes.contains_key(root) {
                let group: Vec<&ContrastCheck> = failing
                    .iter()
                    .filter(|c| fixable.contains(&c.fg) && leader(c.fg, &blocked) == root)
                    .collect();
                fixes.insert(root, fix_for(tokens, typed, root, &group));
            }
        }
        let stuck: Vec<&'static str> = fixes
            .iter()
            .filter(|&(&root, fix)| {
                fix.is_none()
                    && !blocked.contains(root)
                    && fixable.iter().any(|&f| f != root && leader(f, &blocked) == root)
            })
            .map(|(&root, _)| root)
            .collect();
        if stuck.is_empty() {
            break fixes;
        }
        blocked.extend(stuck);
    };

    failing
        .iter()
        .map(|check| {
            if !fixable.contains(&check.fg) {
                return ContrastIssue { check: check.clone(), fix: None, follows: None };
            }
            match named(check.fg, &blocked) {
                Some(follows) => ContrastIssue { check: check.clone(), fix: None, follows: Some(follows) },
                None => ContrastIssue {
                    check: check.clone(),
                    fix: fixes.get(check.fg).cloned().flatten(),
                    follows: None,
                },
            }
        })
        .collect()
}

/// The fix for `name`, raising it over the backgrounds of every pair in `group` (its own
/// and its followers'). None when every one of them is made from `name` itself, which
/// `readable()` would turn into a cycle.
fn fix_for(
    tokens: &ThemeTokens,
    typed: &BTreeMap<String, String>,
    name: &str,
    group: &[&ContrastCheck],
) -> Option<ContrastFix> {
    let src = source(tokens, typed, name)?;
    let mut backgrounds: Vec<(String, Color)> = Vec::new();
    for check in group {
        if depends_on(tokens, check.bg, name) || depends_on(tokens, check.underlay, name) {
            continue;
        }
        let translucent = tokens.get(check.bg).is_some_and(|c| c.a < 1.0);
        let term = if translucent && check.underlay != "surface.window" {
            format!("over({}, {})", check.bg, check.underlay)
        } else {
            check.bg.to_string()
        };
        if backgrounds.iter().all(|(t, _)| *t != term) {
            backgrounds.push((term, tokens.opaque(check.bg, check.underlay)?));
        }
    }
    if backgrounds.is_empty() {
        return None;
    }
    let min = group.iter().map(|c| c.min).fold(0.0, f32::max);
    let terms: Vec<&str> = backgrounds.iter().map(|(t, _)| t.as_str()).collect();
    let expression = format!("readable({src}, {}, {min})", terms.join(", "));
    let label = match parse_expr(&src) {
        // A color typed as is: say which way it moves.
        Ok(Expr::Lit(old)) => {
            let new = backgrounds.iter().fold(old, |c, (_, bg)| c.readable_on(*bg, min + READABLE_MARGIN));
            let way = if new.luminance() >= old.luminance() { "Lighten" } else { "Darken" };
            format!("{way} to {}", ratio_text(min))
        }
        _ => format!("Use readable({}, …, {min})", shortened(&src)),
    };
    Some(ContrastFix { token: name.to_string(), expression, label })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::contrast_issues;
    use crate::tokens::test_support::builtin;
    use crate::tokens::{ThemeFile, ThemeTokens, resolve};

    fn draft(base: &str, ui: &[(&str, &str)]) -> ThemeFile {
        let ui: BTreeMap<String, String> = ui.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        ThemeFile { name: Some("Draft".into()), extends: Some(base.into()), ui, ..Default::default() }
    }

    fn tokens(file: &ThemeFile) -> ThemeTokens {
        resolve(file, &|_| None).unwrap()
    }

    /// Applies every fix, as clicking each one would, and resolves again.
    fn fixed(mut file: ThemeFile) -> ThemeTokens {
        for issue in contrast_issues(&tokens(&file), &file.ui) {
            if let Some(fix) = issue.fix {
                file.ui.insert(fix.token, fix.expression);
            }
        }
        tokens(&file)
    }

    fn interface_warnings(t: &ThemeTokens) -> Vec<(String, String, f32)> {
        t.contrast_warnings().into_iter().filter(|w| !w.0.starts_with("terminal.")).collect()
    }

    /// A dark primary (#3d59a1) on Tokyo Night. The primary itself gets a fix; the
    /// working status, which is the primary, follows it; one click clears both.
    #[test]
    fn a_dark_primary_on_tokyo_night_is_fixed_in_one_step() {
        let file = draft("Tokyo Night", &[("primary", "#3d59a1")]);
        let issues = contrast_issues(&tokens(&file), &file.ui);
        let primary = issues.iter().find(|i| i.check.fg == "primary").expect("primary falls short");
        assert!(primary.check.ratio < 3.0 && primary.check.min == 3.0, "{primary:?}");
        let fix = primary.fix.as_ref().expect("a fix");
        assert_eq!((fix.token.as_str(), fix.label.as_str()), ("primary", "Lighten to 3:1"));
        assert!(fix.expression.starts_with("readable(#3d59a1, surface.window"), "{}", fix.expression);
        let working = issues.iter().find(|i| i.check.fg == "agent.working").expect("the working status");
        assert_eq!((working.follows, working.fix.as_ref()), (Some("primary"), None));
        // The follower's background is in the primary's fix.
        assert!(fix.expression.contains("sidebar.background"), "{}", fix.expression);
        assert!(issues.iter().all(|i| i.fix.is_some() || i.follows.is_some()), "{issues:?}");
        assert_eq!(interface_warnings(&fixed(file)), []);
    }

    /// Text that names the primary but needs more than it (a link, 4.5:1) gets a fix of its
    /// own, written round what it names.
    #[test]
    fn a_follower_that_needs_more_contrast_gets_its_own_fix() {
        let file = draft("Tokyo Night", &[("primary", "#3d59a1"), ("text.link", "primary")]);
        let issues = contrast_issues(&tokens(&file), &file.ui);
        let link = issues.iter().find(|i| i.check.fg == "text.link").expect("the link falls short");
        assert_eq!(link.follows, None);
        let fix = link.fix.as_ref().expect("a fix");
        assert_eq!(fix.label, "Use readable(primary, …, 4.5)");
        assert_eq!(fix.expression, "readable(primary, surface.window, 4.5)");
        assert_eq!(interface_warnings(&fixed(file)), []);
    }

    #[test]
    fn a_pale_primary_on_a_light_theme_is_darkened() {
        let file = draft("GitHub Light", &[("primary", "#b8d4ff")]);
        let issues = contrast_issues(&tokens(&file), &file.ui);
        let fix = issues.iter().find(|i| i.check.fg == "primary").and_then(|i| i.fix.clone()).expect("a fix");
        assert_eq!(fix.label, "Darken to 3:1");
        assert_eq!(interface_warnings(&fixed(file)), []);
    }

    /// A long rule is cut in the label, never in the expression.
    #[test]
    fn a_derived_token_is_wrapped_in_its_rule() {
        let file = draft("Tokyo Night", &[("text.muted", "mix(text.primary, surface.window, 80%)")]);
        let issues = contrast_issues(&tokens(&file), &file.ui);
        let muted = issues.iter().find(|i| i.check.fg == "text.muted").expect("muted falls short");
        let fix = muted.fix.as_ref().expect("a fix");
        assert_eq!(fix.label, "Use readable(mix(text.primary, surfa…, …, 4.5)");
        assert!(fix.expression.starts_with("readable(mix(text.primary, surface.window, 80%), "));
        assert_eq!(interface_warnings(&fixed(file)), []);
    }

    #[test]
    fn the_terminal_palette_is_not_fixed_from_the_interface() {
        let file = ThemeFile {
            terminal: crate::tokens::TerminalFile {
                foreground: Some("#303030".into()),
                background: Some("#202020".into()),
                ..Default::default()
            },
            ..draft("Tokyo Night", &[])
        };
        let issues = contrast_issues(&tokens(&file), &file.ui);
        let terminal = issues.iter().find(|i| i.check.fg == "terminal.foreground").expect("the palette");
        assert_eq!((terminal.fix.as_ref(), terminal.follows), (None, None));
    }

    #[test]
    fn a_tokens_checks_list_it_as_text_and_as_background() {
        let t = builtin("Tokyo Night");
        let primary = t.contrast_checks_of("primary");
        assert!(primary.iter().any(|c| c.fg == "primary" && c.bg == "surface.window"));
        assert!(primary.iter().any(|c| c.fg == "text.on_primary" && c.bg == "primary"));
        let fill = t.contrast_checks_of("sidebar.row.selected");
        assert!(fill.iter().any(|c| c.fg == "sidebar.row.selected.text"));
        assert!(fill.iter().all(|c| c.passes()), "{fill:?}");
        assert!(t.contrast_checks_of("not.a.token").is_empty());
        assert!(contrast_issues(&t, &BTreeMap::new()).is_empty());
    }
}
