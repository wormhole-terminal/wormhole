//! A profile's settings over core's `Config`: what the file says, a trusted project's overrides on top,
//! and a revision that moves whenever what a window shows would read differently. Plain Rust with no
//! UniFFI in it, so the tests drive it over a temp folder; `handle` wraps it in the object.
//!
//! Every rule is core's: `Config::update` merges, sanitises and writes in place (comments kept, never
//! over a file that does not read), `settings_index::set` and `reset` refuse what a key does not take,
//! `with_overrides_reported` layers a project and `Values` says what differs from the default.

use serde_json::{Map, Value};
use wormhole_core::config::settings_index::{self, Values, entries};
use wormhole_core::config::{Config, PROJECT_SETTINGS, Settings};
use wormhole_core::storage::paths::ProfilePaths;

use super::values::{schema_keys, schema_values};

pub(crate) struct State {
    config: Config,
    /// The profile's folder: what `edits` counts Settings' changes by.
    dir: std::path::PathBuf,
    /// The active terminal's project settings, already limited to what a project may change.
    overrides: Option<Map<String, Value>>,
    /// What shows: the file's settings with `overrides` on top.
    effective: Settings,
    /// The keys `overrides` really set (known, and with a value that reads), sorted.
    project_keys: Vec<String>,
    revision: u64,
}

/// What the file and the last save say, for the page's banner.
pub(crate) struct Health {
    pub error: Option<String>,
    pub unreadable: bool,
}

impl State {
    /// Loads the profile's `config.json`, writing it with the defaults when it is not there yet.
    pub(crate) fn load(paths: &ProfilePaths) -> Self {
        let config = Config::load_for(paths);
        let effective = config.settings.clone();
        Self {
            config,
            dir: paths.dir().to_path_buf(),
            overrides: None,
            effective,
            project_keys: Vec::new(),
            revision: 0,
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Every schema key with its effective value (the project's where it overrides).
    pub(crate) fn values(&self) -> Map<String, Value> {
        schema_values(&self.effective)
    }

    /// The effective settings, for what needs the typed value (search's visibility, the shell list).
    pub(crate) fn effective(&self) -> &Settings {
        &self.effective
    }

    pub(crate) fn health(&self) -> Health {
        Health { error: self.config.error.clone(), unreadable: self.config.is_unreadable() }
    }

    /// Sets each `(key, value)` in order and saves once. All or nothing: the first value core refuses
    /// leaves the settings as they were and returns its sentence. `Ok(false)` when nothing differed.
    pub(crate) fn set_many(&mut self, writes: Vec<(String, Value)>) -> Result<bool, String> {
        let mut next = self.config.settings.clone();
        for (key, value) in writes {
            next = settings_index::set(&next, &key, value)?;
        }
        Ok(self.replace(next))
    }

    /// Lets `change` edit a copy of the settings and saves it (an import brings several keys at once).
    /// `false` when nothing differed.
    pub(crate) fn update(&mut self, change: impl FnOnce(&mut Settings)) -> bool {
        let mut next = self.config.settings.clone();
        change(&mut next);
        self.replace(next)
    }

    /// Copies a theme file into the profile's themes folder and selects it (core's `Config::import_theme`).
    pub(crate) fn import_theme(&mut self, source: &std::path::Path) -> Result<(), String> {
        let before = self.config.settings.clone();
        self.config.import_theme(source)?;
        if self.config.settings != before {
            super::edits::note(&self.dir);
            self.refresh();
            self.revision += 1;
        }
        Ok(())
    }

    /// Puts `keys` back at their defaults and saves. `Ok(false)` when they already were.
    pub(crate) fn reset(&mut self, keys: &[String]) -> bool {
        let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
        let next = settings_index::reset(&self.config.settings, &keys);
        self.replace(next)
    }

    fn replace(&mut self, next: Settings) -> bool {
        // `update` sanitises, so compare what would stick, not what was asked for.
        if next == self.config.settings {
            return false;
        }
        self.config.update(|settings| *settings = next);
        super::edits::note(&self.dir);
        self.refresh();
        self.revision += 1;
        true
    }

    /// Reads the file again when it changed on disk (a hand edit, another window). One `stat` when it
    /// did not. Returns whether anything was read: the settings, or the error the page shows, may
    /// have moved.
    pub(crate) fn poll(&mut self) -> bool {
        if !self.config.poll() {
            return false;
        }
        self.refresh();
        self.revision += 1;
        true
    }

    /// Layers the project's settings (or none) over the person's. Keys a project may not change are
    /// dropped, as the app does. Returns whether anything shown changed.
    pub(crate) fn set_project_overrides(&mut self, overrides: Option<Map<String, Value>>) -> bool {
        let scoped = overrides
            .map(|map| map.into_iter().filter(|(key, _)| PROJECT_SETTINGS.contains(&key.as_str())).collect());
        if scoped == self.overrides {
            return false;
        }
        self.overrides = scoped;
        self.refresh();
        self.revision += 1;
        true
    }

    fn refresh(&mut self) {
        let (effective, keys) = match &self.overrides {
            Some(overrides) => self.config.settings.with_overrides_reported(overrides),
            None => (self.config.settings.clone(), Vec::new()),
        };
        self.effective = effective;
        self.project_keys = keys;
    }

    /// Every schema key that differs from its default in the person's own settings (not the project's).
    pub(crate) fn changed_keys(&self) -> Vec<String> {
        let values = Values::of(&self.config.settings);
        schema_keys().iter().filter(|key| values.changed(key)).cloned().collect()
    }

    /// The rows the Changed page lists, as `(section, entry id)` in page order: the rows that differ
    /// from their default and show now.
    pub(crate) fn changed_rows(&self) -> Vec<(String, &'static str)> {
        Values::of(&self.config.settings)
            .changed_rows(&self.effective)
            .into_iter()
            .map(|row| (crate::settings_dump::section_id(row.section), row.entry.id))
            .collect()
    }

    /// `keys` and their current values as one line of JSON to paste into config.json ("Copy as JSON").
    pub(crate) fn copy_json(&self, keys: &[String]) -> String {
        let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
        Values::of(&self.config.settings).json(&keys)
    }

    /// The same for every key the Changed page lists.
    pub(crate) fn changed_json(&self) -> String {
        let values = Values::of(&self.config.settings);
        let rows = values.changed_rows(&self.effective);
        values.json(&values.changed_keys_of(&rows))
    }

    /// The ids of the rows a project's settings set (the "set by project" mark).
    pub(crate) fn overridden_entries(&self) -> Vec<&'static str> {
        entries()
            .filter(|(_, _, entry)| settings_index::set_by_project(entry, &self.project_keys))
            .map(|(_, _, entry)| entry.id)
            .collect()
    }
}
