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
use crate::theme::tokens::file::{TerminalFile, ThemeFile, ThemeVariant};
use crate::theme::tokens::{Appearance, Color, ThemeTokens};

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

/// The colour with its lightness turned over in OKLCH (`L` becomes `1 - L`), hue, chroma and
/// opacity kept.
pub fn flip_lightness(color: Color) -> Color {
    let (l, chroma, hue) = color.to_oklch();
    Color::from_oklch(1.0 - l, chroma, hue).with_alpha(color.a)
}

/// `value` flipped when it is a plain colour; an expression stays one (it follows what it
/// names).
fn flip_value(value: &str) -> String {
    Color::from_hex(value).map_or_else(|_| value.to_string(), |c| flip_lightness(c).to_hex())
}

/// A base with its colours' lightness flipped, for an appearance that has no counterpart to copy.
fn flipped(base: &BaseLayer, to: Appearance) -> BaseLayer {
    let mut file = base.file.clone();
    file.appearance = Some(to);
    let t = &mut file.terminal;
    for slot in [
        &mut t.foreground,
        &mut t.background,
        &mut t.cursor,
        &mut t.cursor_text,
        &mut t.selection_background,
        &mut t.selection_foreground,
    ] {
        if let Some(v) = slot {
            *v = flip_value(v);
        }
    }
    t.ansi.values_mut().for_each(|v| *v = flip_value(v));
    // The accent comes from the theme being kept, so it is not in the new appearance's base.
    file.ui.retain(|name, _| token_scope(name) == Scope::Appearance);
    file.ui.values_mut().for_each(|v| *v = flip_value(v));
    BaseLayer { file, ..base.clone() }
}

/// Why a theme cannot be made a pair.
const NEEDS_BASE: &str = "only a theme in the new format with a base can get a second appearance";

/// The appearance `to` added to the single theme `file` (version 1, with a `base`), whose own
/// appearance is `have`: what `file` is becomes the variant of its appearance, and the new one
/// copies the counterpart of its base (or, with none, the base with its lightness flipped). The
/// accent of the theme carries over to both. `copied` is today's date for the new base.
pub fn add_appearance(
    file: &ThemeFile,
    have: &ThemeTokens,
    to: Appearance,
    lookup: Lookup<'_>,
    copied: &str,
) -> Result<ThemeFile, String> {
    let base = file.base.as_deref().filter(|_| file.variants.is_empty()).ok_or(NEEDS_BASE)?;
    let name_of = |a: Appearance| if a == Appearance::Light { LIGHT } else { DARK };
    let counter = counterpart(&base.from)
        .map(|slug| {
            let mut base = copy_builtin_base(slug, lookup, copied)?;
            // The accent is the theme's, not the counterpart's.
            base.file.ui.retain(|name, _| token_scope(name) == Scope::Appearance);
            base.file.appearance = Some(to);
            Ok::<_, String>(base)
        })
        .transpose()?;
    let new_base = counter.unwrap_or_else(|| flipped(base, to));

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
    let added = ThemeVariant {
        name: name_of(to).to_string(),
        base: Some(Box::new(new_base)),
        appearance: Some(to),
        terminal: TerminalFile::default(),
        ..ThemeVariant::default()
    };
    // Light, then dark: the order the person reads them in.
    shared.variants = vec![own, added];
    shared.variants.sort_by_key(|v| v.name != LIGHT);
    Ok(shared)
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
    fn flipping_lightness_keeps_hue_chroma_and_opacity() {
        let c = Color::from_hex("#3b6ea8").unwrap();
        let f = flip_lightness(c);
        let ((l, ch, h), (fl, fch, fh)) = (c.to_oklch(), f.to_oklch());
        assert!((fl - (1.0 - l)).abs() < 0.02, "{l} -> {fl}");
        assert!((fch - ch).abs() < 0.03 && (fh - h).abs() < 3.0, "{ch},{h} -> {fch},{fh}");
        assert_eq!(flip_lightness(Color::WHITE).to_hex(), "#000000");
        assert_eq!(flip_value("primary / 20%"), "primary / 20%");
        assert!((flip_lightness(c.with_alpha(0.4)).a - 0.4).abs() < 1e-6);
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
        assert!(pair.ui.get("surface.panel").is_none(), "an appearance's own value moved into it");
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
    }

    #[test]
    fn scopes_follow_what_a_value_is_for() {
        assert_eq!(token_scope("primary"), Scope::Both);
        assert_eq!(token_scope("metrics.radius"), Scope::Both);
        assert_eq!(token_scope("syntax.keyword"), Scope::Both);
        assert_eq!(token_scope("surface.panel"), Scope::Appearance);
        assert_eq!(token_scope("text.primary"), Scope::Appearance);
        assert_eq!(token_scope("terminal.ansi.3"), Scope::Appearance);
        assert!(
            add_appearance(&ThemeFile::default(), &single("nord").1, Appearance::Light, &|_| None, "x")
                .is_err()
        );
    }
}
