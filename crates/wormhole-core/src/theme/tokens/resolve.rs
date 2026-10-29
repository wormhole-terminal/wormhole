//! Evaluating rules: a memoizing resolver over named nodes, and `build`,
//! which turns a palette plus overrides into finished tokens.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::catalog::UI_TOKENS;
use super::expr::Expr;
use super::high_contrast::high_contrast_rules;
use super::model::{Appearance, Metrics, SyntaxStyle, TerminalColors};
use super::rules::{dark_yellow_warning, default_rules, rule_flags};
use super::theme_tokens::{Recipe, ThemeTokens};
use crate::storage::jsonc::suggest::did_you_mean;
use crate::theme::tokens::color::Color;

/// Longest chain of references (`a` refers to `b`, which refers to `c`, ...)
/// a theme may build; the built-in rules stay under 16. Each link is a level
/// of recursion, so without a bound a long chain in an untrusted theme file
/// overflows the stack and aborts the app.
pub(super) const MAX_REF_DEPTH: usize = 64;

/// Evaluation steps that may be in progress at once, across a reference chain
/// and the expressions along it (each of which may nest 32 deep on its own).
/// This is what bounds the stack; the built-in rules (chains under 16 links,
/// rules under 6 levels) stay well inside it.
const MAX_EVAL_DEPTH: usize = 256;

/// How far past its ratio `readable()` aims. A color that only just reaches 4.5:1 can
/// measure 4.49:1 once it is rounded to 8 bits per channel (or shown rounded to two
/// decimals), which reads as a failure in every contrast checker.
pub(super) const READABLE_MARGIN: f32 = 0.03;

/// No rules: the base for resolvers that bring all of theirs.
pub(super) static NO_RULES: BTreeMap<String, Expr> = BTreeMap::new();

/// Memoizing evaluator over named nodes, with cycle detection.
pub(super) struct Resolver {
    /// Shared rules (the cached defaults); `rules` win over them.
    base: &'static BTreeMap<String, Expr>,
    rules: BTreeMap<String, Rc<Expr>>,
    pub(super) values: BTreeMap<String, Color>,
    stack: Vec<String>,
    /// `eval` calls in progress.
    depth: usize,
    /// Extra hint appended to unknown-name errors.
    pub(super) scope_hint: &'static str,
}

impl Resolver {
    /// `rules` on top of `base`; `values` for names neither has a rule for.
    pub(super) fn new(
        base: &'static BTreeMap<String, Expr>,
        rules: BTreeMap<String, Expr>,
        values: Vec<(String, Color)>,
    ) -> Self {
        let values =
            values.into_iter().filter(|(n, _)| !rules.contains_key(n) && !base.contains_key(n)).collect();
        let rules = rules.into_iter().map(|(n, e)| (n, Rc::new(e))).collect();
        Self { base, rules, values, stack: Vec::new(), depth: 0, scope_hint: "" }
    }

    /// Every node with a rule, sorted.
    pub(super) fn names(&self) -> BTreeSet<&str> {
        self.base.keys().chain(self.rules.keys()).map(String::as_str).collect()
    }

    pub(super) fn color(&mut self, name: &str) -> Result<Color, String> {
        if let Some(c) = self.values.get(name) {
            return Ok(*c);
        }
        if let Some(i) = self.stack.iter().position(|n| n == name) {
            let path: Vec<&str> = self.stack[i..].iter().map(String::as_str).chain([name]).collect();
            return Err(format!("reference cycle: {}", path.join(" -> ")));
        }
        // Held outside `self` (a shared handle or a static) so evaluating can borrow `self`.
        let (owned, shared) = match self.rules.get(name) {
            Some(rule) => (Some(Rc::clone(rule)), None),
            None => (None, self.base.get(name)),
        };
        let Some(expr) = owned.as_deref().or(shared) else {
            let known =
                self.base.keys().chain(self.rules.keys()).chain(self.values.keys()).map(String::as_str);
            return Err(format!("unknown color `{name}`{}{}", did_you_mean(name, known), self.scope_hint));
        };
        if self.stack.len() >= MAX_REF_DEPTH {
            let first = self.stack.first().map_or(name, String::as_str);
            return Err(format!("reference chain is longer than {MAX_REF_DEPTH} ({first} -> … -> {name})"));
        }
        self.stack.push(name.to_string());
        let result = self.eval(expr);
        self.stack.pop();
        let c = result?;
        self.values.insert(name.to_string(), c);
        Ok(c)
    }

    /// Whether the palette's yellow is darker than its muted text, so `status.warning`
    /// takes the amber rule (`rules::dark_yellow_warning`). An error counts as no: the
    /// full resolution reports it against the theme's own lines.
    pub(super) fn yellow_is_dark(&mut self) -> bool {
        match (self.color("terminal.ansi.3"), self.color("text.muted")) {
            (Ok(yellow), Ok(muted)) => yellow.luminance() < muted.luminance(),
            _ => false,
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Color, String> {
        if self.depth >= MAX_EVAL_DEPTH {
            let first = self.stack.first().map_or("", String::as_str);
            let last = self.stack.last().map_or("", String::as_str);
            return Err(format!(
                "expressions nest deeper than {MAX_EVAL_DEPTH} levels ({first} -> … -> {last})"
            ));
        }
        self.depth += 1;
        let result = self.eval_step(e);
        self.depth -= 1;
        result
    }

    fn eval_step(&mut self, e: &Expr) -> Result<Color, String> {
        Ok(match e {
            Expr::Lit(c) => *c,
            Expr::Ref(n) => self.color(n)?,
            Expr::Alpha(x, a) => self.eval(x)?.with_alpha(*a),
            Expr::Mix(x, y, t) => self.eval(x)?.mix(self.eval(y)?, *t),
            Expr::Over(x, under) => self.eval(x)?.over(self.eval(under)?),
            Expr::Lighten(x, t) => self.eval(x)?.lighten(*t),
            Expr::Shade(x, t) => {
                let c = self.eval(x)?;
                if c.text_on() == Color::WHITE { c.darken(*t) } else { c.lighten(*t) }
            }
            Expr::TextOn(x) => self.eval(x)?.text_on(),
            Expr::Readable(x, bgs, ratio) => {
                let mut c = self.eval(x)?;
                for bg in bgs {
                    let mut bg = self.eval(bg)?;
                    if bg.a < 1.0 {
                        let window = self.color("surface.window")?.over(Color::BLACK);
                        bg = bg.over(window);
                    }
                    c = c.readable_on(bg, *ratio + READABLE_MARGIN);
                }
                c
            }
        })
    }
}

#[cfg(test)]
thread_local! {
    /// How many times `build` ran on this thread: the expensive part of every
    /// theme resolution, counted so tests can tell when work is skipped.
    pub(super) static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Syntax flag overrides: (bold, italic).
pub(super) type FlagOverrides = BTreeMap<String, (Option<bool>, Option<bool>)>;

/// Evaluate every UI token and syntax color for a final terminal palette.
/// `overrides` replace rules by node name (bare UI names, `syntax.*`).
pub(super) fn build(
    name: String,
    terminal: TerminalColors,
    appearance: Appearance,
    metrics: Metrics,
    overrides: BTreeMap<String, (Expr, String)>,
    flags: &FlagOverrides,
) -> Result<ThemeTokens, String> {
    build_variant(name, terminal, appearance, metrics, overrides, flags, false)
}

/// [`build`], on the Increase Contrast rules (`high_contrast::high_contrast_rules`) when
/// `high_contrast` is set; the result remembers which, for [`ThemeTokens::with_primary`].
pub(super) fn build_variant(
    name: String,
    terminal: TerminalColors,
    appearance: Appearance,
    metrics: Metrics,
    overrides: BTreeMap<String, (Expr, String)>,
    flags: &FlagOverrides,
    high_contrast: bool,
) -> Result<ThemeTokens, String> {
    #[cfg(test)]
    BUILDS.with(|n| n.set(n.get() + 1));
    let base = if high_contrast { high_contrast_rules(appearance) } else { default_rules(appearance) };
    let mut rules = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut recipe = Recipe { overrides: BTreeMap::new(), flags: flags.clone(), high_contrast };
    for (node, (expr, origin)) in overrides {
        recipe.overrides.insert(node.clone(), expr.clone());
        rules.insert(node.clone(), expr);
        sources.insert(node, origin);
    }
    for key in flags.keys() {
        let node = format!("syntax.{key}");
        if !base.contains_key(&node) {
            rules.entry(node).or_insert_with(|| Expr::Ref("syntax.variable".into()));
        }
    }
    let amber = dark_yellow_warning(appearance).filter(|_| !sources.contains_key("status.warning"));
    // Increase Contrast darkens a light theme's muted text, which would hand some palettes
    // back the brown warning the amber replaces: the default rules judge the yellow.
    let mut plain = (amber.is_some() && high_contrast)
        .then(|| Resolver::new(default_rules(appearance), rules.clone(), terminal.named()));
    let mut r = Resolver::new(base, rules, terminal.named());
    if let Some(amber) = amber {
        let judge = plain.as_mut().unwrap_or(&mut r);
        // The check resolved muted text first; a theme rule that made it follow the
        // warning has seen the plain one, and stays with it.
        if judge.yellow_is_dark() && !judge.values.contains_key("status.warning") {
            r.rules.insert("status.warning".into(), Rc::new(amber.clone()));
        }
    }
    let context = |node: &str, e: String, sources: &BTreeMap<String, String>| {
        let what = match node.strip_prefix("syntax.") {
            Some(k) => format!("syntax {k:?}"),
            None => format!("ui {node:?}"),
        };
        match sources.get(node) {
            Some(origin) => format!("{origin}: {what}: {e}"),
            None => format!("{what} (derived): {e}"),
        }
    };
    // Resolve the file's own entries first so that a cycle through a derived
    // token is reported against the line the user wrote.
    for node in sources.keys() {
        r.color(node).map_err(|e| context(node, e, &sources))?;
    }
    let mut ui = BTreeMap::new();
    for t in UI_TOKENS {
        ui.insert(t.name.to_string(), r.color(t.name).map_err(|e| context(t.name, e, &sources))?);
    }
    let syntax_nodes: Vec<String> =
        r.names().into_iter().filter(|k| k.starts_with("syntax.")).map(str::to_string).collect();
    let mut syntax = BTreeMap::new();
    for node in syntax_nodes {
        let color = r.color(&node).map_err(|e| context(&node, e, &sources))?;
        let key = &node["syntax.".len()..];
        let (mut bold, mut italic) = rule_flags(key);
        if let Some((b, i)) = flags.get(key) {
            bold = b.unwrap_or(bold);
            italic = i.unwrap_or(italic);
        }
        syntax.insert(key.to_string(), SyntaxStyle { color, bold, italic });
    }
    Ok(ThemeTokens { name, appearance, terminal, ui, syntax, metrics, recipe })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::theme::tokens::test_support::{hex, no_lookup, resolve_str};
    use crate::theme::tokens::{Color, ThemeFile, resolve};

    #[test]
    fn expression_evaluation() {
        let mut ui = BTreeMap::new();
        ui.insert("primary".into(), "#ff0000".into());
        ui.insert("sidebar.row.selected".into(), "primary / 15%".into());
        ui.insert("border".into(), "mix(primary, #0000ff, 25%)".into());
        ui.insert("border.strong".into(), "lighten(terminal.ansi.0, 10%)".into());
        ui.insert("state.hover".into(), "darken(mix(primary, text.primary, 50%), 5%) / 40%".into());
        ui.insert("chip.border".into(), "syntax.keyword".into());
        let file = ThemeFile { extends: Some("Tokyo Night".into()), ui, ..Default::default() };
        let t = resolve(&file, &no_lookup).unwrap();
        let get = |n: &str| t.get(n).unwrap();
        assert_eq!(get("primary").to_hex(), "#ff0000");
        assert_eq!(get("sidebar.row.selected").to_hex(), "#ff000026");
        assert_eq!(get("border").to_hex(), "#bf0040");
        assert_eq!(get("border.strong"), get("terminal.ansi.0").lighten(0.1));
        let hover = get("primary").mix(get("text.primary"), 0.5).darken(0.05).with_alpha(0.4);
        assert_eq!(get("state.hover"), hover);
        assert_eq!(get("chip.border"), t.syntax["keyword"].color);
        let t = resolve_str(r##"{"ui": {"border": "over(#ffffff / 50%, #000000)"}}"##).unwrap();
        assert_eq!(t.get("border").unwrap().to_hex(), "#808080");
    }

    /// What `readable()` gives for the text (4.5) and non-text (3) minimums never shows
    /// as just below them: not after rounding to 8 bits per channel, nor with the ratio
    /// rounded to two decimals. (The margin is sized for those; at 7:1 a dark color's
    /// 8-bit step is larger than it.)
    #[test]
    fn readable_never_rounds_below_its_target() {
        let colors = ["#7aa2f7", "#e0af68", "#9d4a8c", "#4d2d00", "#808080", "#3b4261", "#df8e1d", "#c0caf5"];
        let backgrounds = ["#1a1b26", "#16161e", "#eff1f5", "#ffffff", "#2e3440", "#cad5ed", "#232a3d"];
        for (fg, bg) in colors.iter().flat_map(|f| backgrounds.iter().map(move |b| (f, b))) {
            for target in [3.0, 4.5] {
                let src = format!(r#"{{"ui": {{"border.focus": "readable({fg}, {bg}, {target})"}}}}"#);
                let shown = resolve_str(&src).unwrap().get("border.focus").unwrap().quantized();
                let ratio = Color::contrast_ratio(shown, hex(bg));
                assert!(ratio >= target, "{fg} on {bg} for {target}: {} at {ratio:.4}", shown.to_hex());
                assert!(format!("{ratio:.2}").parse::<f32>().unwrap() >= target, "{fg} on {bg}: {ratio:.4}");
            }
        }
    }

    /// A version 1 file with a complete palette and `rest` after it.
    fn v1(rest: &str) -> String {
        crate::theme::tokens::test_support::v1_text(rest)
    }

    #[test]
    fn unknown_references_and_cycles() {
        let e = resolve_str(r#"{"ui": {"border": "primry / 10%"}}"#).unwrap_err();
        assert!(e.contains("unknown color `primry` (did you mean `primary`?)"), "{e}");
        assert!(e.contains("ui \"border\""), "{e}");

        let e = resolve_str(r#"{"name": "C", "ui": {"primary": "border.focus"}}"#).unwrap_err();
        // border.focus is derived from primary, so this closes a loop.
        assert!(e.contains("reference cycle: primary -> border.focus -> primary"), "{e}");
        assert!(e.starts_with("theme \"C\""), "{e}");

        let e = resolve_str(r#"{"ui": {"border": "border.strong", "border.strong": "border"}}"#).unwrap_err();
        assert!(e.contains("reference cycle"), "{e}");
        let e = resolve_str(r#"{"ui": {"border": "lighten(border, 5%)"}}"#).unwrap_err();
        assert!(e.contains("reference cycle: border -> border"), "{e}");

        // A version 0 file with a replaced token is migrated on reading (see `format::migrate`);
        // only a file that is not migrated, a version 1 one, reaches the resolver's message.
        assert_eq!(
            resolve_str(r##"{"ui": {"accent": "#fff"}}"##).unwrap().get("primary").unwrap().to_hex(),
            "#ffffff"
        );
        let e = resolve_str(&v1(r##""ui": {"accent": "#fff"}"##)).unwrap_err();
        assert!(e.contains("\"accent\" no longer exists: use `primary`"), "{e}");
        let e = resolve_str(&v1(r##""ui": {"button.primary.text": "#fff"}"##)).unwrap_err();
        assert!(e.contains("removed; primary buttons use"), "{e}");

        let e = resolve_str(r##"{"ui": {"primry": "#fff"}}"##).unwrap_err();
        assert!(e.contains("unknown ui token \"primry\" (did you mean `primary`?)"), "{e}");

        let e = resolve_str(r#"{"terminal": {"background": "primary"}}"#).unwrap_err();
        assert!(e.contains("terminal colors may only reference terminal.* colors"), "{e}");
        let e = resolve_str(r##"{"terminal": {"background": "#00000080"}}"##).unwrap_err();
        assert!(e.contains("must be opaque"), "{e}");
        let e = resolve_str(r##"{"terminal": {"ansi": {"purple": "#000"}}}"##).unwrap_err();
        assert!(e.contains("bad key \"purple\""), "{e}");
        let e = resolve_str(r#"{"terminal": {"cursor": "terminal.cursor"}}"#).unwrap_err();
        assert!(e.contains("reference cycle"), "{e}");
    }
}
