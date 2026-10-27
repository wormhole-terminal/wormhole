//! Zen mode in the store: what it layers over the settings, that the person's own values stay under it
//! untouched, and that nothing of it reaches `config.json`.

use std::path::PathBuf;

use wormhole_core::config::presets::{Preset, Shown};
use wormhole_core::config::{Config, Settings};
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::view::layout::title_parts::{TitlePart, TitleRow};

use super::{ParkedSettings, SettingsStore, SystemLook};

/// A data folder of its own under the system temp dir, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let dir = std::env::temp_dir().join(format!("wormhole-zen-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("makes the scratch folder");
        Self(dir)
    }

    fn paths(&self) -> ProfilePaths {
        DataRoot::new(&self.0).profile(&ProfileId::default_profile())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A store over a real `config.json` in `scratch`, where the person has everything Zen mode hides
/// switched on and a title row of their own.
fn store_with_everything(scratch: &Scratch) -> SettingsStore {
    let mut store = SettingsStore::load_for(&scratch.paths());
    store.config.update(|settings| {
        settings.status_bar = true;
        settings.minimap = true;
        settings.column_dividers = true;
        settings.title_row = Some(own_row());
    });
    store.derive();
    store
}

fn own_row() -> TitleRow {
    TitleRow::new([TitlePart::Folder, TitlePart::Terminal, TitlePart::Space, TitlePart::Ports])
}

fn zen(store: &mut SettingsStore, on: bool) {
    store.take_zen(on);
}

fn file(scratch: &Scratch) -> Vec<u8> {
    std::fs::read(scratch.paths().config_file()).expect("config.json exists")
}

#[test]
fn zen_mode_shows_the_zen_preset_and_nothing_else_changes() {
    let scratch = Scratch::new("preset");
    let mut store = store_with_everything(&scratch);
    let before = store.settings.clone();

    zen(&mut store, true);

    let preset = Shown::of(Preset::Zen);
    assert!(store.zen());
    assert_eq!(store.settings.title_row().into_owned(), preset.title.row());
    assert!(!store.settings.status_bar && !store.settings.minimap && !store.settings.column_dividers);
    // The rest of the settings are as they were: the same but for the four Zen mode sets.
    let mut expected = before;
    expected.title_row = Some(preset.title.row());
    expected.status_bar = false;
    expected.minimap = false;
    expected.column_dividers = false;
    assert_eq!(store.settings, expected);
    // The main window draws no line between its columns.
    assert_eq!(store.ui.divider_width, gpui_kit::px(0.0));
}

#[test]
fn ending_zen_mode_restores_exactly_what_was_there() {
    let scratch = Scratch::new("restore");
    let mut store = store_with_everything(&scratch);
    let (settings, ui, revisions) = (store.settings.clone(), store.ui, store.revisions);

    zen(&mut store, true);
    assert!(store.revisions.settings > revisions.settings, "a view that draws from the settings hears it");
    zen(&mut store, false);

    assert_eq!(store.settings, settings);
    assert_eq!(store.ui, ui);
    assert!(store.settings_without_zen() == &store.settings);
    assert_eq!(store.settings.title_row().into_owned(), own_row(), "the person's own title row survived");
}

#[test]
fn the_settings_window_reads_the_settings_without_the_layer() {
    let scratch = Scratch::new("without");
    let mut store = store_with_everything(&scratch);
    zen(&mut store, true);

    let own = store.settings_without_zen();
    assert!(own.status_bar && own.minimap && own.column_dividers);
    assert_eq!(own.title_row().into_owned(), own_row());
    // What is in force is not that.
    assert!(!store.settings.status_bar);

    // A project's overrides are part of what is under the layer; Zen mode comes after them.
    let overrides: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({ "diff_wrap": true, "status_bar": false })).expect("a map");
    store.project_overrides = Some(overrides);
    store.derive();
    assert!(store.settings_without_zen().diff_wrap && !store.settings_without_zen().status_bar);
    assert!(store.settings.diff_wrap && !store.settings.status_bar);
    zen(&mut store, false);
    assert!(store.settings_without_zen() == &store.settings, "off: the two are one");
}

#[test]
fn turning_it_on_and_off_writes_nothing() {
    let scratch = Scratch::new("quiet");
    let mut store = store_with_everything(&scratch);
    let (bytes, modified) = (file(&scratch), std::fs::metadata(scratch.paths().config_file()).ok());

    zen(&mut store, true);
    zen(&mut store, false);
    zen(&mut store, true);

    assert_eq!(file(&scratch), bytes);
    assert_eq!(
        std::fs::metadata(scratch.paths().config_file()).ok().map(|m| m.modified().ok()),
        modified.map(|m| m.modified().ok())
    );
}

#[test]
fn a_setting_changed_while_zen_mode_is_on_saves_the_persons_values() {
    let scratch = Scratch::new("saves");
    let mut store = store_with_everything(&scratch);
    zen(&mut store, true);

    // What `SettingsStore::update` does with a change, minus the GPUI global.
    store.config.update(|settings| settings.diff_wrap = !settings.diff_wrap);
    store.derive();

    let saved = Config::load_for(&scratch.paths()).settings;
    assert!(saved.status_bar && saved.minimap && saved.column_dividers, "the values Zen mode hides");
    assert_eq!(saved.title_row().into_owned(), own_row(), "no Zen title row in the file");
    assert_eq!(saved.diff_wrap, store.config.settings.diff_wrap, "the change itself was saved");
    assert_eq!(&saved, store.user_settings());
    // And the window still shows less.
    assert!(!store.settings.status_bar);
}

#[test]
fn a_change_to_what_zen_mode_hides_is_saved_and_shows_when_it_ends() {
    let scratch = Scratch::new("hidden");
    let mut store = store_with_everything(&scratch);
    zen(&mut store, true);

    store.config.update(|settings| settings.minimap = false);
    store.derive();
    assert!(!store.settings_without_zen().minimap);
    assert!(!Config::load_for(&scratch.paths()).settings.minimap);

    zen(&mut store, false);
    assert!(!store.settings.minimap && store.settings.status_bar);
}

#[test]
fn another_profiles_store_follows_the_app() {
    let mut shown = SettingsStore::new(Config::placeholder());
    let mut parked = ParkedSettings::new(ProfileId::default_profile());
    let other = ProfileId::parse("other").expect("a profile id");
    let mut elsewhere = SettingsStore::new(Config::placeholder());
    elsewhere.config.update(|settings| settings.status_bar = true);
    elsewhere.derive();
    parked.park(other.clone(), elsewhere);

    // Turned on while it is parked.
    parked.set_zen(true);
    assert!(parked.get(&other).is_some_and(|store| store.zen() && !store.settings.status_bar));

    // A store opened after Zen mode went on takes it at the next tick.
    let mut late = SettingsStore::new(Config::placeholder());
    assert!(!late.zen());
    shown.take_zen(true);
    assert!(late.follow(SystemLook::default(), shown.zen()));
    assert!(late.zen());
    assert!(!late.follow(SystemLook::default(), shown.zen()), "nothing moved the second time");

    // Off, everywhere.
    parked.set_zen(false);
    assert!(parked.get(&other).is_some_and(|store| !store.zen() && store.settings.status_bar));
}

#[test]
fn the_layer_is_the_four_settings_of_the_preset() {
    let mut settings = Settings { status_bar: true, minimap: true, ..Settings::default() };
    super::zen::layer(&mut settings);
    let preset = Shown::of(Preset::Zen);
    assert_eq!(
        (settings.title_row().into_owned(), settings.status_bar, settings.minimap, settings.column_dividers),
        (preset.title.row(), preset.status_bar, preset.minimap, preset.column_dividers)
    );
}
