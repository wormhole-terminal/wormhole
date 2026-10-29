//! Light and dark appearances of one theme: which built-in is another's counterpart, where a new
//! value goes by default (one appearance or both), and turning a single theme into a pair.
//!
//! A pair is a theme file with two `variants` named `light` and `dark`. What is shared lives at the
//! top of the file (the accent, the metrics, the syntax colours); what is an appearance's own
//! (surfaces, text, the terminal's colours) lives in its variant, with the `base` it was copied
//! from. A value lives in exactly one place.

use super::BaseLayer;
use super::copy::{Lookup, copy_builtin_base};
use super::slug::BUILTIN_SLUGS;
use crate::theme::tokens::catalog::UI_TOKENS;
use crate::theme::tokens::file::{SyntaxOverride, TerminalFile, ThemeFile, ThemeVariant, ansi_index};
use crate::theme::tokens::{Appearance, Color, ThemeTokens, contrast_issues, resolve, variants};

/// The names of a pair's two variants in the file.
pub const LIGHT: &str = "light";
pub const DARK: &str = "dark";

/// Where a new value of a token goes by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Both appearances: the accent, the metrics, the syntax colours.
    Both,
    /// The appearance being edited: surfaces, text, the terminal's colours.
    Appearance,
}

/// The default [`Scope`] of the token `name`.
pub fn token_scope(name: &str) -> Scope {
    let shared = name.starts_with("metrics.")
        || name.starts_with("syntax.")
        || UI_TOKENS.iter().any(|t| t.name == name && t.group == "Primary");
    if shared { Scope::Both } else { Scope::Appearance }
}

/// Pairs of built-ins that are not two versions of one family.
const PAIRS: &[(&str, &str)] = &[("catppuccin_mocha", "catppuccin_latte"), ("github_dark", "github_light")];

/// The built-in that is the other appearance of the built-in `slug`: the `.dark` of a family's
/// `.light` and back, or one of [`PAIRS`].
pub fn counterpart(slug: &str) -> Option<&'static str> {
    let listed = |s: &str| BUILTIN_SLUGS.iter().map(|(_, slug)| *slug).find(|l| *l == s);
    for (a, b) in PAIRS {
        if *a == slug {
            return listed(b);
        }
        if *b == slug {
            return listed(a);
        }
    }
    let (family, version) = slug.split_once('.')?;
    let other = match version {
        LIGHT => DARK,
        DARK => LIGHT,
        _ => return None,
    };
    listed(&format!("{family}.{other}"))
}

/// Where a generated appearance puts the lightness (OKLCH `L`) of its background and of its text:
/// a light one near paper white with near-black text, a dark one near black with near-white text.
/// The theme's other colours are placed between them in proportion, so a palette keeps its
/// character and stays readable (a plain turning over of `L` puts a mid-grey background where
/// a dark theme's slate was).
fn poles(to: Appearance) -> (f32, f32) {
    if to == Appearance::Light { (0.97, 0.15) } else { (0.20, 0.93) }
}

/// How the lightness of one appearance's colours maps to the other's: a straight line through
/// the two poles, `L' = offset + scale * L`.
#[derive(Clone, Copy, Debug)]
struct Remap {
    offset: f32,
    scale: f32,
}

impl Remap {
    /// From the appearance `have` (its background and text lightness) to `to`'s poles.
    fn between(have: &ThemeTokens, to: Appearance) -> Self {
        let (from_bg, from_fg) =
            (have.terminal.background.to_oklch().0, have.terminal.foreground.to_oklch().0);
        let (to_bg, to_fg) = poles(to);
        if (from_fg - from_bg).abs() < 0.1 {
            // No spread to measure: turn lightness over.
            return Self { offset: 1.0, scale: -1.0 };
        }
        let scale = (to_fg - to_bg) / (from_fg - from_bg);
        Self { offset: to_bg - scale * from_bg, scale }
    }

    fn color(self, color: Color) -> Color {
        let (l, chroma, hue) = color.to_oklch();
        Color::from_oklch((self.offset + self.scale * l).clamp(0.0, 1.0), chroma, hue).with_alpha(color.a)
    }

    /// `value` remapped when it is a plain colour; an expression stays one (it follows what it
    /// names).
    fn value(self, value: &str) -> String {
        Color::from_hex(value).map_or_else(|_| value.to_string(), |c| self.color(c).to_hex())
    }
}

/// The colour moved in lightness, away from `against`, until the two reach `min` contrast
/// (hue and chroma kept); the colour itself when it already does.
fn with_contrast(color: Color, against: Color, min: f32) -> Color {
    if Color::contrast_ratio(color, against) >= min {
        return color;
    }
    let (l, chroma, hue) = color.to_oklch();
    let step = if against.luminance() > 0.18 { -0.01 } else { 0.01 };
    let mut at = l;
    for _ in 0..100 {
        at = (at + step).clamp(0.0, 1.0);
        let candidate = Color::from_oklch(at, chroma, hue).with_alpha(color.a);
        if Color::contrast_ratio(candidate, against) >= min {
            return candidate;
        }
    }
    Color::from_oklch(at, chroma, hue).with_alpha(color.a)
}

/// The layers of an appearance remapped to `to`: the terminal's colours and the appearance's own
/// interface colours (the accent is the theme's, shared), with the terminal's text, palette,
/// cursor and selection then kept readable on the new background.
fn flip_layers(source: &ThemeFile, remap: Remap, to: Appearance) -> ThemeFile {
    let mut file =
        ThemeFile { appearance: Some(to), metrics: source.metrics.clone(), ..ThemeFile::default() };
    let (s, t) = (&source.terminal, &mut file.terminal);
    let flip = |value: &Option<String>| value.as_deref().map(|v| remap.value(v));
    t.foreground = flip(&s.foreground);
    t.background = flip(&s.background);
    t.cursor = flip(&s.cursor);
    t.cursor_text = flip(&s.cursor_text);
    t.selection_background = flip(&s.selection_background);
    t.selection_foreground = flip(&s.selection_foreground);
    t.ansi = s.ansi.iter().map(|(k, v)| (k.clone(), remap.value(v))).collect();
    file.ui = source
        .ui
        .iter()
        .filter(|(name, _)| token_scope(name) == Scope::Appearance)
        .map(|(name, v)| (name.clone(), remap.value(v)))
        .collect();
    file.syntax = source
        .syntax
        .iter()
        .map(|(name, v)| {
            let flipped = match v {
                SyntaxOverride::Color(c) => SyntaxOverride::Color(remap.value(c)),
                SyntaxOverride::Style { color, bold, italic } => SyntaxOverride::Style {
                    color: color.as_deref().map(|c| remap.value(c)),
                    bold: *bold,
                    italic: *italic,
                },
            };
            (name.clone(), flipped)
        })
        .collect();
    keep_readable(&mut file.terminal);
    file
}

/// What the terminal's own colours have to be on their background for a theme to be usable: the
/// text 7:1, the palette and cursor 3:1, a selection that shows.
fn keep_readable(t: &mut TerminalFile) {
    let Some(bg) = t.background.as_deref().and_then(|v| Color::from_hex(v).ok()) else { return };
    let fix = |slot: &mut Option<String>, min: f32| {
        if let Some(c) = slot.as_deref().and_then(|v| Color::from_hex(v).ok()) {
            *slot = Some(with_contrast(c, bg, min).to_hex());
        }
    };
    fix(&mut t.foreground, 7.0);
    fix(&mut t.cursor, 3.0);
    for (index, value) in &mut t.ansi {
        // The palette's first colour is the "black" a program draws on a background of its own.
        if ansi_index(index) == Some(0) {
            continue;
        }
        if let Ok(c) = Color::from_hex(value) {
            *value = with_contrast(c, bg, 3.0).to_hex();
        }
    }
    if let Some(sel) = t.selection_background.as_deref().and_then(|v| Color::from_hex(v).ok()) {
        t.selection_background = Some(with_contrast(sel, bg, SELECTION_CONTRAST).to_hex());
    }
}

/// How far a selection has to stand out from the background it is drawn on.
const SELECTION_CONTRAST: f32 = 1.25;

/// A base for an appearance that has no counterpart to copy: the other's, remapped. It is marked
/// as generated by having no `copied` day, so nothing offers to update it from its source.
fn flipped(base: &BaseLayer, remap: Remap, to: Appearance) -> BaseLayer {
    BaseLayer { from: base.from.clone(), copied: None, file: flip_layers(&base.file, remap, to) }
}

/// A counterpart base as the new appearance holds it: the accent is the theme's (shared), and the
/// appearance is the one the variant is. Applying the same to a fresh copy of the counterpart is
/// what makes the two comparable.
pub fn as_variant_base(base: &mut BaseLayer, appearance: Option<Appearance>) {
    base.file.ui.retain(|name, _| token_scope(name) == Scope::Appearance);
    base.file.appearance = appearance;
}

/// Why a theme cannot be made a pair.
const NEEDS_SINGLE: &str = "this theme already has more than one appearance";

/// How often generated colours are re-checked against the interface's contrast pairs.
const REPAIR_ROUNDS: usize = 4;

/// The appearance `to` added to the single theme `file` (version 1), whose own appearance is
/// `have`: what `file` is becomes the variant of its appearance, and the new one copies the
/// counterpart of its base. With none (or no base at all), it is generated: the theme's own
/// colours with their lightness remapped (see [`poles`]) and the text colours the interface
/// reads from them raised to the contrast the interface asks for. The accent of the theme carries
/// over to both. `copied` is today's date for the new base.
pub fn add_appearance(
    file: &ThemeFile,
    have: &ThemeTokens,
    to: Appearance,
    lookup: Lookup<'_>,
    copied: &str,
) -> Result<ThemeFile, String> {
    if !file.variants.is_empty() {
        return Err(NEEDS_SINGLE.into());
    }
    let name_of = |a: Appearance| if a == Appearance::Light { LIGHT } else { DARK };
    let base = file.base.as_deref();
    let counter = base
        .and_then(|base| counterpart(&base.from))
        .map(|slug| {
            let mut base = copy_builtin_base(slug, lookup, copied)?;
            as_variant_base(&mut base, Some(to));
            Ok::<_, String>(base)
        })
        .transpose()?;
    let generated = counter.is_none();

    let mut shared = ThemeFile {
        version: file.version,
        id: file.id.clone(),
        schema: file.schema.clone(),
        name: file.name.clone(),
        syntax: file.syntax.clone(),
        metrics: file.metrics.clone(),
        ..ThemeFile::default()
    };
    let mut own = ThemeVariant {
        name: name_of(have.appearance).to_string(),
        base: file.base.clone(),
        appearance: file.appearance.or(Some(have.appearance)),
        terminal: file.terminal.clone(),
        ..ThemeVariant::default()
    };
    for (name, value) in &file.ui {
        match token_scope(name) {
            Scope::Both => shared.ui.insert(name.clone(), value.clone()),
            Scope::Appearance => own.ui.insert(name.clone(), value.clone()),
        };
    }
    if !shared.ui.contains_key("primary")
        && let Some(primary) = have.get("primary")
    {
        shared.ui.insert("primary".into(), primary.to_hex());
    }
    let mut added = ThemeVariant {
        name: name_of(to).to_string(),
        appearance: Some(to),
        terminal: TerminalFile::default(),
        ..ThemeVariant::default()
    };
    let remap = Remap::between(have, to);
    match (counter, base) {
        (Some(counter), _) => added.base = Some(Box::new(counter)),
        (None, Some(base)) => added.base = Some(Box::new(flipped(base, remap, to))),
        (None, None) => {
            // Nothing under the theme to copy: its own colours, turned, are the new appearance's.
            let layers = flip_layers(file, remap, to);
            added.terminal = layers.terminal;
            added.ui = layers.ui;
            added.syntax = layers.syntax;
        }
    }
    // Light, then dark: the order the person reads them in.
    shared.variants = vec![own, added];
    shared.variants.sort_by_key(|v| v.name != LIGHT);
    if generated {
        raise_text_contrast(&mut shared, name_of(to), lookup)?;
    }
    Ok(shared)
}

/// Apply the interface's contrast fixes to the generated appearance `name` of `pair` until its text
/// colours meet the minimums of [`crate::theme::tokens::CONTRAST_PAIRS`] (as far as a fix can; the terminal's
/// own colours were placed already).
fn raise_text_contrast(pair: &mut ThemeFile, name: &str, lookup: Lookup<'_>) -> Result<(), String> {
    let Some(at) = pair.variants.iter().position(|v| v.name == name) else { return Ok(()) };
    for _ in 0..REPAIR_ROUNDS {
        let folded = variants(pair, pair.name.as_deref().unwrap_or("Theme"))?;
        let tokens = resolve(&folded[at].file, lookup)?;
        let variant = &mut pair.variants[at];
        // What the person would have typed: the generated values, the base's under them.
        let mut typed = variant.base.as_ref().map(|b| b.file.ui.clone()).unwrap_or_default();
        typed.extend(variant.ui.clone());
        let fixes: Vec<_> = contrast_issues(&tokens, &typed).into_iter().filter_map(|i| i.fix).collect();
        if fixes.is_empty() {
            break;
        }
        for fix in fixes {
            // A base keeps the values a generated appearance starts from; with none they are its own.
            match variant.base.as_mut() {
                Some(base) => base.file.ui.insert(fix.token, fix.expression),
                None => variant.ui.insert(fix.token, fix.expression),
            };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::slug::builtin_name;
    use super::*;
    use crate::theme::tokens::{copy, resolve, variants};

    #[test]
    fn every_counterpart_is_a_built_in_of_the_other_appearance_and_goes_both_ways() {
        let mut found = 0;
        for (name, slug) in BUILTIN_SLUGS {
            let Some(other) = counterpart(slug) else { continue };
            found += 1;
            assert_eq!(counterpart(other), Some(*slug), "{slug} <-> {other}");
            let a = crate::theme::tokens::builtin_tokens().into_iter().find(|t| t.name == *name).unwrap();
            let other_name = builtin_name(other).unwrap();
            let b =
                crate::theme::tokens::builtin_tokens().into_iter().find(|t| t.name == other_name).unwrap();
            assert_ne!(a.appearance, b.appearance, "{slug} and {other} differ in appearance");
        }
        assert!(found >= 30, "{found}");
        assert_eq!(counterpart("wormhole.light"), Some("wormhole.dark"));
        assert_eq!(counterpart("catppuccin_latte"), Some("catppuccin_mocha"));
        assert_eq!(counterpart("nord"), None);
    }

    #[test]
    fn remapping_keeps_hue_chroma_and_opacity_and_puts_the_poles_where_they_belong() {
        let nord = crate::theme::tokens::builtin_tokens().into_iter().find(|t| t.name == "Nord").unwrap();
        let remap = Remap::between(&nord, Appearance::Light);
        let bg = remap.color(nord.terminal.background);
        let fg = remap.color(nord.terminal.foreground);
        assert!((bg.to_oklch().0 - 0.97).abs() < 0.02, "{:?}", bg.to_oklch());
        assert!((fg.to_oklch().0 - 0.15).abs() < 0.02, "{:?}", fg.to_oklch());
        let c = Color::from_hex("#3b6ea8").unwrap();
        let (m, (l, ch, h)) = (remap.color(c), c.to_oklch());
        let (ml, mch, mh) = m.to_oklch();
        assert!(
            ml != l && (mch - ch).abs() < 0.03 && (mh - h).abs() < 3.0,
            "{l},{ch},{h} -> {ml},{mch},{mh}"
        );
        assert_eq!(remap.value("primary / 20%"), "primary / 20%");
        assert!((remap.color(c.with_alpha(0.4)).a - 0.4).abs() < 1e-6);
    }

    /// What a generated appearance is held to: readable text, a readable palette, a selection that shows.
    fn assert_readable(tokens: &ThemeTokens, light: bool) {
        let t = &tokens.terminal;
        let bg = t.background;
        let l = bg.to_oklch().0;
        if light {
            assert!(l >= 0.9, "a light background: L {l}");
        } else {
            assert!(l <= 0.3, "a dark background: L {l}");
        }
        assert!(
            Color::contrast_ratio(t.foreground, bg) >= 7.0,
            "text {}",
            Color::contrast_ratio(t.foreground, bg)
        );
        for (i, c) in t.ansi.iter().enumerate().filter(|(i, _)| *i != 0) {
            let ratio = Color::contrast_ratio(*c, bg);
            assert!(ratio >= 3.0, "ansi.{i} {} on {}: {ratio}", c.to_hex(), bg.to_hex());
        }
        let sel = Color::contrast_ratio(t.selection_background, bg);
        assert!(sel >= 1.2, "the selection {} on {}: {sel}", t.selection_background.to_hex(), bg.to_hex());
        assert!(Color::contrast_ratio(t.cursor, bg) >= 3.0, "the cursor");
        let warned: Vec<_> = tokens.contrast_warnings();
        let bad: Vec<_> =
            warned.iter().filter(|(fg, _, _)| fg.starts_with("text.") || fg == "primary").collect();
        assert!(bad.iter().all(|(fg, _, _)| fg == "primary"), "text colours meet their minimums: {bad:?}");
    }

    fn single(from: &str) -> (ThemeFile, ThemeTokens) {
        let base = copy::copy_builtin_base(from, &|_| None, "2026-10-29").unwrap();
        let mut file = ThemeFile {
            version: Some(1),
            id: Some("mine".into()),
            name: Some("Mine".into()),
            base: Some(Box::new(base)),
            ..ThemeFile::default()
        };
        file.ui.insert("primary".into(), "#ff8800".into());
        file.ui.insert("surface.panel".into(), "#101010".into());
        let tokens = resolve(&file, &|_| None).unwrap();
        (file, tokens)
    }

    #[test]
    fn a_dark_theme_gets_its_counterpart_light_with_the_accent_carried_over() {
        let (file, have) = single("wormhole.dark");
        let pair = add_appearance(&file, &have, Appearance::Light, &|_| None, "2026-10-30").unwrap();
        let names: Vec<_> = pair.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["light", "dark"]);
        assert_eq!(pair.ui.get("primary").map(String::as_str), Some("#ff8800"), "shared");
        assert!(!pair.ui.contains_key("surface.panel"), "an appearance's own value moved into it");
        let all = variants(&pair, "Mine").unwrap();
        let dark = resolve(&all[1].file, &|_| None).unwrap();
        assert_eq!(dark.get("surface.panel").unwrap().to_hex(), "#101010");
        assert_eq!(dark.get("primary").unwrap().to_hex(), "#ff8800");
        let light = resolve(&all[0].file, &|_| None).unwrap();
        assert_eq!(light.appearance, Appearance::Light);
        assert_eq!(light.get("primary").unwrap().to_hex(), "#ff8800");
        let wormhole_light =
            crate::theme::tokens::builtin_tokens().into_iter().find(|t| t.name == "Wormhole Light").unwrap();
        assert_eq!(light.terminal.background, wormhole_light.terminal.background, "the counterpart's");
        assert_eq!(pair.variants[0].base.as_ref().unwrap().from, "wormhole.light");
    }

    #[test]
    fn a_theme_without_a_counterpart_is_flipped_in_lightness() {
        let (file, have) = single("nord");
        let pair = add_appearance(&file, &have, Appearance::Light, &|_| None, "2026-10-30").unwrap();
        let all = variants(&pair, "Mine").unwrap();
        let light = resolve(&all[0].file, &|_| None).unwrap();
        assert_eq!(light.appearance, Appearance::Light);
        assert!(light.terminal.background.luminance() > have.terminal.background.luminance());
        assert_eq!(light.get("primary").unwrap().to_hex(), "#ff8800");
        assert_readable(&light, true);
        let base = pair.variants[0].base.as_ref().unwrap();
        assert!(base.copied.is_none(), "generated: nothing to update it from");
        assert_eq!(base.from, "nord");
    }

    #[test]
    fn a_generated_dark_appearance_is_readable_too() {
        let (file, have) = single("wormhole.light");
        let pair = add_appearance(&file, &have, Appearance::Dark, &|_| None, "2026-10-30").unwrap();
        let all = variants(&pair, "Mine").unwrap();
        let dark = resolve(&all[1].file, &|_| None).unwrap();
        assert_eq!(dark.appearance, Appearance::Dark);
        assert_readable(&dark, false);
    }

    #[test]
    fn a_theme_with_no_base_gets_its_other_appearance_from_its_own_colours() {
        for (from, to) in [(Appearance::Dark, Appearance::Light), (Appearance::Light, Appearance::Dark)] {
            let file = ThemeFile {
                id: Some("plain".into()),
                name: Some("Plain".into()),
                ..crate::theme::tokens::starter::starter(from)
            };
            let have = resolve(&file, &|_| None).unwrap();
            let pair = add_appearance(&file, &have, to, &|_| None, "2026-10-30").unwrap();
            let added = pair.variants.iter().find(|v| v.appearance == Some(to)).unwrap();
            assert!(added.base.is_none(), "its colours are its own");
            let all = variants(&pair, "Plain").unwrap();
            let ix = pair.variants.iter().position(|v| v.appearance == Some(to)).unwrap();
            let made = resolve(&all[ix].file, &|_| None).unwrap();
            assert_eq!(made.appearance, to);
            assert_readable(&made, to == Appearance::Light);
            // The one it came from is as it was.
            let kept = resolve(&all[1 - ix].file, &|_| None).unwrap();
            assert_eq!(kept.terminal, have.terminal);
        }
    }

    #[test]
    fn scopes_follow_what_a_value_is_for() {
        assert_eq!(token_scope("primary"), Scope::Both);
        assert_eq!(token_scope("metrics.radius"), Scope::Both);
        assert_eq!(token_scope("syntax.keyword"), Scope::Both);
        assert_eq!(token_scope("surface.panel"), Scope::Appearance);
        assert_eq!(token_scope("text.primary"), Scope::Appearance);
        assert_eq!(token_scope("terminal.ansi.3"), Scope::Appearance);
        let (file, have) = single("nord");
        let pair = add_appearance(&file, &have, Appearance::Light, &|_| None, "x").unwrap();
        assert!(add_appearance(&pair, &have, Appearance::Dark, &|_| None, "x").is_err(), "already a pair");
    }
}
