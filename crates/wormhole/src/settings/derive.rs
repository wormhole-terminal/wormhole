//! What the store works out from the settings and the theme, apart from the store itself: the tokens in
//! effect (with the colours chosen over the theme, at the contrast the system asks), how the windows let the desktop
//! through, the main window's colours and what follows from a theme alone. Each is a function of its
//! inputs, with the last result remembered where working it out costs.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use wormhole_core::config::Settings;
use wormhole_core::theme::tokens::{ColorChoices, ThemeTokens, Translucency};

use super::{ChoicesMemo, ContrastMemo, UiColors};
use crate::palette::Palette;
use crate::theme_bridge;

/// The tokens in effect: the Studio's preview exactly as drafted, otherwise the
/// project's theme or the user's with the colors they chose (a primary, a branch color) on
/// top. Re-deriving a theme around them is the one costly step, so the last result is reused
/// while the base theme (by identity) and the choices stay the same.
pub(super) fn themed(
    preview: Option<&Arc<ThemeTokens>>,
    project: Option<&Arc<ThemeTokens>>,
    config: &Arc<ThemeTokens>,
    choices: ColorChoices,
    memo: &mut Option<ChoicesMemo>,
) -> Arc<ThemeTokens> {
    if let Some(preview) = preview {
        return preview.clone();
    }
    let base = project.unwrap_or(config);
    if choices.is_empty() {
        return base.clone();
    }
    if let Some(memo) = memo.as_ref()
        && Arc::ptr_eq(&memo.base, base)
        && memo.choices == choices
    {
        return memo.tokens.clone();
    }
    tally("with_choices");
    let tokens = Arc::new(base.with_choices(choices));
    *memo = Some(ChoicesMemo { base: base.clone(), choices, tokens: tokens.clone() });
    tokens
}

/// Under Increase Contrast, the theme's high-contrast variant (core derives it from the theme's
/// own rules, so what the theme sets still wins). It is worked out after the theme, never cached
/// with it: the built-in themes stay keyed by appearance alone. The last one is reused while the
/// tokens it came from are the same allocation, which `themed` keeps them while nothing changes.
pub(super) fn contrasted(
    tokens: Arc<ThemeTokens>,
    high: bool,
    memo: &mut Option<ContrastMemo>,
) -> Arc<ThemeTokens> {
    if !high {
        return tokens;
    }
    if let Some(memo) = memo.as_ref()
        && Arc::ptr_eq(&memo.base, &tokens)
    {
        return memo.tokens.clone();
    }
    tally("high_contrast");
    let high = Arc::new(tokens.high_contrast());
    *memo = Some(ContrastMemo { base: tokens, tokens: high.clone() });
    high
}

/// How the windows let the desktop through, by the settings in force.
pub(super) fn translucency(settings: &Settings) -> Translucency {
    Translucency { opacity: settings.opacity, blur: settings.blur }
}

/// The main window's colours: the tokens at the window's look, and no line between the columns
/// when the person turned Column dividers off. Increase Contrast keeps the lines, as the
/// window's columns would otherwise be told apart by a tint alone. Its icons hold still when the
/// person turned Animate icons off.
pub(super) fn main_ui(tokens: &ThemeTokens, settings: &Settings, increase_contrast: bool) -> UiColors {
    let ui = UiColors::new(tokens, translucency(settings));
    let ui = if settings.column_dividers || increase_contrast { ui } else { ui.without_dividers() };
    if settings.icon_animations { ui } else { ui.without_icon_motion() }
}

/// An opaque window's look: nothing shows through, so the tokens stand as they are.
pub(super) const OPAQUE_LOOK: Translucency = Translucency { opacity: 1.0, blur: false };

/// What follows from a theme's tokens alone: the terminal palette and the syntax theme.
pub(super) fn theme_parts(tokens: &ThemeTokens) -> (Rc<Palette>, Arc<HighlightTheme>) {
    tally("theme_parts");
    let palette = Palette::from_theme(&tokens.to_theme()).with_tokens(tokens);
    (Rc::new(palette), theme_bridge::highlight_for(tokens))
}

// Counts the derivations a test is about, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    pub(super) static TALLY: std::cell::RefCell<std::collections::BTreeMap<&'static str, usize>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[cfg(test)]
pub(super) fn tally(what: &'static str) {
    TALLY.with(|t| *t.borrow_mut().entry(what).or_default() += 1);
}

#[cfg(not(test))]
#[inline(always)]
pub(super) fn tally(_: &'static str) {}
