//! Theme files that ship several versions of one theme (light, dark, a minimal one, …). The
//! file's own layers are shared; each entry of its `variants` is a theme of its own, named
//! "<the file's name> <the variant's name>", made of the variant's layers over the shared ones.
//! A variant may `extends` another variant of the file by name, and through it the shared
//! layers; an `extends` no variant answers to names any theme, as a file's own does.
//!
//! Everything else sees plain theme files: [`variants`] folds each version into one, so
//! `extends`, the picker, the Studio and `settings.theme` need nothing new.

use super::file::{ThemeFile, ThemeVariant};
use super::format::{CURRENT_VERSION, slug};

/// Most versions one file may ship. A family has a handful; the cap bounds the work an
/// untrusted file (a project's `.wormhole/theme.json`) can cause.
pub const MAX_VARIANTS: usize = 32;

/// One version of a theme file, folded into a theme of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    /// "Soltari Light": what selects it and what the picker calls it.
    pub name: String,
    /// "Light": the variant's own name, what its chip says.
    pub short: String,
    /// Its layers over the siblings it extends and the file's shared ones, with what the last
    /// of them extends outside the file.
    pub file: ThemeFile,
    /// Version 1: the slug that selects it (`my_theme.dark`), when the file has an `id`.
    pub slug: Option<String>,
}

/// The name a variant `short` of the theme `family` is selected by.
pub fn variant_name(family: &str, short: &str) -> String {
    format!("{family} {short}")
}

/// The versions `file` ships, in its order, each folded into one theme file (empty when it
/// ships none: it is one theme). `family` names them when the file has no `name`.
pub fn variants(file: &ThemeFile, family: &str) -> Result<Vec<Variant>, String> {
    if file.variants.is_empty() {
        return Ok(Vec::new());
    }
    let family = file.name.as_deref().filter(|n| !n.trim().is_empty()).unwrap_or(family);
    if file.variants.len() > MAX_VARIANTS {
        return Err(format!("theme {family:?}: more than {MAX_VARIANTS} variants"));
    }
    for (ix, v) in file.variants.iter().enumerate() {
        if v.name.trim().is_empty() {
            return Err(format!("theme {family:?}: variant {} has no name", ix + 1));
        }
        if file.variants[..ix].iter().any(|w| w.name.eq_ignore_ascii_case(&v.name)) {
            return Err(format!("theme {family:?}: two variants are called {:?}", v.name));
        }
    }
    let shared = ThemeFile { variants: Vec::new(), ..file.clone() };
    if file.version == Some(CURRENT_VERSION) {
        return file.variants.iter().map(|v| fold_v1(&shared, v, family)).collect();
    }
    file.variants.iter().map(|v| fold(&shared, &file.variants, v, family)).collect()
}

/// What a version 1 version is called where people read it: its own name, lowercase in the
/// file, with a capital and spaces (`light_minimal` is "Light Minimal").
pub fn display_short(short: &str) -> String {
    short
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A version 1 version: the base it brings (or else the file's), the file's own layers, then
/// its own. A version has no `extends`: what it starts from is a base.
fn fold_v1(shared: &ThemeFile, v: &ThemeVariant, family: &str) -> Result<Variant, String> {
    let under = v.base.as_deref().or(shared.base.as_deref());
    let shared_own = ThemeFile { base: None, ..shared.clone() };
    let mut folded = match under {
        Some(base) => {
            let base = ThemeFile { extends: None, base: None, variants: Vec::new(), ..base.file.clone() };
            shared_own.layered_over(&base)
        }
        None => shared_own,
    };
    folded = ThemeFile { extends: None, ..v.to_file() }.layered_over(&folded);
    let short = display_short(&v.name);
    let name = variant_name(family, &short);
    folded.name = Some(name.clone());
    folded.version = Some(CURRENT_VERSION);
    folded.base = None;
    let slug = shared.id.as_ref().map(|id| slug::version_slug(id, &v.name));
    folded.id = slug.clone();
    Ok(Variant { name, short: v.name.clone(), file: folded, slug })
}

/// `v` over the siblings it extends, nearest last, over `shared`.
fn fold(shared: &ThemeFile, all: &[ThemeVariant], v: &ThemeVariant, family: &str) -> Result<Variant, String> {
    let sibling = |name: &str| {
        all.iter().find(|w| {
            w.name.eq_ignore_ascii_case(name) || variant_name(family, &w.name).eq_ignore_ascii_case(name)
        })
    };
    let mut chain = vec![v];
    let mut outside = None;
    let mut next = v.extends.as_deref();
    while let Some(name) = next {
        let Some(w) = sibling(name) else {
            outside = Some(name.to_string());
            break;
        };
        if chain.iter().any(|seen| std::ptr::eq(*seen, w)) {
            let path: Vec<&str> = chain.iter().map(|w| w.name.as_str()).chain([w.name.as_str()]).collect();
            return Err(format!("theme {family:?}: variants extend each other: {}", path.join(" -> ")));
        }
        chain.push(w);
        next = w.extends.as_deref();
    }
    let mut folded = ThemeFile { extends: outside.or_else(|| shared.extends.clone()), ..shared.clone() };
    for layer in chain.iter().rev() {
        // A layer's `extends` was followed above; what it names outside the file is kept.
        let own = ThemeFile { extends: None, ..layer.to_file() };
        folded = own.layered_over(&folded);
    }
    let name = variant_name(family, &v.name);
    folded.name = Some(name.clone());
    Ok(Variant { name, short: v.name.clone(), file: folded, slug: None })
}

#[cfg(test)]
mod tests {
    use super::{MAX_VARIANTS, variants};
    use crate::theme::tokens::test_support::no_lookup;
    use crate::theme::tokens::{Appearance, parse_theme_file, resolve};

    const FAMILY: &str = r##"{
      "name": "Soltari",
      "extends": "Nord",
      "ui": { "primary": "#c08a1e" },
      "syntax": { "keyword": { "italic": true } },
      "variants": [
        { "name": "Dark", "appearance": "dark", "terminal": { "background": "#15131c" } },
        { "name": "Light", "appearance": "light",
          "terminal": { "background": "#f6f2ea", "foreground": "#2b2a35" } },
        { "name": "Light Minimal", "extends": "Light", "metrics": { "radius": 2 } },
        { "name": "Tokyo", "extends": "Tokyo Night" },
      ],
    }"##;

    #[test]
    fn each_variant_is_a_theme_over_the_shared_layers() {
        let file = parse_theme_file(FAMILY).unwrap();
        let all = variants(&file, "stem").unwrap();
        let names: Vec<&str> = all.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["Soltari Dark", "Soltari Light", "Soltari Light Minimal", "Soltari Tokyo"]);
        assert_eq!(all[2].short, "Light Minimal");
        let tokens: Vec<_> = all.iter().map(|v| resolve(&v.file, &no_lookup).unwrap()).collect();
        assert_eq!(tokens[0].name, "Soltari Dark");
        assert_eq!(tokens[0].terminal.background.to_hex(), "#15131c");
        // The shared primary and syntax style reach every version.
        for t in &tokens {
            assert_eq!(t.get("primary").unwrap().to_hex(), "#c08a1e", "{}", t.name);
            assert!(t.syntax["keyword"].italic, "{}", t.name);
        }
        // Light Minimal is Light and its own radius.
        assert_eq!(tokens[2].appearance, Appearance::Light);
        assert_eq!(tokens[2].terminal, tokens[1].terminal);
        assert_eq!((tokens[1].metrics.radius, tokens[2].metrics.radius), (6.0, 2.0));
        // The file's `extends` is the base of each version; a variant's own replaces it.
        assert_eq!(all[0].file.extends.as_deref(), Some("Nord"));
        assert_eq!(all[2].file.extends.as_deref(), Some("Nord"));
        assert_eq!(all[3].file.extends.as_deref(), Some("Tokyo Night"));
        assert!(all.iter().all(|v| v.file.variants.is_empty()));
    }

    #[test]
    fn a_file_without_variants_is_one_theme_and_an_unnamed_family_takes_its_stem() {
        let plain = parse_theme_file(r#"{"name": "Plain"}"#).unwrap();
        assert!(variants(&plain, "plain").unwrap().is_empty());
        let unnamed = parse_theme_file(r#"{"variants": [{"name": "Dark"}]}"#).unwrap();
        assert_eq!(variants(&unnamed, "mine").unwrap()[0].name, "mine Dark");
        // Resolving a family file directly gives its first version.
        let t = resolve(&parse_theme_file(FAMILY).unwrap(), &no_lookup).unwrap();
        assert_eq!(t.name, "Soltari Dark");
    }

    #[test]
    fn a_sibling_answers_to_its_own_name_or_its_full_one() {
        let file = parse_theme_file(
            r##"{"name": "F", "variants": [
                {"name": "A", "ui": {"primary": "#112233"}},
                {"name": "B", "extends": "F A"},
                {"name": "C", "extends": "b"}
            ]}"##,
        )
        .unwrap();
        for v in variants(&file, "F").unwrap() {
            let t = resolve(&v.file, &no_lookup).unwrap();
            assert_eq!(t.get("primary").unwrap().to_hex(), "#112233", "{}", v.name);
        }
    }

    #[test]
    fn bad_variants_say_what_is_wrong() {
        let err = |text: &str| variants(&parse_theme_file(text).unwrap(), "F").unwrap_err();
        let e = err(
            r#"{"name": "F", "variants": [{"name": "A", "extends": "B"}, {"name": "B", "extends": "A"}]}"#,
        );
        assert!(e.contains("variants extend each other: A -> B -> A"), "{e}");
        let e = err(r#"{"name": "F", "variants": [{"name": "Dark"}, {"name": "dark"}]}"#);
        assert!(e.contains("two variants are called \"dark\""), "{e}");
        let e = err(r#"{"name": "F", "variants": [{"name": " "}]}"#);
        assert!(e.contains("variant 1 has no name"), "{e}");
        let many: Vec<String> = (0..=MAX_VARIANTS).map(|i| format!(r#"{{"name": "V{i}"}}"#)).collect();
        let e = err(&format!(r#"{{"name": "F", "variants": [{}]}}"#, many.join(",")));
        assert!(e.contains("more than 32 variants"), "{e}");
        // A variant is read as strictly as a theme.
        let e = parse_theme_file(r#"{"variants": [{"name": "A", "variants": []}]}"#).unwrap_err();
        assert!(e.contains("unknown field `variants`"), "{e}");
    }
}
