//! The themes the native window picks from: the catalog (built-in and the profile's themes folder), kept while the
//! folder is unchanged.

use std::rc::Rc;

use gpui_kit::{App, Global};
use serde_json::{Map, Value, json};
use wormhole_core::config::Settings;
use wormhole_core::theme::catalog::{self, ThemeEntry};
use wormhole_core::theme::pairs;
use wormhole_core::theme::tokens::{Appearance, Color};

#[derive(Default)]
struct Cache {
    fingerprint: Option<Vec<(std::path::PathBuf, u64, Option<std::time::SystemTime>)>>,
    entries: Rc<Vec<ThemeEntry>>,
}

impl Global for Cache {}

/// The catalog, made again only when the themes folder changed.
pub(super) fn entries(cx: &mut App) -> Rc<Vec<ThemeEntry>> {
    let dir = crate::active_profile::paths().themes_dir();
    let fingerprint = catalog::fingerprint_in(&dir);
    let cache = cx.default_global::<Cache>();
    if cache.fingerprint.as_ref() != Some(&fingerprint) {
        cache.entries = Rc::new(catalog::catalog_in(&dir));
        cache.fingerprint = Some(fingerprint);
    }
    cache.entries.clone()
}

/// A pair whose two halves are the picks of Dark and Light Mode.
fn picked_pair(entries: &[ThemeEntry], s: &Settings) -> Option<pairs::ThemePair> {
    pairs::pairs(entries).into_iter().find(|pair| {
        pair.is_whole()
            && pair.halves().all(|ix| match entries[ix].tokens.appearance {
                Appearance::Dark => entries[ix].is_selected_by(&s.theme),
                Appearance::Light => entries[ix].is_selected_by(&s.theme_light),
            })
    })
}

/// Whether the picks are the halves of one pair ("One theme for both").
pub(super) fn picks_are_a_pair(entries: &[ThemeEntry], s: &Settings) -> bool {
    picked_pair(entries, s).is_some()
}

/// The pair card chosen as `name`: it becomes both picks.
pub(super) fn choose_pair(entries: &[ThemeEntry], s: &mut Settings, name: &str) {
    let found = pairs::pairs(entries).into_iter().find(|pair| pair.is_whole() && pair.name == name);
    if let Some(pair) = found {
        s.theme = entries[pair.dark.unwrap_or_default()].value.clone();
        s.theme_light = entries[pair.light.unwrap_or_default()].value.clone();
    }
}

/// Dark and light picks separate again after "One theme for both": the light pick becomes the dark one's partner.
pub(super) fn pair_with_dark(entries: &[ThemeEntry], s: &mut Settings) {
    let dark = s.theme.clone();
    let partner = pairs::pairs(entries)
        .into_iter()
        .find(|pair| pair.dark.is_some_and(|ix| entries[ix].is_selected_by(&dark)))
        .and_then(|pair| pair.light);
    if let Some(light) = partner {
        s.theme_light = entries[light].value.clone();
    }
}

/// The current pick of a kind, as the id of the card.
pub(super) fn pick(entries: &[ThemeEntry], s: &Settings, appearance: Appearance) -> String {
    let setting = if appearance == Appearance::Dark { &s.theme } else { &s.theme_light };
    entries
        .iter()
        .find(|e| e.tokens.appearance == appearance && e.is_selected_by(setting))
        .map(|e| e.value.clone())
        .unwrap_or_default()
}

/// The theme in effect for one appearance: what the setting selects, else the first of that appearance.
pub(super) fn effective<'a>(
    entries: &'a [ThemeEntry],
    s: &Settings,
    appearance: Appearance,
) -> Option<&'a ThemeEntry> {
    let setting = if appearance == Appearance::Dark { &s.theme } else { &s.theme_light };
    entries
        .iter()
        .find(|e| e.tokens.appearance == appearance && e.is_selected_by(setting))
        .or_else(|| entries.iter().find(|e| e.tokens.appearance == appearance))
}

/// The pair card that is picked, by name; empty when none is.
pub(super) fn pair_name(entries: &[ThemeEntry], s: &Settings) -> String {
    picked_pair(entries, s).map(|pair| pair.name).unwrap_or_default()
}

pub(super) fn hex(c: Color) -> u32 {
    u32::from_str_radix(c.to_hex().trim_start_matches('#').get(..6).unwrap_or("000000"), 16).unwrap_or(0)
}

/// A theme as the window draws it: background, foreground, then ANSI blue, green, yellow, red, magenta.
pub(super) fn palette(entry: &ThemeEntry) -> Value {
    let t = &entry.tokens.terminal;
    json!([
        hex(t.background),
        hex(t.foreground),
        hex(t.ansi[4]),
        hex(t.ansi[2]),
        hex(t.ansi[3]),
        hex(t.ansi[1]),
        hex(t.ansi[5])
    ])
}

/// The most cards a kind shows before the person asks for more: the window has no "show all" for separate picks.
const SEPARATE_CARDS: usize = 16;

/// What the window needs to draw the pickers: palettes by id, the whole pairs, the lists per kind and the names.
pub(super) fn snapshot(entries: &[ThemeEntry], s: &Settings) -> Value {
    let mut palettes = Map::new();
    let mut names = Map::new();
    let mut add = |ix: usize| {
        let entry = &entries[ix];
        palettes.insert(entry.value.clone(), palette(entry));
        names.insert(entry.value.clone(), Value::String(entry.label.clone()));
        entry.value.clone()
    };
    let mut pair_list = Vec::new();
    for pair in pairs::pairs(entries).into_iter().filter(pairs::ThemePair::is_whole) {
        let (dark, light) = (add(pair.dark.unwrap_or_default()), add(pair.light.unwrap_or_default()));
        pair_list.push(json!([pair.name, dark, light]));
    }
    let mut kind = |appearance: Appearance| {
        let picked = pick(entries, s, appearance);
        let shown: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.tokens.appearance == appearance && !s.theme_hidden(&e.value))
            .map(|(ix, _)| ix)
            .collect();
        let mut chosen: Vec<usize> = shown.iter().copied().take(SEPARATE_CARDS).collect();
        let pick = shown.iter().copied().find(|ix| entries[*ix].value == picked);
        if let Some(at) = pick.filter(|at| !chosen.contains(at)) {
            chosen.pop();
            chosen.push(at);
        }
        chosen.into_iter().map(&mut add).collect::<Vec<_>>()
    };
    let darks = kind(Appearance::Dark);
    let lights = kind(Appearance::Light);
    json!({ "palettes": palettes, "names": names, "pairs": pair_list, "darks": darks, "lights": lights })
}

/// Import the theme file the person picked in the window; a failure is said under the theme picker.
pub(super) fn import(path: &std::path::Path, cx: &mut App) {
    let failure = crate::settings::SettingsStore::import_theme(cx, path).err();
    if failure.is_some() {
        tracing::warn!("importing a theme: {failure:?}");
    }
    super::set_import_failure(failure, cx);
}
