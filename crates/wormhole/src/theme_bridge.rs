//! Maps Wormhole's design tokens onto GPUI Kit's theme, so its controls
//! (buttons, inputs, switches, lists, tabs, …) and syntax highlighting follow
//! the active theme instead of GPUI Kit's defaults.
//!
//! The mapping is a table ([`KIT_COLORS`]) rather than code, and goes in through
//! `Theme::update`, which keeps the Kit's two copies of every color and its Base
//! layer (scrollbars, text selection) in step; writing the fields directly left
//! those on the Kit's defaults.

use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::{Theme as KitTheme, ThemeColor, ThemeMode};
use gpui_kit::{App, Global, Hsla, px, transparent_black};
use wormhole_core::theme::fonts::{DEFAULT_UI_FONT_SIZE, ResolvedFonts};
use wormhole_core::theme::tokens::{Appearance, Color, ThemeTokens};

pub fn hsla(color: Color) -> Hsla {
    gpui_kit::Rgba { r: color.r, g: color.g, b: color.b, a: color.a }.into()
}

/// The inverse of [`hsla`].
pub fn color(hsla: Hsla) -> Color {
    let rgba = gpui_kit::Rgba::from(hsla);
    Color { r: rgba.r, g: rgba.g, b: rgba.b, a: rgba.a }
}

/// The window's rem size for these fonts: GPUI Kit sizes text and spacing in rems of the theme's
/// `font_size` (16 px where our 13 pt interface text sits).
pub fn rem_size(fonts: &ResolvedFonts) -> gpui_kit::Pixels {
    px(fonts.ui_size * 16.0 / DEFAULT_UI_FONT_SIZE)
}

/// What a token that does not exist shows as: magenta, so a typo is easy to spot.
pub const MISSING_TOKEN: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };

/// A token as GPUI color; unknown names (a bug) fall back to [`MISSING_TOKEN`].
pub fn token(tokens: &ThemeTokens, name: &str) -> Hsla {
    tokens.get(name).map_or_else(|| hsla(MISSING_TOKEN), hsla)
}

/// Where one of the Kit's colors comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Token(&'static str),
    /// Nothing: our own panels paint that surface, or the Kit draws no line there.
    Clear,
}

const fn tok(name: &'static str) -> Source {
    Source::Token(name)
}
const CLEAR: Source = Source::Clear;

impl Source {
    /// `None` for a token that does not exist.
    fn resolve(self, tokens: &ThemeTokens) -> Option<Hsla> {
        match self {
            Source::Token(name) => tokens.get(name).map(hsla),
            Source::Clear => Some(transparent_black()),
        }
    }
}

/// One of the Kit's colors, and where it comes from.
struct Mapping {
    field: &'static str,
    slot: fn(&mut ThemeColor) -> &mut Hsla,
    from: Source,
}

macro_rules! mapping {
    ($($field:ident = $from:expr;)*) => {
        &[$(Mapping { field: stringify!($field), slot: |colors| &mut colors.$field, from: $from }),*]
    };
}

/// Every Kit color we set (the rest keep the Kit's defaults for the mode).
const KIT_COLORS: &[Mapping] = mapping! {
    // Our own panels paint the window surfaces (with opacity + blur).
    background = CLEAR;
    foreground = tok("text.primary");
    border = tok("border");
    window_border = tok("border");
    // The Kit has no ring width; our own rings take `UiColors::focus_ring_width`.
    ring = tok("border.focus");
    caret = tok("text.primary");
    selection = tok("selection.ui");
    link = tok("text.link");
    link_hover = tok("text.link");
    link_active = tok("text.link");
    overlay = tok("surface.overlay");
    muted = tok("state.hover");
    muted_foreground = tok("text.muted");
    accent = tok("state.hover");
    accent_foreground = tok("text.primary");
    popover = tok("surface.elevated");
    popover_foreground = tok("text.primary");
    title_bar = tok("titlebar.background");
    title_bar_border = tok("border");
    sidebar = tok("sidebar.background");
    sidebar_foreground = tok("text.primary");
    sidebar_border = tok("border");
    sidebar_accent = tok("sidebar.row.selected");
    sidebar_accent_foreground = tok("sidebar.row.selected.text");
    sidebar_primary = tok("primary");
    sidebar_primary_foreground = tok("text.on_primary");
    primary = tok("primary");
    primary_hover = tok("primary.hover");
    primary_active = tok("primary.hover");
    primary_foreground = tok("text.on_primary");
    secondary = tok("button.background");
    secondary_hover = tok("button.hover");
    secondary_active = tok("state.active");
    secondary_foreground = tok("button.text");
    button = tok("button.background");
    button_hover = tok("button.hover");
    button_active = tok("state.active");
    button_foreground = tok("button.text");
    button_primary = tok("primary");
    button_primary_hover = tok("primary.hover");
    button_primary_active = tok("primary.hover");
    button_primary_foreground = tok("text.on_primary");
    button_secondary = tok("button.background");
    button_secondary_hover = tok("button.hover");
    button_secondary_active = tok("state.active");
    button_secondary_foreground = tok("button.text");
    danger = tok("status.danger");
    danger_hover = tok("status.danger");
    danger_active = tok("status.danger");
    danger_foreground = tok("text.on_primary");
    success = tok("status.success");
    success_hover = tok("status.success");
    success_active = tok("status.success");
    success_foreground = tok("text.on_primary");
    warning = tok("status.warning");
    warning_hover = tok("status.warning");
    warning_active = tok("status.warning");
    warning_foreground = tok("text.on_primary");
    info = tok("status.info");
    info_hover = tok("status.info");
    info_active = tok("status.info");
    info_foreground = tok("text.on_primary");
    input = tok("input.border");
    list = CLEAR;
    list_hover = tok("state.hover");
    // The selection in pickers and Kit lists: an opaque tint and no outline, as in the
    // palette (Kit list items keep their own text color on it).
    list_active = tok("list.selected");
    list_active_border = CLEAR;
    list_even = CLEAR;
    list_head = tok("surface.panel");
    tab_bar = tok("surface.panel");
    tab = CLEAR;
    tab_active = tok("tab.active.background");
    tab_foreground = tok("tab.inactive.text");
    tab_active_foreground = tok("text.primary");
    scrollbar = CLEAR;
    scrollbar_thumb = tok("scrollbar.thumb");
    scrollbar_thumb_hover = tok("text.faint");
    switch = tok("border.strong");
    switch_thumb = tok("switch.knob");
    slider_bar = tok("primary");
    slider_thumb = tok("switch.knob");
    progress_bar = tok("primary");
    drop_target = tok("state.highlight");
    drag_border = tok("primary");
};

/// Which theme the Kit was last given, so handing it the same one again is free.
#[derive(Clone)]
struct Applied {
    tokens: Arc<ThemeTokens>,
    fonts: ResolvedFonts,
}

impl Global for Applied {}

fn is_applied(
    tokens: &ThemeTokens,
    shared: Option<&Arc<ThemeTokens>>,
    fonts: &ResolvedFonts,
    cx: &App,
) -> bool {
    cx.try_global::<Applied>().is_some_and(|applied| {
        applied.fonts == *fonts
            && (shared.is_some_and(|t| Arc::ptr_eq(t, &applied.tokens)) || *applied.tokens == *tokens)
    })
}

/// Apply `tokens` and the resolved fonts to GPUI Kit's global theme, unless it already
/// has them. The settings store does this itself whenever the theme or the fonts change
/// ([`apply_shared`]); this is for a caller that holds the values separately.
pub fn apply(tokens: &ThemeTokens, fonts: &ResolvedFonts, cx: &mut App) {
    if is_applied(tokens, None, fonts, cx) {
        return;
    }
    apply_now(&Arc::new(tokens.clone()), fonts, &highlight_for(tokens), cx);
}

/// Apply a theme and fonts whose syntax theme is already built (the settings store's, which
/// passes its own: settings sits above this bridge and hands it the values), unless the Kit
/// already has them. The shared tokens compare by pointer first.
pub(crate) fn apply_shared(
    tokens: &Arc<ThemeTokens>,
    fonts: &ResolvedFonts,
    highlight: &Arc<HighlightTheme>,
    cx: &mut App,
) {
    if is_applied(tokens, Some(tokens), fonts, cx) {
        return;
    }
    apply_now(tokens, fonts, highlight, cx);
}

fn apply_now(
    tokens: &Arc<ThemeTokens>,
    fonts: &ResolvedFonts,
    highlight: &Arc<HighlightTheme>,
    cx: &mut App,
) {
    let mode = match tokens.appearance {
        Appearance::Light => ThemeMode::Light,
        Appearance::Dark => ThemeMode::Dark,
    };
    // Loading a mode's defaults resets everything the table does not name, so it is only
    // needed when the mode changes (or there is no Kit theme yet); every other change
    // is an edit of the fields we set.
    if !cx.has_global::<KitTheme>() || KitTheme::global(cx).mode != mode {
        KitTheme::change(mode, None, cx);
    }
    KitTheme::update(cx, |theme| {
        for mapping in KIT_COLORS {
            *(mapping.slot)(&mut theme.colors) = mapping.from.resolve(tokens).unwrap_or_else(|| {
                tracing::warn!(kit_color = mapping.field, "mapped to a token that does not exist");
                hsla(MISSING_TOKEN)
            });
        }
        theme.radius = px(tokens.metrics.radius);
        theme.radius_lg = px(tokens.metrics.radius_lg);
        // GPUI Kit sizes text and spacing in rems of `font_size` (16 px by
        // default, where our 13 pt interface text sits), so the interface size
        // scales the chrome proportionally.
        theme.font_family = fonts.ui_family.clone().into();
        theme.font_size = rem_size(fonts);
        theme.mono_font_family = fonts.code_family.clone().into();
        theme.mono_font_size = px(fonts.code_size);
        theme.highlight_theme = highlight.clone();
    });
    cx.set_global(Applied { tokens: tokens.clone(), fonts: fonts.clone() });
}

/// The syntax theme for `tokens`: [`highlight_theme`], or GPUI Kit's own for the
/// appearance if the tokens don't convert.
pub fn highlight_for(tokens: &ThemeTokens) -> Arc<HighlightTheme> {
    highlight_theme(tokens).unwrap_or_else(|| match tokens.appearance {
        Appearance::Light => HighlightTheme::default_light(),
        Appearance::Dark => HighlightTheme::default_dark(),
    })
}

/// GPUI Kit's syntax theme, built from our syntax tokens (it's deserializable,
/// so the tree-sitter names map across directly).
pub fn highlight_theme(tokens: &ThemeTokens) -> Option<Arc<HighlightTheme>> {
    let mut syntax = serde_json::Map::new();
    for (key, style) in &tokens.syntax {
        let mut entry = serde_json::Map::new();
        entry.insert("color".into(), style.color.to_hex().into());
        if style.italic {
            entry.insert("font_style".into(), "italic".into());
        }
        if style.bold {
            entry.insert("font_weight".into(), 700.into());
        }
        syntax.insert(key.clone(), entry.into());
    }
    let appearance = match tokens.appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    };
    let json = serde_json::json!({
        "name": tokens.name,
        "appearance": appearance,
        "style": {
            "editor.background": tokens.terminal.background.to_hex(),
            "editor.foreground": tokens.terminal.foreground.to_hex(),
            "syntax": syntax,
        }
    });
    match serde_json::from_value::<HighlightTheme>(json) {
        Ok(theme) => Some(Arc::new(theme)),
        Err(err) => {
            tracing::warn!("syntax theme from tokens: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::tokens::UI_TOKENS;

    use super::{KIT_COLORS, Source};

    #[test]
    fn every_kit_color_comes_from_a_real_token() {
        for mapping in KIT_COLORS {
            if let Source::Token(name) = mapping.from {
                assert!(
                    UI_TOKENS.iter().any(|t| t.name == name),
                    "Kit color {} is mapped to {name:?}, which is not a UI token",
                    mapping.field
                );
            }
        }
    }

    #[test]
    fn every_kit_color_is_mapped_once() {
        let mut seen = std::collections::BTreeSet::new();
        for mapping in KIT_COLORS {
            assert!(seen.insert(mapping.field), "{} is mapped twice", mapping.field);
        }
    }
}

/// Handing the Kit a theme, in a headless GPUI (needs `devctl`'s test support). That a settings
/// change reaches the Kit is the store's to test (`settings::app_tests`).
#[cfg(all(test, feature = "devctl"))]
mod kit_tests {
    use gpui_kit::TestAppContext;
    use gpui_kit::component::Theme as KitTheme;
    use wormhole_core::config::Config;

    use super::apply;

    #[gpui_kit::test]
    fn handing_the_kit_the_same_theme_again_does_nothing(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            let config = Config::placeholder();
            let (tokens, fonts) =
                (config.tokens.clone(), config.settings.resolved_fonts(&config.tokens.metrics));
            apply(&tokens, &fonts, cx);
            // Make a change a repeat would undo, to see whether it is redone.
            KitTheme::global_mut(cx).colors.primary = gpui_kit::red();
            apply(&tokens, &fonts, cx);
            assert_eq!(KitTheme::global(cx).colors.primary, gpui_kit::red());
        });
    }
}
