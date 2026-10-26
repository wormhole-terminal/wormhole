//! Ghostty: `config` and `config.ghostty` in `~/.config/ghostty/` and in `~/Library/Application
//! Support/com.mitchellh.ghostty/`, all four read and merged in that order, `config-file` includes
//! read at the end of the file that names them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::entries::{Entries, Entry};
use super::env::Env;
use super::locations;
use super::proposal::{Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::{Reader, Scope};
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::theme;

/// What Ghostty draws when `font-size` is not set; the base of an added cell height.
const DEFAULT_FONT_SIZE: f32 = 13.0;

/// The colour keys a Ghostty theme file holds, which the main config can override.
const COLOR_KEYS: [&str; 7] = [
    "palette",
    "background",
    "foreground",
    "cursor-color",
    "cursor-text",
    "selection-background",
    "selection-foreground",
];

/// Read Ghostty's files and say what they set.
pub(crate) fn import(env: &Env, files: &mut Reader, located: &[PathBuf]) -> Proposal {
    let mut entries = Entries::default();
    for path in located {
        load(env, files, path, Scope::Anywhere, &mut entries);
    }
    let mut themes = BTreeMap::new();
    if let Some(entry) = entries.last("theme") {
        for name in ThemeRef::parse(&entry.value).names() {
            if let Some(text) = read_theme(env, files, &name) {
                themes.insert(name, text);
            }
        }
    }
    propose(env, &entries, &themes)
}

/// One file's entries, then the files it includes (which must be in the home folder).
fn load(env: &Env, files: &mut Reader, path: &Path, scope: Scope, entries: &mut Entries) {
    files.nested_in(path, scope, |files, text| {
        let mut includes = Vec::new();
        parse_into(&text, entries, &mut includes);
        let dir = path.parent().unwrap_or(Path::new("/"));
        for include in includes {
            load(env, files, &env.include_path(&include, dir), Scope::Home, entries);
        }
    });
}

/// A file's `key = value` lines. `#` starts a comment only at the start of a line, so a colour
/// like `#112233` is a value; `config-file` lines are returned apart, `?` (optional) removed.
fn parse_into(text: &str, entries: &mut Entries, includes: &mut Vec<String>) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if key == "config-file" {
            let path = value.strip_prefix('?').unwrap_or(value);
            if !unquote(path).is_empty() {
                includes.push(unquote(path).to_string());
            }
        } else {
            entries.push(key, unquote(value), line);
        }
    }
}

fn unquote(value: &str) -> &str {
    value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).unwrap_or(value)
}

/// The theme a `theme` line names: one for both appearances, or a light and a dark.
struct ThemeRef {
    dark: Option<String>,
    light: Option<String>,
}

impl ThemeRef {
    /// `NAME`, or `light:NAME,dark:NAME` (either order; one is enough).
    fn parse(value: &str) -> Self {
        let value = value.trim();
        // Any of `,` `:` `=` makes it pairs (`light:A,dark:B`, `light=A`), as Ghostty reads it.
        if !value.contains([',', ':', '=']) || value.starts_with('/') {
            return Self { dark: (!value.is_empty()).then(|| value.to_string()), light: None };
        }
        let (mut dark, mut light) = (None, None);
        for part in value.split(',') {
            match part.trim().split_once([':', '=']) {
                Some(("dark", name)) => dark = Some(name.trim().to_string()),
                Some(("light", name)) => light = Some(name.trim().to_string()),
                _ => {}
            }
        }
        Self { dark, light }
    }

    fn names(&self) -> Vec<String> {
        self.dark.iter().chain(&self.light).cloned().collect()
    }
}

/// A theme's file: by path, else the person's themes folder, else the app's own.
fn read_theme(env: &Env, files: &mut Reader, name: &str) -> Option<String> {
    if name.starts_with('/') || name.starts_with("~/") {
        return files.text_in_home(&env.expand_tilde(name));
    }
    if name.split('/').any(|part| part == ".." || part.is_empty()) {
        return None;
    }
    locations::ghostty_theme_dirs(env).iter().find_map(|dir| files.text(&dir.join(name)))
}

/// What the merged entries set. `themes` holds the text of each theme file the config names.
pub(crate) fn propose(env: &Env, entries: &Entries, themes: &BTreeMap<String, String>) -> Proposal {
    let mut proposal = Proposal::default();
    propose_font(&mut proposal, entries);
    propose_window(&mut proposal, entries);
    propose_keys(&mut proposal, env, entries);
    propose_palette(&mut proposal, entries, themes);
    skip_unsupported(&mut proposal, entries);
    proposal.sorted()
}

fn propose_font(proposal: &mut Proposal, entries: &Entries) {
    // `font-family` adds up; an empty value starts the list over. The first is the one drawn.
    let mut families: Vec<&Entry> = Vec::new();
    for entry in entries.all("font-family") {
        if entry.value.is_empty() {
            families.clear();
        } else {
            families.push(entry);
        }
    }
    if let Some(entry) = families.first() {
        proposal.propose(
            Setting::FontFamily,
            &entry.raw,
            SettingValue::Font(super::proposal::FontName::family_name(&entry.value)),
        );
    }

    let mut size = DEFAULT_FONT_SIZE;
    if let Some(entry) = entries.last("font-size").filter(|e| !e.value.is_empty()) {
        match units::number(&entry.value) {
            Some(points) => {
                size = units::font_size(points);
                proposal.propose(Setting::FontSize, &entry.raw, SettingValue::Number(size));
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }

    // `adjust-cell-height` adds to the cell's height: a percentage of it, or points.
    if let Some(entry) = entries.last("adjust-cell-height").filter(|e| !e.value.is_empty()) {
        let multiplier = match entry.value.strip_suffix('%') {
            Some(percent) => units::number(percent).map(|p| 1.0 + p / 100.0),
            None => units::number(&entry.value).map(|delta| units::added_height_multiplier(delta, size)),
        };
        match multiplier {
            Some(value) => {
                proposal.propose(
                    Setting::LineHeight,
                    &entry.raw,
                    SettingValue::Number(units::line_height(value)),
                );
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }

    // Ligatures are on unless `calt` is turned off, which is how Ghostty's docs say to do it.
    let mut calt: Option<(&Entry, bool)> = None;
    for entry in entries.all("font-feature") {
        for feature in entry.value.split(',').map(|f| unquote(f.trim())) {
            let (tag, on) = match feature.split_once('=') {
                Some((tag, value)) => (tag.trim(), units::number(value).is_some_and(|v| v > 0.0)),
                None => match feature.strip_prefix('-') {
                    Some(tag) => (tag, false),
                    None => (feature.strip_prefix('+').unwrap_or(feature), true),
                },
            };
            if tag == "calt" {
                calt = Some((entry, on));
            }
        }
    }
    if let Some((entry, on)) = calt {
        proposal.propose(Setting::Ligatures, &entry.raw, SettingValue::Flag(on));
    }
}

fn propose_window(proposal: &mut Proposal, entries: &Entries) {
    if let Some(entry) = entries.last("background-opacity").filter(|e| !e.value.is_empty()) {
        match units::number(&entry.value) {
            Some(value) => {
                proposal.propose(Setting::Opacity, &entry.raw, SettingValue::Number(units::opacity(value)))
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    let blur = entries.last("background-blur").or_else(|| entries.last("background-blur-radius"));
    if let Some(entry) = blur.filter(|e| !e.value.is_empty()) {
        // A radius, or a switch, or the macOS 26 glass styles, which blur and more.
        let on = match entry.value.as_str() {
            "true" | "macos-glass-regular" | "macos-glass-clear" => Some(true),
            "false" => Some(false),
            other => units::number(other).map(|radius| radius > 0.0),
        };
        match on {
            Some(on) => proposal.propose(Setting::Blur, &entry.raw, SettingValue::Flag(on)),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    if let Some(entry) = entries.last("scrollback-limit-lines").filter(|e| !e.value.is_empty()) {
        match entry.value.as_str() {
            "unlimited" => proposal.propose(
                Setting::ScrollbackLines,
                &entry.raw,
                SettingValue::Count(units::unlimited_scrollback()),
            ),
            text => match text.parse::<usize>() {
                Ok(lines) => proposal.propose(
                    Setting::ScrollbackLines,
                    &entry.raw,
                    SettingValue::Count(units::scrollback(lines)),
                ),
                Err(_) => proposal.skip(&entry.raw, Reason::UnsupportedValue),
            },
        }
    } else if let Some(entry) = entries
        .last("scrollback-limit-bytes")
        .or_else(|| entries.last("scrollback-limit"))
        .filter(|e| !e.value.is_empty())
    {
        // Ghostty limits memory, not lines: the line count is an estimate (see `lines_from_bytes`).
        match entry.value.as_str() {
            "unlimited" => proposal.propose(
                Setting::ScrollbackLines,
                &entry.raw,
                SettingValue::Count(units::unlimited_scrollback()),
            ),
            text => match text.parse::<u64>() {
                Ok(bytes) => proposal.propose(
                    Setting::ScrollbackLines,
                    &entry.raw,
                    SettingValue::Count(units::scrollback(units::lines_from_bytes(bytes))),
                ),
                Err(_) => proposal.skip(&entry.raw, Reason::UnsupportedValue),
            },
        }
    }
}

fn propose_keys(proposal: &mut Proposal, env: &Env, entries: &Entries) {
    if let Some(entry) = entries.last("macos-option-as-alt").filter(|e| !e.value.is_empty()) {
        let meta = match entry.value.as_str() {
            "true" => Some(OptionAsMeta::Both),
            "false" => Some(OptionAsMeta::Off),
            "left" => Some(OptionAsMeta::Left),
            "right" => Some(OptionAsMeta::Right),
            _ => None,
        };
        match meta {
            Some(meta) => proposal.propose(Setting::OptionAsMeta, &entry.raw, SettingValue::Meta(meta)),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }

    // Audible is `system` (the alert sound) or `audio` (a file); the one visual cue that Wormhole
    // has, a glow along the edge, is `border`.
    if let Some(entry) = entries.last("bell-features").filter(|e| !e.value.is_empty()) {
        let (mut audible, mut border): (Option<bool>, Option<bool>) = (None, None);
        for feature in entry.value.split(',').map(str::trim) {
            let (name, on) = feature.strip_prefix("no-").map_or((feature, true), |name| (name, false));
            match name {
                "system" | "audio" => audible = Some(audible.unwrap_or(false) || on),
                "border" => border = Some(on),
                _ => {}
            }
        }
        if let Some(on) = audible {
            proposal.propose(Setting::BellSound, &entry.raw, SettingValue::Flag(on));
        }
        if let Some(on) = border {
            proposal.propose(Setting::BellVisual, &entry.raw, SettingValue::Flag(on));
        }
    }

    if let Some(entry) = entries.last("command").filter(|e| !e.value.is_empty()) {
        let line = entry
            .value
            .strip_prefix("direct:")
            .or_else(|| entry.value.strip_prefix("shell:"))
            .unwrap_or(&entry.value);
        match shell::plain_words(line) {
            Some(words) => shell::propose(proposal, env, Source::Ghostty, &entry.raw, &words),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
}

/// The colours: a theme file (or a light and a dark one) with the config's own colour lines over it.
fn propose_palette(proposal: &mut Proposal, entries: &Entries, themes: &BTreeMap<String, String>) {
    let mut overlay: Vec<&Entry> = Vec::new();
    for entry in entries.iter().filter(|e| COLOR_KEYS.contains(&e.key.as_str())) {
        // A colour by name, or `cell-foreground`, is not a value Wormhole's themes hold.
        if theme::parse_ghostty("line", &entry.raw).is_ok() {
            overlay.push(entry);
        } else {
            proposal.skip(&entry.raw, Reason::UnsupportedValue);
        }
    }
    let overlay_text: String = overlay.iter().map(|e| format!("\n{} = {}", e.key, e.value)).collect();

    let reference = entries.last("theme").filter(|e| !e.value.is_empty());
    let Some(reference) = reference else {
        if let Some(first) = overlay.first()
            && let Ok(builder) = theme::ghostty_builder(Source::Ghostty.brand(), &overlay_text)
        {
            proposal.propose_palette(Setting::Theme, &first.raw, Source::Ghostty.brand(), builder);
        }
        return;
    };
    let wanted = ThemeRef::parse(&reference.value);
    for (setting, name) in [(Setting::Theme, wanted.dark), (Setting::ThemeLight, wanted.light)] {
        let Some(name) = name else { continue };
        let Some(text) = themes.get(&name) else {
            proposal.skip(&reference.raw, Reason::BuiltInTheme(name));
            continue;
        };
        let label = format!("{} {name}", Source::Ghostty.brand());
        match theme::ghostty_builder(&label, &format!("{text}{overlay_text}")) {
            Ok(builder) => proposal.propose_palette(setting, &reference.raw, &label, builder),
            Err(_) => proposal.skip(&reference.raw, Reason::Unreadable),
        }
    }
}

/// What Ghostty sets that Wormhole has no setting for, or will not take.
fn skip_unsupported(proposal: &mut Proposal, entries: &Entries) {
    for entry in entries.iter().filter(|e| !e.value.is_empty()) {
        let kind = match entry.key.as_str() {
            "cursor-style" => SkippedKind::CursorShape,
            "cursor-style-blink" => SkippedKind::CursorBlink,
            "window-padding-x" | "window-padding-y" | "window-padding-balance" | "window-padding-color" => {
                SkippedKind::Padding
            }
            "copy-on-select" => SkippedKind::CopyOnSelect,
            "confirm-close-surface" => SkippedKind::ConfirmClose,
            "working-directory" => SkippedKind::WorkingDirectory,
            "keybind" => SkippedKind::Keybinding,
            "env" => SkippedKind::Environment,
            "font-family-bold"
            | "font-family-italic"
            | "font-family-bold-italic"
            | "font-style"
            | "font-style-bold"
            | "font-style-italic"
            | "font-style-bold-italic"
            | "font-thicken"
            | "font-variation" => SkippedKind::FontVariant,
            "adjust-cell-width" | "adjust-font-baseline" => SkippedKind::CellWidth,
            "initial-command" => {
                proposal.skip(&entry.raw, Reason::Executes);
                continue;
            }
            _ => continue,
        };
        proposal.skip_kind(&entry.raw, kind);
    }
}
