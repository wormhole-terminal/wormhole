//! The copy step: making a version 1 theme's `base` from another theme, turning a version 0
//! theme into version 1, and comparing a stored base with its source today.
//!
//! A base holds what the source's whole chain *sets*, as written, so expressions stay
//! expressions and the colors the rules work out stay worked out (they follow the person's
//! edits). Two things are filled in because there is nothing under a version 1 theme to
//! supply them: the whole terminal palette (all 16 ANSI colors, the cursor and the
//! selection), taken from the source as it resolves, so the Tokyo Night that sits under every
//! version 0 theme without an `extends` never shows through. Two things are left out on
//! purpose: a metric the source does not set stays Wormhole's default, and an `appearance`
//! the source leaves to be detected from its background is detected again.

use std::collections::BTreeMap;

use super::slug::{self, part_from};
use super::{BaseLayer, CURRENT_VERSION};
use crate::theme::tokens::extends::{Chain, walk_chain};
use crate::theme::tokens::file::{SyntaxOverride, ThemeFile, ThemeVariant, ansi_index};
use crate::theme::tokens::{Appearance, resolve};

/// Where a theme named `name` is found: a user theme's file, if there is one.
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<ThemeFile>;

/// A base made from the theme `file` is, the whole chain under it merged. `from` is the
/// source's slug, `copied` the day (`2026-10-29`).
pub fn copy_base(
    from: &str,
    file: &ThemeFile,
    lookup: Lookup<'_>,
    copied: &str,
) -> Result<BaseLayer, String> {
    let tokens = resolve(file, lookup)?;
    let flat = super::effective(file);
    let Chain { files, .. } = walk_chain(flat.as_ref(), lookup)?;
    let mut merged = files[0].clone();
    for parent in &files[1..] {
        merged = merged.layered_over(parent);
    }
    let set: Vec<String> = merged.terminal.entries()?.into_iter().map(|(node, _)| node).collect();
    for (node, color) in tokens.terminal.named() {
        if !set.contains(&node) {
            merged.terminal.insert(&node["terminal.".len()..], color.to_hex());
        }
    }
    let layers = ThemeFile {
        appearance: merged.appearance,
        terminal: merged.terminal,
        ui: merged.ui,
        syntax: merged.syntax,
        metrics: merged.metrics,
        ..ThemeFile::default()
    };
    Ok(BaseLayer { from: from.to_string(), copied: Some(copied.to_string()), file: layers })
}

/// A base copied from a built-in theme, by slug (`wormhole.light`, `nord`).
pub fn copy_builtin_base(from: &str, lookup: Lookup<'_>, copied: &str) -> Result<BaseLayer, String> {
    let name = slug::builtin_name(from).ok_or_else(|| format!("no built-in theme {from:?}"))?;
    let by_name = ThemeFile { extends: Some(name.to_string()), ..ThemeFile::default() };
    // The slug a family means is its first version; the base says which one it is.
    copy_base(slug::builtin_slug(name).unwrap_or(from), &by_name, lookup, copied)
}

/// A version 0 theme as a version 1 one: the theme its `extends` chain leads to copied into
/// the `base`, its own values on top, `id` as given. `slug_of` is the slug of a theme named
/// in an `extends` (a user theme's id, or a built-in's). A file that is already version 1 is
/// returned as it is.
pub fn convert_to_v1(
    file: &ThemeFile,
    lookup: Lookup<'_>,
    slug_of: &dyn Fn(&str) -> String,
    id: &str,
    copied: &str,
) -> Result<ThemeFile, String> {
    if file.version == Some(CURRENT_VERSION) {
        return Ok(file.clone());
    }
    // Without an `extends` a version 0 theme stands on Tokyo Night; the base says so.
    let root = |extends: Option<&str>| -> Result<BaseLayer, String> {
        let name = extends.unwrap_or("Tokyo Night");
        let by_name = ThemeFile { extends: Some(name.to_string()), ..ThemeFile::default() };
        copy_base(&slug_of(name), &by_name, lookup, copied)
    };
    let mut out = ThemeFile {
        version: Some(CURRENT_VERSION),
        id: Some(id.to_string()),
        schema: file.schema.clone(),
        name: file.name.clone(),
        appearance: file.appearance,
        terminal: file.terminal.clone(),
        ui: file.ui.clone(),
        syntax: file.syntax.clone(),
        metrics: file.metrics.clone(),
        ..ThemeFile::default()
    };
    out.base = Some(Box::new(root(file.extends.as_deref())?));
    for v in &file.variants {
        out.variants.push(convert_variant(file, v, &root)?);
    }
    // Names are lowercase `[a-z0-9_]` in version 1; two that collapse to one are numbered.
    let mut seen: Vec<String> = Vec::new();
    for v in &mut out.variants {
        let mut name = part_from(&v.name, "version");
        let mut n = 2;
        while seen.contains(&name) {
            name = format!("{}_{n}", part_from(&v.name, "version"));
            n += 1;
        }
        seen.push(name.clone());
        v.name = name;
    }
    Ok(out)
}

/// A version of a version 0 file: the versions it extends (by name) merged under its own
/// layers; what the last of them extends outside the file becomes its own base.
fn convert_variant(
    file: &ThemeFile,
    v: &ThemeVariant,
    root: &dyn Fn(Option<&str>) -> Result<BaseLayer, String>,
) -> Result<ThemeVariant, String> {
    let family = file.name.as_deref().unwrap_or("Theme");
    let sibling = |name: &str| {
        file.variants.iter().find(|w| {
            w.name.eq_ignore_ascii_case(name)
                || super::super::variant_name(family, &w.name).eq_ignore_ascii_case(name)
        })
    };
    let mut chain = vec![v];
    let mut outside = None;
    let mut next = v.extends.as_deref();
    while let Some(name) = next {
        let Some(w) = sibling(name) else {
            outside = Some(name);
            break;
        };
        if chain.iter().any(|seen| std::ptr::eq(*seen, w)) {
            return Err(format!("theme {family:?}: versions extend each other"));
        }
        chain.push(w);
        next = w.extends.as_deref();
    }
    let mut layers = ThemeFile::default();
    for layer in chain.iter().rev() {
        layers = ThemeFile { extends: None, ..layer.to_file() }.layered_over(&layers);
    }
    let mut out = ThemeVariant { name: v.name.clone(), ..ThemeVariant::default() };
    out.set_layers(&ThemeFile { extends: None, name: None, ..layers });
    if let Some(name) = outside {
        out.base = Some(Box::new(root(Some(name))?));
    }
    Ok(out)
}

/// One value that is different in the source now from what a base copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseChange {
    /// The key as a version 1 file writes it: `terminal.ansi.3`, `ui.primary`,
    /// `syntax.keyword`, `metrics.radius.sm`, `appearance`.
    pub key: String,
    /// What the stored base has (`None`: it has none).
    pub old: Option<String>,
    /// What the source has now (`None`: it no longer sets it).
    pub new: Option<String>,
    /// The theme's own blocks set this key, so updating the base changes nothing the person
    /// sees: "yours, kept".
    pub kept: bool,
}

/// What would change if `stored` were replaced by `now` (a fresh [`copy_base`] of the same
/// source), key by key in key order; `own` is the theme whose base it is.
pub fn diff_base(stored: &BaseLayer, now: &BaseLayer, own: &ThemeFile) -> Vec<BaseChange> {
    let (old, new, mine) = (flat(&stored.file), flat(&now.file), flat(own));
    let keys: std::collections::BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    keys.into_iter()
        .filter(|k| old.get(*k) != new.get(*k))
        .map(|k| BaseChange {
            key: k.clone(),
            old: old.get(k).cloned(),
            new: new.get(k).cloned(),
            kept: mine.contains_key(k),
        })
        .collect()
}

/// Every value a file's layers set, by its version 1 key, as text.
fn flat(file: &ThemeFile) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let show = |v: &str| super::v1::expression_text(v);
    if let Some(a) = file.appearance {
        out.insert("appearance".into(), if a == Appearance::Light { "light" } else { "dark" }.into());
    }
    let named = |t: &str| super::keys::new_key("terminal", t).unwrap_or(t).to_string();
    for (node, value) in file.terminal.entries().unwrap_or_default() {
        let key = &node["terminal.".len()..];
        let key = match key.strip_prefix("ansi.") {
            Some(i) => ansi_index(i).map_or_else(|| key.to_string(), |i| format!("ansi.{i}")),
            None => named(key),
        };
        out.insert(format!("terminal.{key}"), show(&value));
    }
    for (k, v) in &file.ui {
        out.insert(format!("ui.{k}"), show(v));
    }
    for (k, v) in &file.syntax {
        let text = match v {
            SyntaxOverride::Color(c) => show(c),
            SyntaxOverride::Style { color, bold, italic } => {
                let part = |name: &str, v: Option<String>| v.map(|v| format!("{name} {v}"));
                [
                    part("color", color.as_deref().map(show)),
                    part("bold", bold.map(|b| b.to_string())),
                    part("italic", italic.map(|b| b.to_string())),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ")
            }
        };
        out.insert(format!("syntax.{k}"), text);
    }
    for key in super::keys::METRIC_ORDER {
        let Some(internal) = super::keys::metric_internal(key) else { continue };
        let value = match internal {
            "ui_font_family" => file.metrics.ui_font_family.clone(),
            _ => file.metrics.get(internal).map(|v| v.to_string()),
        };
        if let Some(v) = value {
            out.insert(format!("metrics.{key}"), v);
        }
    }
    out
}
