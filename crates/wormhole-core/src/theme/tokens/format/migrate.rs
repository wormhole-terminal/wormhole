//! Migrating older files, in memory, every time one is read.
//!
//! The file on disk is never touched: a version 0 file stays as its author wrote it until
//! they save it in the Studio. What a migration produces is the resolver's one model, so a
//! version 0 and a version 1 file resolve the same way.
//!
//! The steps are a list, one per change of the format, and a step that has shipped is never
//! edited (as `storage::store`'s `MIGRATIONS` are not): a later change adds a step. What the
//! `0 -> 1` step does to a file already in the resolver's model:
//!
//! - The key renames of version 1 (`KEY_TABLE`) need no step here: the resolver's names are
//!   the old ones, so version 0 is already in them, and the version 1 reader and writer
//!   translate (`format::v1`). They are the same table, so the migration and the docs agree.
//! - A token that was replaced (`accent` -> `primary`) is rewritten, in a `ui` key and in
//!   every expression that refers to it. One with no successor is dropped, and the notice
//!   says so: a theme with an old token loads, with that value missing, not fails.

use std::collections::BTreeMap;

use super::refs::{Rename, rewrite_refs};
use crate::theme::tokens::file::{SyntaxOverride, ThemeFile};

/// What replaced a token.
#[derive(Clone, Copy, Debug)]
pub(super) enum Successor {
    /// Another token of the same job: its key and its references change.
    Token(&'static str),
    /// Only an expression stands in for it (a tint): references change, a `ui` key cannot.
    Expression(&'static str),
    /// Nothing: the value is dropped. Says what to do instead. No token is in this state today;
    /// the next one removed without a successor is.
    #[allow(dead_code, reason = "constructed by the next removal and by the tests")]
    Nothing(&'static str),
}

/// The tokens older files may name, and what took their place. Kept beside
/// `rules::replaced_by` (a test checks that every name there is here).
pub(super) const REPLACED_TOKENS: &[(&str, Successor)] = &[
    ("accent", Successor::Token("primary")),
    ("accent.hover", Successor::Token("primary.hover")),
    ("text.on_accent", Successor::Token("text.on_primary")),
    ("accent.muted", Successor::Expression("primary / 20%")),
    ("button.primary.background", Successor::Expression("primary")),
    ("button.primary.hover", Successor::Expression("primary.hover")),
    ("button.primary.text", Successor::Expression("text.on_primary")),
];

fn successor(name: &str) -> Option<Successor> {
    REPLACED_TOKENS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// One step of the format's history.
pub struct Step {
    /// The version it reads.
    pub from: u32,
    /// What it does, for the docs.
    pub about: &'static str,
    apply: fn(&mut ThemeFile, &mut Vec<String>),
}

/// Every step, oldest first. Never edit one that has shipped; add the next.
pub const STEPS: &[Step] = &[Step {
    from: 0,
    about: "replaced tokens are rewritten (also inside expressions); ones with no successor are dropped with a notice",
    apply: replaced_tokens,
}];

/// Run the steps from version `from` on, returning the notices they raised.
pub(super) fn run(file: &mut ThemeFile, from: u32) -> Vec<String> {
    let mut notices = Vec::new();
    for step in STEPS.iter().filter(|s| s.from >= from) {
        (step.apply)(file, &mut notices);
    }
    notices
}

fn replaced_tokens(file: &mut ThemeFile, notices: &mut Vec<String>) {
    layers(&mut file.ui, &mut file.syntax, "", notices);
    for v in &mut file.variants {
        let at = format!("variants {:?}: ", v.name);
        layers(&mut v.ui, &mut v.syntax, &at, notices);
    }
}

fn layers(
    ui: &mut BTreeMap<String, String>,
    syntax: &mut BTreeMap<String, SyntaxOverride>,
    at: &str,
    notices: &mut Vec<String>,
) {
    let old = std::mem::take(ui);
    // Own tokens first, so a replaced key never overwrites a value written under the new name.
    let (replaced, current): (Vec<_>, Vec<_>) = old.into_iter().partition(|(k, _)| successor(k).is_some());
    for (key, value) in current {
        if let Some(value) = expression(&value, &format!("{at}ui.{key}"), notices) {
            ui.insert(key, value);
        }
    }
    for (key, value) in replaced {
        match successor(&key) {
            Some(Successor::Token(now)) if ui.contains_key(now) => {
                notices.push(format!("{at}ui.{key}: `{now}` is set too, which wins; this value was skipped"));
            }
            Some(Successor::Token(now)) => {
                if let Some(value) = expression(&value, &format!("{at}ui.{key}"), notices) {
                    ui.insert(now.to_string(), value);
                }
            }
            Some(Successor::Expression(now)) => {
                notices.push(format!(
                    "{at}ui.{key}: no longer a token; skipped. Set `{now}` where this was used"
                ));
            }
            Some(Successor::Nothing(why)) => notices.push(format!("{at}ui.{key}: skipped. {why}")),
            None => {}
        }
    }
    for (key, style) in syntax.iter_mut() {
        let ctx = format!("{at}syntax.{key}");
        match style {
            SyntaxOverride::Color(c) => match expression(c, &ctx, notices) {
                Some(new) => *c = new,
                None => *style = SyntaxOverride::Style { color: None, bold: None, italic: None },
            },
            SyntaxOverride::Style { color, .. } => {
                if let Some(c) = color {
                    match expression(c, &ctx, notices) {
                        Some(new) => *c = new,
                        None => *color = None,
                    }
                }
            }
        }
    }
}

/// `src` with replaced tokens swapped for their successors; `None` (and a notice) when it
/// refers to one that has none.
fn expression(src: &str, ctx: &str, notices: &mut Vec<String>) -> Option<String> {
    expression_with(src, ctx, notices, &successor)
}

fn expression_with(
    src: &str,
    ctx: &str,
    notices: &mut Vec<String>,
    lookup: &dyn Fn(&str) -> Option<Successor>,
) -> Option<String> {
    let result = rewrite_refs(src, &|name| match lookup(name) {
        None => Rename::Keep,
        Some(Successor::Token(now)) => Rename::To(now.to_string()),
        Some(Successor::Expression(e)) if e.contains(' ') => Rename::To(format!("({e})")),
        Some(Successor::Expression(e)) => Rename::To(e.to_string()),
        Some(Successor::Nothing(why)) => {
            Rename::Refuse(format!("{ctx}: refers to `{name}`, which is gone. {why}"))
        }
    });
    match result {
        Ok(new) => Some(new),
        Err(why) => {
            notices.push(why);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::{parse_theme_file, rules::replaced_by};

    #[test]
    fn every_replaced_token_is_known_to_the_resolver_too() {
        for (name, _) in REPLACED_TOKENS {
            assert!(replaced_by(name).is_some(), "{name}");
        }
    }

    #[test]
    fn steps_are_in_order() {
        assert!(STEPS.windows(2).all(|w| w[0].from < w[1].from));
    }

    #[test]
    fn replaced_keys_and_expressions_are_rewritten() {
        let f = parse_theme_file(
            r##"{"ui": {"accent": "#f00", "accent.hover": "mix(accent, #fff, 10%)", "border": "accent.muted / 50%",
                        "text.on_accent": "text_on(accent)"}}"##,
        )
        .unwrap();
        assert_eq!(f.ui["primary"], "#f00");
        assert_eq!(f.ui["primary.hover"], "mix(primary, #fff, 10%)");
        assert_eq!(f.ui["border"], "(primary / 20%) / 50%");
        assert_eq!(f.ui["text.on_primary"], "text_on(primary)");
        assert!(!f.ui.contains_key("accent") && f.read.notices.is_empty());
    }

    #[test]
    fn a_value_under_the_new_name_beats_one_under_the_old() {
        let f = parse_theme_file(r##"{"ui": {"accent": "#f00", "primary": "#0f0"}}"##).unwrap();
        assert_eq!(f.ui["primary"], "#0f0");
        assert_eq!(f.read.notices.len(), 1, "{:?}", f.read.notices);
    }

    #[test]
    fn a_token_that_is_only_an_expression_is_dropped_with_a_notice() {
        let f =
            parse_theme_file(r##"{"ui": {"accent.muted": "#123", "button.primary.text": "#fff"}}"##).unwrap();
        assert!(f.ui.is_empty());
        assert_eq!(f.read.notices.len(), 2);
        assert!(f.read.notices[0].contains("ui.accent.muted"), "{:?}", f.read.notices);
    }

    #[test]
    fn a_token_with_no_successor_drops_the_value_not_the_file() {
        let lookup = |n: &str| (n == "gone").then_some(Successor::Nothing("Use `border`."));
        let mut notices = Vec::new();
        assert_eq!(expression_with("mix(gone, #fff, 5%)", "ui.x", &mut notices, &lookup), None);
        assert_eq!(notices, ["ui.x: refers to `gone`, which is gone. Use `border`."]);
        assert_eq!(
            expression_with("mix(border, #fff, 5%)", "ui.x", &mut notices, &lookup).as_deref(),
            Some("mix(border, #fff, 5%)")
        );
    }

    #[test]
    fn syntax_values_and_variants_are_rewritten_too() {
        let f = parse_theme_file(
            r##"{"syntax": {"keyword": "accent", "string": {"color": "accent.hover", "bold": true}},
                 "variants": [{"name": "A", "ui": {"accent": "#abc", "border": "accent / 5%"}}]}"##,
        )
        .unwrap();
        assert_eq!(f.syntax["keyword"], SyntaxOverride::Color("primary".into()));
        assert!(
            matches!(&f.syntax["string"], SyntaxOverride::Style { color: Some(c), bold: Some(true), .. } if c == "primary.hover")
        );
        assert_eq!(f.variants[0].ui["primary"], "#abc");
        assert_eq!(f.variants[0].ui["border"], "primary / 5%");
    }

    #[test]
    fn a_file_without_old_tokens_is_unchanged() {
        let text = r##"{"extends": "Nord", "ui": {"primary": "#f00", "border": "mix(primary, #fff, 20%)"}}"##;
        let f = parse_theme_file(text).unwrap();
        assert_eq!(f.ui["border"], "mix(primary, #fff, 20%)");
        assert!(f.read.notices.is_empty());
    }
}
