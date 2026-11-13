//! What the Theme Studio asks of core about theme *files*: reading one in whatever format it is in, writing
//! the format this release writes (version 1: a `base` of copied values under the person's own blocks), making
//! a new theme, a copy of another, a pair out of a single theme, and comparing a copy's base with its source.
//! Every rule is core's (`theme::tokens::format`, the Studio's own `document` and `themes` of the GPUI app are
//! written over the same functions); text goes in and text comes out, so the Swift draft stays a small JSON
//! mirror of the file and never decides what a version, a base or an id is. Nothing here writes a file.

use std::path::Path;
use std::time::SystemTime;

use wormhole_core::config::resolve_theme_file_in;
use wormhole_core::theme::catalog::{ThemeEntry, cards, catalog_in};
use wormhole_core::theme::registry::{ThemeId, ThemeSource as _, UserThemes};
use wormhole_core::theme::tokens::appearance::{self, as_variant_base, token_scope};
use wormhole_core::theme::tokens::copy::{self, diff_base};
use wormhole_core::theme::tokens::{
    Appearance, BaseLayer, CURRENT_VERSION, KEY_TABLE, ThemeFile, read_theme_file, slug, starter, variants,
    write_v1,
};
use wormhole_text::keys::studio_date;

use crate::error::FfiError;
use crate::isolation;

fn theme_error(reason: String) -> FfiError {
    FfiError::Theme { reason }
}

/// What reading a theme file had to do, for the Studio to say (`ReadReport`).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioRead {
    /// The version the file says it is: 0 when it says none.
    pub version: u32,
    /// Made by a newer Wormhole than this one: the Studio never writes it back.
    pub newer: bool,
    /// Entries of a newer file this version does not know.
    pub skipped: u32,
    pub notices: Vec<String>,
}

/// A key version 1 spells differently from the resolver's name: `terminal` `cursor_text` is `cursor.text`.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioKeyRename {
    pub block: String,
    pub old: String,
    pub new: String,
}

/// One value that is different in the source now from what a base copied.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioUpdateChange {
    /// `terminal.ansi.3`, `ui.primary`, `metrics.radius.sm`.
    pub key: String,
    pub old: Option<String>,
    pub new: Option<String>,
    /// The theme sets it itself, so updating changes nothing the person sees.
    pub kept: bool,
}

/// Whether a theme copied from another has something to update from, and what.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioUpdate {
    /// `none` (no base, or one that was never copied), `missing` (the source is gone), `current`, `changed`.
    pub state: String,
    /// The source's name as people read it.
    pub from: String,
    /// The day the base was copied, "29 Oct 2026".
    pub date: String,
    pub changes: Vec<StudioUpdateChange>,
}

/// How a file reads and what its reading had to do. An error is core's sentence (`line L, column C: ...`).
#[uniffi::export]
pub fn studio_read(text: String) -> Result<StudioRead, FfiError> {
    let file = read_theme_file(&text).map_err(theme_error)?;
    Ok(StudioRead {
        version: file.read.source_version,
        newer: file.read.newer,
        skipped: u32::try_from(file.read.skipped).unwrap_or(u32::MAX),
        notices: file.read.notices,
    })
}

/// Every key version 1 spells differently, so a version 1 file's own blocks can be read and edited by token name.
#[uniffi::export]
pub fn studio_key_table() -> Vec<StudioKeyRename> {
    KEY_TABLE
        .iter()
        .map(|r| StudioKeyRename { block: r.block.into(), old: r.old.into(), new: r.new.into() })
        .collect()
}

/// Whether a new value of the token goes to both appearances of a pair (the primary, the metrics, syntax) rather
/// than the appearance being edited.
#[uniffi::export]
pub fn studio_token_in_both(name: String) -> bool {
    token_scope(&name) == appearance::Scope::Both
}

/// The file as this release writes it: version 1 text in the fixed order and spelling core reads; a file of
/// the older format is returned as it is (the Studio converts it with `studio_convert` on its first save).
#[uniffi::export]
pub fn studio_canonical(text: String) -> Result<String, FfiError> {
    let file = read_theme_file(&text).map_err(theme_error)?;
    Ok(if file.version.is_some() { write_v1(&file) } else { text })
}

/// A new theme from nothing: the neutral palette of `dark` or light, no base, called `name`.
#[uniffi::export]
pub fn studio_new_theme(dark: bool, name: String) -> String {
    let appearance = if dark { Appearance::Dark } else { Appearance::Light };
    write_v1(&ThemeFile { name: Some(name), ..starter::starter(appearance) })
}

/// A new theme called `name` that has the theme `from` copied into its `base`: a built-in's name or slug, or a
/// theme of the folder by slug or path. `None` when there is nothing to copy it from (the Studio then copies
/// its palette by name).
#[uniffi::export]
pub fn studio_copy_theme(themes_dir: String, from: String, name: String) -> Option<String> {
    let base = fresh_base(Path::new(&themes_dir), &from)?;
    let file = ThemeFile {
        version: Some(CURRENT_VERSION),
        name: Some(name),
        base: Some(Box::new(base)),
        ..ThemeFile::default()
    };
    Some(write_v1(&file))
}

/// The id a theme called `name` gets: nobody else's and no built-in's. The file at `itself` (when it exists) is
/// the theme itself, whose own id does not count against it.
#[uniffi::export]
pub fn studio_new_id(themes_dir: String, name: String, itself: Option<String>) -> String {
    let itself = itself.as_deref().map(Path::new);
    let taken: Vec<String> = UserThemes::read(Path::new(&themes_dir))
        .themes
        .into_iter()
        .filter(|t| Some(t.path.as_path()) != itself)
        .map(|t| t.slug)
        .collect();
    let wanted = slug::derive_id(&name);
    std::iter::once(wanted.clone())
        .chain((2..).map(|n| format!("{wanted}_{n}")))
        .find(|id| !slug::is_reserved(id) && !taken.contains(id))
        .unwrap_or(wanted)
}

/// A theme of the older format as one in the new, with `id`: its chain copied into the `base`, its own values on
/// top. Refused unless it resolves to the very same theme afterwards, so nothing on screen changes. A file that
/// is version 1 already is returned in canonical form.
#[uniffi::export]
pub fn studio_convert(themes_dir: String, text: String, id: String) -> Result<String, FfiError> {
    let dir = Path::new(&themes_dir);
    let file = read_theme_file(&text).map_err(theme_error)?;
    Ok(write_v1(&converted(dir, &file, &id).map_err(theme_error)?))
}

/// A single theme made a pair: what it is becomes one appearance, and `to` (`light` or `dark`) is added from the
/// counterpart of its base, or generated. An older file is converted first (with `id`).
#[uniffi::export]
pub fn studio_add_appearance(
    themes_dir: String,
    text: String,
    dark: bool,
    id: String,
) -> Result<String, FfiError> {
    let dir = Path::new(&themes_dir);
    let file = read_theme_file(&text).map_err(theme_error)?;
    let single = converted(dir, &file, &id).map_err(theme_error)?;
    let have = resolve_theme_file_in(dir, &single).map_err(theme_error)?;
    let to = if dark { Appearance::Dark } else { Appearance::Light };
    let themes = UserThemes::read(dir);
    let pair = appearance::add_appearance(&single, &have, to, &|n| themes.theme_file(n), &today())
        .map_err(theme_error)?;
    Ok(write_v1(&pair))
}

/// Whether the base under `shown` (its variant's own, else the file's) has changed in its source since it was
/// copied, and what.
#[uniffi::export]
pub fn studio_update_check(
    themes_dir: String,
    text: String,
    shown: Option<String>,
) -> Result<StudioUpdate, FfiError> {
    let file = read_theme_file(&text).map_err(theme_error)?;
    let none = |state: &str| StudioUpdate {
        state: state.into(),
        from: String::new(),
        date: String::new(),
        changes: Vec::new(),
    };
    let Some(stored) = base_under(&file, shown.as_deref()) else { return Ok(none("none")) };
    let from = slug::builtin_name(&stored.from).map_or_else(|| stored.from.clone(), str::to_string);
    let named = |state: &str| StudioUpdate { from: from.clone(), ..none(state) };
    let Some(copied) = stored.copied.clone() else { return Ok(named("none")) };
    let Some((_, changes)) = fresh_changes(Path::new(&themes_dir), &file, shown.as_deref()) else {
        return Ok(named("missing"));
    };
    let state = if changes.is_empty() { "current" } else { "changed" };
    Ok(StudioUpdate { state: state.into(), date: display_date(&copied), changes, ..named(state) })
}

/// The file with the base under `shown` replaced by a fresh copy of its source (the "Update from" step).
#[uniffi::export]
pub fn studio_update_apply(
    themes_dir: String,
    text: String,
    shown: Option<String>,
) -> Result<String, FfiError> {
    let mut file = read_theme_file(&text).map_err(theme_error)?;
    let Some((fresh, _)) = fresh_changes(Path::new(&themes_dir), &file, shown.as_deref()) else {
        return Err(theme_error("the theme this one was copied from is gone".into()));
    };
    match variant_at(&file, shown.as_deref()) {
        Some(ix) if file.variants[ix].base.is_some() => file.variants[ix].base = Some(Box::new(fresh)),
        _ => file.base = Some(Box::new(fresh)),
    }
    Ok(write_v1(&file))
}

/// The theme as Export, Copy JSON and a project's `theme.json` write it, in the format every release reads: a
/// file of the older format as it is, a version 1 file as the one standalone theme the shown appearance works out to.
#[uniffi::export]
pub fn studio_export(themes_dir: String, text: String, shown: Option<String>) -> Result<String, FfiError> {
    let file = read_theme_file(&text).map_err(theme_error)?;
    if file.version.is_none() {
        return Ok(file.to_json_pretty());
    }
    let tokens = resolve_theme_file_in(Path::new(&themes_dir), &folded(&file, shown.as_deref()))
        .map_err(theme_error)?;
    Ok(wormhole_core::theme::tokens::export::write_standalone_v0(&tokens))
}

/// One theme the Studio's theme menu offers: `value` is the file of a theme of the person's, or the name of a
/// built-in to copy.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioThemeChoice {
    pub name: String,
    pub value: String,
}

/// A built-in theme, or a family of them (a light and a dark one) named by their appearance.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioThemeFamily {
    pub name: String,
    /// Wormhole's own (the accent families and Wormhole itself): the menu puts them under one name.
    pub wormhole: bool,
    pub choices: Vec<StudioThemeChoice>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioThemeList {
    pub own: Vec<StudioThemeChoice>,
    pub builtins: Vec<StudioThemeFamily>,
}

/// The themes the theme menu offers: the person's own (a file that ships several appearances once, by its
/// family's name, sorted), then the built-ins with a family's appearances together (the GPUI app's `theme_list`).
#[uniffi::export]
pub fn studio_theme_list(themes_dir: String) -> StudioThemeList {
    let catalog = catalog_in(Path::new(&themes_dir));
    let mut own = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for entry in catalog.iter().filter(|e| !e.builtin) {
        match &entry.id {
            ThemeId::File(path) => own.push(choice(entry.label.clone(), path)),
            ThemeId::Variant(path, _) if seen.insert(path.clone()) => {
                own.push(choice(entry.card_label().to_string(), path));
            }
            _ => {}
        }
    }
    own.sort_by_key(|c| c.name.to_lowercase());
    let mut builtins = Vec::new();
    for card in cards(&catalog) {
        let entries: Vec<&ThemeEntry> = card.iter().map(|&ix| &catalog[ix]).filter(|e| e.builtin).collect();
        let Some(first) = entries.first() else { continue };
        let name = first.card_label().to_string();
        let choices = entries
            .iter()
            .map(|e| StudioThemeChoice {
                name: e.family.as_ref().map_or_else(|| e.label.clone(), |(_, version)| version.clone()),
                value: e.label.clone(),
            })
            .collect();
        builtins.push(StudioThemeFamily {
            wormhole: name == "Wormhole" || name.starts_with("Wormhole "),
            name,
            choices,
        });
    }
    StudioThemeList { own, builtins }
}

fn choice(name: String, path: &Path) -> StudioThemeChoice {
    StudioThemeChoice { name, value: path.to_string_lossy().into_owned() }
}

/// The store key of the folder the last Export went to, so the next dialog starts there, across launches (the GPUI Studio's).
const EXPORT_DIR_KEY: &str = "studio_export_dir";

/// The folder the last Export went to, while it is still a folder; `None` before the first export or once it is gone.
#[uniffi::export]
pub fn studio_export_dir() -> Option<String> {
    isolation::require().ok()?;
    let dir = wormhole_core::storage::store::Store::open().ok()?.meta(EXPORT_DIR_KEY).ok().flatten()?;
    (Path::new(&dir).is_absolute() && Path::new(&dir).is_dir()).then_some(dir)
}

/// Remembers the folder an Export just went to.
#[uniffi::export]
pub fn studio_remember_export_dir(dir: String) {
    if isolation::require().is_err() {
        return;
    }
    let remembered = wormhole_core::storage::store::Store::open().and_then(|store| store.set_meta(EXPORT_DIR_KEY, &dir));
    if let Err(err) = remembered {
        tracing::warn!("remembering the export folder failed: {err:#}");
    }
}

// MARK: shared helpers (the same as the GPUI Studio's `document::places` and `themes`)

fn variant_at(file: &ThemeFile, shown: Option<&str>) -> Option<usize> {
    let shown = shown?;
    file.variants.iter().position(|v| v.name == shown)
}

fn base_under<'a>(file: &'a ThemeFile, shown: Option<&str>) -> Option<&'a BaseLayer> {
    variant_at(file, shown).and_then(|ix| file.variants[ix].base.as_deref()).or(file.base.as_deref())
}

/// What the theme itself sets under `shown`, without a base.
fn own_layers(file: &ThemeFile, shown: Option<&str>) -> ThemeFile {
    let top = ThemeFile { base: None, variants: Vec::new(), ..file.clone() };
    match variant_at(file, shown) {
        Some(ix) => ThemeFile { extends: None, ..file.variants[ix].to_file() }.layered_over(&top),
        None => top,
    }
}

/// The file with the layers `shown` is made of folded into one theme, for resolving.
pub(crate) fn folded(file: &ThemeFile, shown: Option<&str>) -> ThemeFile {
    if file.variants.is_empty() {
        return file.clone();
    }
    let family = file.name.as_deref().unwrap_or("Theme");
    variants(file, family)
        .ok()
        .and_then(|all| all.into_iter().nth(variant_at(file, shown).unwrap_or(0)).map(|v| v.file))
        .unwrap_or_else(|| file.clone())
}

fn converted(dir: &Path, file: &ThemeFile, id: &str) -> Result<ThemeFile, String> {
    if file.version.is_some() {
        return Ok(file.clone());
    }
    let themes = UserThemes::read(dir);
    let slug_of = |name: &str| match slug::builtin_slug(name) {
        Some(builtin) => builtin.to_string(),
        None => themes.named(name).map_or_else(|| slug::derive_id(name), |t| t.slug.clone()),
    };
    let made = copy::convert_to_v1(file, &|n| themes.theme_file(n), &slug_of, id, &today())?;
    let (before, after) = (resolve_theme_file_in(dir, file)?, resolve_theme_file_in(dir, &made)?);
    if before != after {
        return Err("the converted theme does not look the same".into());
    }
    Ok(made)
}

/// The base a copy of `from` would have now: a built-in's slug or name, the slug of a user theme, or the path of
/// its file.
pub(crate) fn fresh_base(dir: &Path, from: &str) -> Option<BaseLayer> {
    let themes = UserThemes::read(dir);
    let by_path = themes.by_path(from).map(|t| t.slug.clone());
    let from = by_path.as_deref().unwrap_or(from);
    let lookup = |name: &str| themes.theme_file(name);
    let slug_for = slug::builtin_slug(from).unwrap_or(from);
    if slug::builtin_name(slug_for).is_some() {
        return copy::copy_builtin_base(slug_for, &lookup, &today()).ok();
    }
    let by_slug = ThemeFile { extends: Some(from.to_string()), ..ThemeFile::default() };
    copy::copy_base(from, &by_slug, &lookup, &today()).ok()
}

/// The source's fresh base (made comparable to the stored one) and what differs.
fn fresh_changes(
    dir: &Path,
    file: &ThemeFile,
    shown: Option<&str>,
) -> Option<(BaseLayer, Vec<StudioUpdateChange>)> {
    let stored = base_under(file, shown)?;
    let mut fresh = fresh_base(dir, &stored.from)?;
    let mut stored = stored.clone();
    if file.variants.len() > 1 {
        let appearance = stored.file.appearance;
        as_variant_base(&mut stored, appearance);
        as_variant_base(&mut fresh, appearance);
    }
    let changes = diff_base(&stored, &fresh, &own_layers(file, shown))
        .into_iter()
        .map(|c| StudioUpdateChange { key: c.key, old: c.old, new: c.new, kept: c.kept })
        .collect();
    Some((fresh, changes))
}

fn display_date(day: &str) -> String {
    let mut parts = day.split('-');
    let (Some(y), Some(m), Some(d), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return day.to_string();
    };
    match (m.parse::<i64>(), d.parse::<u32>()) {
        (Ok(m @ 1..=12), Ok(d)) => studio_date(&d.to_string(), m, y),
        _ => day.to_string(),
    }
}

/// A day as `2026-10-29`, UTC, for the day a base was copied.
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, reason = "a test writing its own scratch theme folder")]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wh-studio-files-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn text(dir: &Path) -> String {
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn a_new_theme_is_version_one_with_nothing_under_it() {
        let made = studio_new_theme(true, "Mine".into());
        let read = studio_read(made.clone()).expect("reads");
        assert_eq!((read.version, read.newer), (1, false));
        assert!(made.contains("\"name\": \"Mine\"") && !made.contains("\"base\""));
        assert_eq!(studio_canonical(made.clone()).expect("canonical"), made, "written the way it is read");
    }

    #[test]
    fn a_copy_carries_its_source_in_a_base_and_looks_the_same() {
        let dir = folder("copy");
        let made =
            studio_copy_theme(text(&dir), "nord".into(), "Untitled (from Nord)".into()).expect("a copy");
        assert!(made.contains("\"base\"") && made.contains("\"from\": \"nord\""), "{made}");
        let file = read_theme_file(&made).expect("reads");
        let nord = resolve_theme_file_in(&dir, &read_theme_file("{\"extends\": \"Nord\"}").expect("v0"))
            .expect("nord");
        assert_eq!(resolve_theme_file_in(&dir, &file).expect("copy").terminal, nord.terminal);
        assert!(studio_copy_theme(text(&dir), "no such theme".into(), "x".into()).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_older_file_is_converted_to_version_one_without_changing_how_it_looks() {
        let dir = folder("convert");
        let old = r##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#ff6600"}}"##;
        let id = studio_new_id(text(&dir), "Mine".into(), None);
        assert_eq!(id, "mine");
        let v1 = studio_convert(text(&dir), old.into(), id).expect("converts");
        assert!(
            v1.contains("\"version\": 1") && v1.contains("\"id\": \"mine\"") && v1.contains("#ff6600"),
            "{v1}"
        );
        let before = resolve_theme_file_in(&dir, &read_theme_file(old).expect("v0")).expect("before");
        assert_eq!(resolve_theme_file_in(&dir, &read_theme_file(&v1).expect("v1")).expect("after"), before);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_single_theme_becomes_a_pair_and_exports_as_one_appearance() {
        let dir = folder("pair");
        let single = studio_copy_theme(text(&dir), "wormhole.dark".into(), "Duo".into()).expect("a copy");
        let pair = studio_add_appearance(text(&dir), single, false, "duo".into()).expect("a pair");
        let file = read_theme_file(&pair).expect("reads");
        let names: Vec<_> = file.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["light", "dark"]);
        let exported = studio_export(text(&dir), pair.clone(), Some("light".into())).expect("exports");
        assert!(!exported.contains("\"version\""), "the older format every release reads: {exported}");
        assert!(
            studio_add_appearance(text(&dir), pair, true, "duo".into()).is_err(),
            "a pair is not a single theme"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_base_that_fell_behind_its_source_is_updated_in_one_step() {
        let dir = folder("update");
        let made = studio_copy_theme(text(&dir), "nord".into(), "Mine".into()).expect("a copy");
        let now = studio_update_check(text(&dir), made.clone(), None).expect("checks");
        assert_eq!((now.state.as_str(), now.from.as_str()), ("current", "Nord"));
        let stale = made.replacen("\"ui\": {", "\"ui\": {\n \"primary\": \"#010203\",", 1);
        let stale = if stale == made {
            made.replacen("\"base\": {", "\"base\": {\n \"ui\": {\"primary\": \"#010203\"},", 1)
        } else {
            stale
        };
        let behind = studio_update_check(text(&dir), stale.clone(), None).expect("checks");
        assert_eq!(behind.state, "changed", "{stale}");
        assert!(behind.changes.iter().any(|c| c.key == "ui.primary" && c.old.as_deref() == Some("#010203")));
        let updated = studio_update_apply(text(&dir), stale, None).expect("applies");
        assert_eq!(studio_update_check(text(&dir), updated, None).expect("checks").state, "current");
        let own = studio_new_theme(true, "Own".into());
        assert_eq!(studio_update_check(text(&dir), own, None).expect("checks").state, "none");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_menu_lists_yours_once_and_the_built_ins_by_family() {
        let dir = folder("list");
        std::fs::write(dir.join("dusk.json"), r#"{"name": "Dusk"}"#).expect("write");
        std::fs::write(
            dir.join("pair.json"),
            r#"{"name": "Pair", "variants": [{"name": "Light"}, {"name": "Dark"}]}"#,
        )
        .expect("write");
        let list = studio_theme_list(text(&dir));
        assert_eq!(
            list.own.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["Dusk", "Pair"],
            "a pair once"
        );
        let wormhole = list.builtins.iter().find(|f| f.name == "Wormhole").expect("a family");
        assert!(wormhole.wormhole);
        assert_eq!(wormhole.choices.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Dark", "Light"]);
        let nord = list.builtins.iter().find(|f| f.name == "Nord").expect("Nord");
        assert!(!nord.wormhole && nord.choices.len() == 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_key_table_and_token_scope_are_core_s() {
        assert!(studio_key_table().iter().any(|r| (r.block.as_str(), r.old.as_str(), r.new.as_str())
            == ("terminal", "cursor_text", "cursor.text")));
        assert!(studio_token_in_both("primary".into()) && studio_token_in_both("metrics.radius".into()));
        assert!(!studio_token_in_both("surface.window".into()));
    }
}
