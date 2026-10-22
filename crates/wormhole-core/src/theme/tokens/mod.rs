//! Design tokens: every color, syntax style and metric the UI uses, derived
//! from a terminal palette (see [`derive`]) and optionally customized by
//! JSONC theme files (see [`parse_theme_file`] / [`resolve`]).
//!
//! # Model
//!
//! Every UI token and syntax color is a *node* with a default rule written in
//! the same expression language theme files use (see [`default_rule`]). A
//! theme file replaces some rules; then everything is resolved once, against
//! the final terminal palette. That is why overriding `primary` also moves
//! `border.focus`, the checkbox and friends, and why changing
//! `terminal.background` re-derives the surfaces.
//!
//! Node names: UI tokens are bare dotted names (`primary`, `text.primary`),
//! syntax colors are `syntax.<key>`, terminal colors are `terminal.<key>` with
//! keys `foreground`, `background`, `cursor`, `cursor_text`,
//! `selection_background`, `selection_foreground` and `ansi.0`..`ansi.15`.
//!
//! # Expression grammar
//!
//! ```text
//! expr    := primary ( "/" amount )*          alpha: X / 15% sets alpha to 0.15
//! primary := "#" hex                          #rgb #rgba #rrggbb #rrggbbaa
//!          | name                             reference to another node
//!          | func "(" expr ("," arg)* ")"
//!          | "(" expr ")"
//! amount  := number "%" | number              0.15 == 15%; must be within 0..1
//! ```
//!
//! Functions:
//! - `mix(X, Y, p)`: `p` of the way from X to Y (straight sRGB blend; 0% = X).
//! - `over(X, Y)`: X composited over Y, as the screen shows a translucent X on Y.
//! - `lighten(X, p)` / `darken(X, p)`: add/subtract `p` to OKLab lightness,
//!   keeping hue and (where the sRGB gamut allows) chroma.
//! - `alpha(X, p)`: same as `X / p`.
//! - `shade(X, p)`: lighten or darken by `p`, whichever moves away from the text
//!   color that sits on X (see `text_on`). Used for hover states so the label
//!   on a hovered button never loses contrast.
//! - `text_on(X)`: black or white, whichever contrasts more with X.
//! - `readable(X, BG1, BG2, ..., ratio?)`: X, pushed toward white or black
//!   along OKLab lightness until it reaches `ratio` (default 4.5) against every
//!   background, aiming 0.03 past it so 8-bit rounding never shows 4.49.
//!   Translucent backgrounds are judged over `surface.window` (`over()` names
//!   another underlay).
//!
//! The rules judge text on opaque surfaces. A translucent window judges a few of
//! them again on what shows through it ([`ThemeTokens::translucent`]), after
//! derivation, so nothing derived or cached here depends on the window opacity.
//!
//! Increase Contrast shows a variant of the theme ([`ThemeTokens::high_contrast`]): its
//! recipe derived again on a second rule set that replaces a few default rules, so
//! the theme's own values still win. Those rules are cached on their own, and neither the
//! default rules nor the built-in themes ever depend on the setting.

mod catalog;
mod check;
pub(crate) mod color;
mod contrast;
#[cfg(test)]
mod design_values;
mod expr;
mod extends;
mod file;
mod fixes;
mod high_contrast;
mod model;
mod own;
mod resolve;
mod rules;
mod save;
#[cfg(test)]
mod test_support;
mod theme_tokens;
mod translucent;
mod uses;
mod variants;

use std::collections::BTreeMap;
use std::sync::LazyLock;

pub use self::catalog::{METRIC_TOKENS, TokenInfo, UI_TOKENS, syntax_keys};
pub use self::check::{
    COMPLETIONS_MAX, Completion, Completions, ExprError, FUNCTIONS, check_expression, completions,
};
pub use self::contrast::{CONTRAST_PAIRS, ContrastPair};
#[cfg(test)]
use self::extends::MAX_SYNTAX_KEYS;
pub use self::extends::resolve;
pub use self::file::{
    MAX_THEME_BYTES, MetricsFile, SyntaxOverride, TerminalFile, ThemeFile, ThemeVariant, parse_theme_file,
};
pub use self::fixes::{ContrastCheck, ContrastFix, ContrastIssue, contrast_issues};
pub use self::high_contrast::HIGH_CONTRAST_RING;
use self::model::auto_appearance;
pub use self::model::{Appearance, Metrics, SyntaxStyle, TerminalColors};
use self::resolve::build;
pub use self::rules::default_rule;
pub use self::theme_tokens::ThemeTokens;
pub use self::translucent::{BLUR_OFF_MIN_OPACITY, OPAQUE, Translucency, TranslucentColors};
pub use self::uses::{Area, TOKEN_USES, TokenUse, followers, refers_to, used_in, uses_of};
pub use self::variants::{MAX_VARIANTS, Variant, variant_name, variants};
pub use crate::color::Color;
use crate::theme;

/// Derive every token from a terminal palette alone.
pub fn derive(primitives: &theme::Theme) -> ThemeTokens {
    let terminal = TerminalColors::from_theme(primitives);
    let appearance = auto_appearance(&terminal);
    // No overrides means only the static rules run; those are covered by the
    // built-in tests, so an error is a bug in this file, never user input.
    build(
        primitives.name.clone(),
        terminal,
        appearance,
        Metrics::default(),
        BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap_or_else(|e| panic!("built-in derivation rules are inconsistent: {e}"))
}

/// Tokens for every bundled theme, in [`theme::builtin`] order (a copy of
/// [`builtins`]).
pub fn builtin_tokens() -> Vec<ThemeTokens> {
    builtins().to_vec()
}

/// Tokens for every bundled theme, derived once: the imported palettes in [`theme::builtin`]
/// order, then Wormhole's own themes, each version of one on its own.
pub fn builtins() -> &'static [ThemeTokens] {
    static TOKENS: LazyLock<Vec<ThemeTokens>> = LazyLock::new(|| {
        let palettes = theme::builtins().iter().map(derive);
        // Part of the app, like the palettes: a theme that doesn't resolve is a bug the tests catch.
        let own = own::own()
            .iter()
            .map(|o| resolve(&o.file, &|_| None).unwrap_or_else(|e| panic!("own theme {}: {e}", o.name)));
        palettes.chain(own).collect()
    });
    &TOKENS
}

/// For a bundled theme that is one version of several (`"Soltari Light"`), the family's name
/// and the version's own (`("Soltari", "Light")`).
pub fn builtin_family(name: &str) -> Option<(&'static str, &'static str)> {
    let own = own::own().iter().find(|o| o.name == name)?;
    own.family.as_ref().map(|(family, short)| (family.as_str(), short.as_str()))
}

/// The bundled theme a family's name means (`"Soltari"`: its first version), if it is not a
/// theme's own name.
pub fn builtin_family_default(name: &str) -> Option<&'static str> {
    own::own_named(name).filter(|o| o.family.is_some()).map(|o| o.name.as_str())
}

/// How many themes were derived or resolved on this thread: what tests count
/// to see that work was skipped.
#[cfg(test)]
pub(crate) fn builds_on_this_thread() -> usize {
    resolve::BUILDS.with(|n| n.get())
}
