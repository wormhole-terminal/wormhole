//! Where in a theme file a value lives, and where it comes from. A file has its top (shared by
//! its appearances) and, as a pair, a variant per appearance; under both sits a `base`, the values
//! the theme was copied from. A value lives in exactly one of the top and the shown variant: the
//! chip on a row says which, and moving it is one change. Pure, tested without a window.

use wormhole_core::theme::tokens::appearance::{Scope, token_scope};
use wormhole_core::theme::tokens::{BaseLayer, ThemeFile, variants};

use super::{override_of, set_override};

/// Where a value is written in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::studio) enum Place {
    /// The top of the file: both appearances.
    Top,
    /// The variant at this index of `variants`: its appearance alone.
    Variant(usize),
}

/// Where a token's value comes from, as the list says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::studio) enum Origin {
    /// The theme sets it itself.
    Yours,
    /// Copied from the theme it started as, and not changed.
    Base,
    /// Neither: Wormhole's rules work it out from other tokens.
    WorkedOut,
}

/// The chip on a row of a pair: which appearances a value is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::studio) enum Reach {
    Both,
    ThisAppearance,
}

/// The index of the variant called `shown`.
pub(in crate::studio) fn variant_at(file: &ThemeFile, shown: Option<&str>) -> Option<usize> {
    let shown = shown?;
    file.variants.iter().position(|v| v.name == shown)
}

/// Whether the file is a pair: a row says which appearances a value is for.
pub(in crate::studio) fn is_pair(file: &ThemeFile) -> bool {
    file.variants.len() > 1
}

/// The value the file sets at `place`, as written.
pub(in crate::studio) fn read_at(file: &ThemeFile, place: Place, name: &str) -> Option<String> {
    match place {
        Place::Top => override_of(file, name),
        Place::Variant(ix) => override_of(&file.variants.get(ix)?.to_file(), name),
    }
}

/// Set (or with `None`, remove) the value at `place`.
pub(in crate::studio) fn write_at(
    file: &mut ThemeFile,
    place: Place,
    name: &str,
    value: Option<String>,
) -> Result<(), String> {
    match place {
        Place::Top => set_override(file, name, value),
        Place::Variant(ix) => {
            let Some(variant) = file.variants.get_mut(ix) else { return Err("no such appearance".into()) }; // text: error (logged, not shown)
            let mut layers = variant.to_file();
            set_override(&mut layers, name, value)?;
            // The variant's own base is not one of the layers `to_file` carries.
            let base = variant.base.take();
            variant.set_layers(&layers);
            variant.base = base;
            Ok(())
        }
    }
}

/// The value the person sees as theirs for `name` while `shown` is edited: the shown appearance's,
/// else the top's.
pub(in crate::studio) fn own_value(file: &ThemeFile, shown: Option<&str>, name: &str) -> Option<String> {
    variant_at(file, shown)
        .and_then(|ix| read_at(file, Place::Variant(ix), name))
        .or_else(|| read_at(file, Place::Top, name))
}

/// The base under `shown`: its variant's own, else the file's.
pub(in crate::studio) fn base_under<'a>(file: &'a ThemeFile, shown: Option<&str>) -> Option<&'a BaseLayer> {
    variant_at(file, shown).and_then(|ix| file.variants[ix].base.as_deref()).or(file.base.as_deref())
}

/// The base under `shown` replaced by `base`: its variant's own, else the file's.
pub(in crate::studio) fn set_base(file: &mut ThemeFile, shown: Option<&str>, base: BaseLayer) {
    match variant_at(file, shown) {
        Some(ix) if file.variants[ix].base.is_some() => file.variants[ix].base = Some(Box::new(base)),
        _ => file.base = Some(Box::new(base)),
    }
}

/// What the theme itself sets under `shown`, the top and the shown appearance together, without a
/// base: what an update of the base would leave as it is.
pub(in crate::studio) fn own_layers(file: &ThemeFile, shown: Option<&str>) -> ThemeFile {
    let top = ThemeFile { base: None, variants: Vec::new(), ..file.clone() };
    match variant_at(file, shown) {
        Some(ix) => ThemeFile { extends: None, ..file.variants[ix].to_file() }.layered_over(&top),
        None => top,
    }
}

/// The value the base has for `name`.
pub(in crate::studio) fn base_value(file: &ThemeFile, shown: Option<&str>, name: &str) -> Option<String> {
    override_of(&base_under(file, shown)?.file, name)
}

pub(in crate::studio) fn origin(file: &ThemeFile, shown: Option<&str>, name: &str) -> Origin {
    if own_value(file, shown, name).is_some() {
        Origin::Yours
    } else if base_value(file, shown, name).is_some() {
        Origin::Base
    } else {
        Origin::WorkedOut
    }
}

/// Where `name`'s value is, or would go if it were set: where it is set already (the shown
/// appearance's wins), else where a value of its kind goes by default.
pub(in crate::studio) fn place_of(file: &ThemeFile, shown: Option<&str>, name: &str) -> Place {
    let Some(ix) = variant_at(file, shown).filter(|_| is_pair(file)) else { return Place::Top };
    if read_at(file, Place::Variant(ix), name).is_some() {
        Place::Variant(ix)
    } else if read_at(file, Place::Top, name).is_some() || token_scope(name) == Scope::Both {
        Place::Top
    } else {
        Place::Variant(ix)
    }
}

pub(in crate::studio) fn reach(file: &ThemeFile, shown: Option<&str>, name: &str) -> Reach {
    match place_of(file, shown, name) {
        Place::Top => Reach::Both,
        Place::Variant(_) => Reach::ThisAppearance,
    }
}

/// Set `name` where it belongs (see [`place_of`]); with `None`, remove it from everywhere the
/// shown appearance would read it from: back to the base.
pub(in crate::studio) fn set_own(
    file: &mut ThemeFile,
    shown: Option<&str>,
    name: &str,
    value: Option<String>,
) -> Result<(), String> {
    if value.is_some() {
        let place = place_of(file, shown, name);
        return write_at(file, place, name, value);
    }
    if let Some(ix) = variant_at(file, shown) {
        write_at(file, Place::Variant(ix), name, None)?;
    }
    write_at(file, Place::Top, name, None)
}

/// What moving a value between "both" and "this appearance" did to the other appearances.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::studio) struct Moved {
    /// Appearances whose own value for the token was removed (name, value): the shown one's kept.
    pub(in crate::studio) removed: Vec<(String, String)>,
}

/// Move the value of `name` between both appearances and the shown one. Both to one: the other
/// appearances keep the look they have (they get the value themselves), so nothing else changes.
/// One to both: the shown value is kept for all, and another appearance's own value is removed
/// and reported. A token with no value of its own gets `current` (what it works out to) in the
/// place it was not.
pub(in crate::studio) fn move_value(
    file: &mut ThemeFile,
    shown: Option<&str>,
    name: &str,
    current: &str,
) -> Result<Moved, String> {
    let Some(here) = variant_at(file, shown).filter(|_| is_pair(file)) else { return Ok(Moved::default()) };
    let mut moved = Moved::default();
    match place_of(file, shown, name) {
        Place::Top => {
            let value = read_at(file, Place::Top, name).unwrap_or_else(|| current.to_string());
            for ix in 0..file.variants.len() {
                if read_at(file, Place::Variant(ix), name).is_none() {
                    write_at(file, Place::Variant(ix), name, Some(value.clone()))?;
                }
            }
            write_at(file, Place::Top, name, None)?;
        }
        Place::Variant(_) => {
            let value = read_at(file, Place::Variant(here), name).unwrap_or_else(|| current.to_string());
            for ix in 0..file.variants.len() {
                if let Some(other) = read_at(file, Place::Variant(ix), name) {
                    if ix != here && other != value {
                        moved.removed.push((file.variants[ix].name.clone(), other));
                    }
                    write_at(file, Place::Variant(ix), name, None)?;
                }
            }
            write_at(file, Place::Top, name, Some(value))?;
        }
    }
    Ok(moved)
}

/// The file with the layers `shown` is made of folded into one theme, for resolving: the file
/// itself when it has no appearances.
pub(in crate::studio) fn folded(file: &ThemeFile, shown: Option<&str>) -> ThemeFile {
    if file.variants.is_empty() {
        return file.clone();
    }
    let family = file.name.as_deref().unwrap_or("Theme"); // text: not shown (a name for the folded variants)
    variants(file, family)
        .ok()
        .and_then(|all| {
            let ix = variant_at(file, shown).unwrap_or(0);
            all.into_iter().nth(ix).map(|v| v.file)
        })
        .unwrap_or_else(|| file.clone())
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::tokens::appearance::add_appearance;
    use wormhole_core::theme::tokens::{Appearance, ThemeTokens, builtin_tokens, copy, resolve};

    use super::*;

    fn pair() -> ThemeFile {
        let base = copy::copy_builtin_base("wormhole.dark", &|_| None, "2026-10-29").expect("base");
        let mut file = ThemeFile {
            version: Some(1),
            id: Some("mine".into()),
            name: Some("Mine".into()),
            base: Some(Box::new(base)),
            ..ThemeFile::default()
        };
        file.ui.insert("primary".into(), "#ff8800".into());
        let have = resolve(&file, &|_| None).expect("resolves");
        add_appearance(&file, &have, Appearance::Light, &|_| None, "2026-10-30").expect("a pair")
    }

    fn look(file: &ThemeFile, shown: &str) -> ThemeTokens {
        resolve(&folded(file, Some(shown)), &|_| None).expect("resolves")
    }

    #[test]
    fn a_value_is_yours_base_or_worked_out() {
        let mut file = pair();
        assert_eq!(origin(&file, Some("dark"), "primary"), Origin::Yours);
        assert_eq!(origin(&file, Some("dark"), "terminal.ansi.1"), Origin::Base);
        assert_eq!(origin(&file, Some("dark"), "surface.window"), Origin::WorkedOut);
        set_own(&mut file, Some("dark"), "terminal.ansi.1", Some("#aa0000".into())).expect("sets");
        assert_eq!(origin(&file, Some("dark"), "terminal.ansi.1"), Origin::Yours);
        assert_eq!(origin(&file, Some("light"), "terminal.ansi.1"), Origin::Base, "the other one's own");
        set_own(&mut file, Some("dark"), "terminal.ansi.1", None).expect("resets");
        assert_eq!(
            origin(&file, Some("dark"), "terminal.ansi.1"),
            Origin::Base,
            "reset goes back to the base"
        );
        let single = ThemeFile::default();
        assert_eq!(origin(&single, None, "primary"), Origin::WorkedOut);
    }

    #[test]
    fn a_new_value_goes_where_its_kind_goes_and_stays_where_it_is() {
        let mut file = pair();
        set_own(&mut file, Some("light"), "surface.panel", Some("#eeeeee".into())).expect("sets");
        assert_eq!(reach(&file, Some("light"), "surface.panel"), Reach::ThisAppearance);
        assert_eq!(read_at(&file, Place::Top, "surface.panel"), None);
        assert_eq!(reach(&file, Some("dark"), "primary"), Reach::Both, "the accent is shared");
        assert_eq!(reach(&file, Some("dark"), "metrics.radius"), Reach::Both);
        set_own(&mut file, Some("dark"), "primary", Some("#00ff00".into())).expect("sets");
        assert_eq!(look(&file, "light").get("primary").map(|c| c.to_hex()).as_deref(), Some("#00ff00"));
    }

    #[test]
    fn moving_to_this_appearance_keeps_both_looks() {
        let mut file = pair();
        let before = (look(&file, "light"), look(&file, "dark"));
        move_value(&mut file, Some("dark"), "primary", "#ff8800").expect("moves");
        assert_eq!(reach(&file, Some("dark"), "primary"), Reach::ThisAppearance);
        assert_eq!(read_at(&file, Place::Top, "primary"), None, "a value lives in one place");
        assert_eq!((look(&file, "light"), look(&file, "dark")), before);
        // Now only the dark one changes.
        set_own(&mut file, Some("dark"), "primary", Some("#0000ff".into())).expect("sets");
        assert_eq!(look(&file, "light").get("primary").map(|c| c.to_hex()).as_deref(), Some("#ff8800"));
    }

    #[test]
    fn moving_to_both_keeps_the_shown_value_and_reports_the_one_it_removed() {
        let mut file = pair();
        move_value(&mut file, Some("dark"), "primary", "#ff8800").expect("moves");
        set_own(&mut file, Some("dark"), "primary", Some("#0000ff".into())).expect("sets");
        let moved = move_value(&mut file, Some("dark"), "primary", "").expect("moves");
        assert_eq!(moved.removed, [("light".to_string(), "#ff8800".to_string())]);
        assert_eq!(read_at(&file, Place::Top, "primary").as_deref(), Some("#0000ff"));
        assert_eq!(look(&file, "light").get("primary").map(|c| c.to_hex()).as_deref(), Some("#0000ff"));
        // Without a clash nothing is reported.
        move_value(&mut file, Some("light"), "primary", "").expect("moves");
        let moved = move_value(&mut file, Some("light"), "primary", "").expect("moves");
        assert!(moved.removed.is_empty());
    }

    #[test]
    fn a_token_with_no_value_of_its_own_gets_what_it_works_out_to() {
        let mut file = pair();
        let tokens = builtin_tokens();
        assert!(!tokens.is_empty());
        move_value(&mut file, Some("dark"), "surface.window", "#123456").expect("moves");
        assert_eq!(read_at(&file, Place::Top, "surface.window").as_deref(), Some("#123456"));
    }
}
