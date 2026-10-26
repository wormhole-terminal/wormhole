//! Kitty: `kitty.conf` in the first of its config folders that has one, `include` read where it
//! stands (a later line overrides an earlier), and the light and dark theme files its themes
//! kitten writes beside it. `geninclude` runs a program and is never followed.

use std::path::{Path, PathBuf};

use super::entries::{Entries, Entry};
use super::env::Env;
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::{MAX_FILES, Reader, Scope};
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::theme::{Builder, parse_hex};

/// What Kitty draws when `font_size` is not set.
const DEFAULT_FONT_SIZE: f32 = 11.0;

/// Read `kitty.conf` and its includes and say what they set.
pub(crate) fn import(env: &Env, files: &mut Reader, located: &[PathBuf]) -> Proposal {
    let mut entries = Entries::default();
    let Some(conf) = located.first() else { return Proposal::default() };
    load(env, files, conf, Scope::Anywhere, &mut entries);
    // The themes kitten's light/dark switch writes win over the colours in `kitty.conf`.
    let dir = conf.parent().unwrap_or(Path::new("/")).to_path_buf();
    let mut auto = |name: &str| {
        let text = files.text(&dir.join(name))?;
        let mut theme = Entries::default();
        parse_into(&text, &mut theme);
        Some(theme)
    };
    let dark = auto("dark-theme.auto.conf");
    let light = auto("light-theme.auto.conf");
    propose(env, &entries, dark.as_ref(), light.as_ref())
}

/// One file's entries; an include is read at its line, so what it sets lands between the entries
/// on either side of it.
fn load(env: &Env, files: &mut Reader, path: &Path, scope: Scope, entries: &mut Entries) {
    files.nested_in(path, scope, |files, text| {
        let dir = path.parent().unwrap_or(Path::new("/")).to_path_buf();
        let mut own = Entries::default();
        let includes = parse_into(&text, &mut own);
        let mut includes = includes.into_iter().peekable();
        for (index, entry) in own.iter().enumerate() {
            while let Some((_, include)) = includes.next_if(|(position, _)| *position == index) {
                follow(env, files, &dir, include, entries);
            }
            entries.push(&entry.key, &entry.value, &entry.raw);
        }
        for (_, include) in includes {
            follow(env, files, &dir, include, entries);
        }
    });
}

/// An include line.
enum Include {
    Path(String),
    Glob(String),
    /// `geninclude`: a program writes the file.
    Generated(String),
    /// `envinclude`: files named by environment variables, which an import cannot know.
    Environment(String),
}

fn follow(env: &Env, files: &mut Reader, dir: &Path, include: Include, entries: &mut Entries) {
    match include {
        // What the file names is read only inside the home folder.
        Include::Path(text) => {
            if let Some(path) = expand(env, &text, dir) {
                load(env, files, &path, Scope::Home, entries);
            }
        }
        Include::Glob(text) => {
            let Some(pattern) = expand(env, &text, dir) else { return };
            if !files.names_home(&pattern) {
                files.note(&pattern, Reason::OutsideHome);
                return;
            }
            for path in glob(&pattern) {
                load(env, files, &path, Scope::Home, entries);
            }
        }
        Include::Generated(text) => files.note(Path::new(&format!("geninclude {text}")), Reason::Executes),
        Include::Environment(text) => {
            files.note(Path::new(&format!("envinclude {text}")), Reason::UnsupportedValue);
        }
    }
}

/// Lines as Kitty reads them: `key value`, `#` comments, a line starting with `\` continuing the one
/// before. The includes come back with the number of entries that precede each.
fn parse_into(text: &str, entries: &mut Entries) -> Vec<(usize, Include)> {
    let mut logical: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        match (trimmed.strip_prefix('\\'), logical.last_mut()) {
            (Some(rest), Some(last)) => last.push_str(rest),
            _ => logical.push(trimmed.to_string()),
        }
    }
    let mut includes = Vec::new();
    let mut count = 0;
    for line in logical.iter().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let (key, value) = match line.split_once(char::is_whitespace) {
            Some((key, value)) => (key, value.trim().to_string()),
            None => (line.as_str(), String::new()),
        };
        match key {
            "include" => includes.push((count, Include::Path(value))),
            "globinclude" => includes.push((count, Include::Glob(value))),
            "geninclude" => includes.push((count, Include::Generated(value))),
            "envinclude" => includes.push((count, Include::Environment(value))),
            _ => {
                entries.push(key, &value, line);
                count += 1;
            }
        }
    }
    includes
}

/// An include's path: `~` and `$HOME` are the home folder, a relative path is relative to the file
/// that names it. Any other variable is the environment's, which an import does not read.
fn expand(env: &Env, text: &str, dir: &Path) -> Option<PathBuf> {
    let home = env.home.to_string_lossy();
    let text = text.replace("${HOME}", &home).replace("$HOME", &home);
    (!text.contains('$')).then(|| env.include_path(&text, dir))
}

/// The files a pattern with `*` and `?` in its last part names, by name.
fn glob(pattern: &Path) -> Vec<PathBuf> {
    let (Some(dir), Some(name)) = (pattern.parent(), pattern.file_name().and_then(|n| n.to_str())) else {
        return Vec::new();
    };
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<PathBuf> = read
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().is_some_and(|file| wildcard(name, file)))
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found.truncate(MAX_FILES);
    found
}

/// The longest glob pattern matched: real ones are a few characters.
const MAX_PATTERN: usize = 256;

/// Whether `name` matches `pattern`, where `*` is any run and `?` any one character. Iterative, with
/// one place to go back to (the last `*`), so it takes at most pattern times name steps however many
/// stars a hostile pattern has.
fn wildcard(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    if p.len() > MAX_PATTERN {
        return false;
    }
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0, 0);
    // Where the last `*` was in the pattern, and how much of the name it has taken so far.
    let mut star: Option<(usize, usize)> = None;
    while ni < n.len() {
        match p.get(pi) {
            Some('*') => {
                star = Some((pi, ni));
                pi += 1;
            }
            Some(c) if *c == '?' || *c == n[ni] => {
                pi += 1;
                ni += 1;
            }
            _ => match star {
                Some((star_pi, star_ni)) => {
                    // Let the star take one more character of the name and try the rest again.
                    star = Some((star_pi, star_ni + 1));
                    pi = star_pi + 1;
                    ni = star_ni + 1;
                }
                None => return false,
            },
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// What the merged entries set; `dark` and `light` are the theme files of the system's appearance
/// switch, when Kitty has them.
pub(crate) fn propose(
    env: &Env,
    entries: &Entries,
    dark: Option<&Entries>,
    light: Option<&Entries>,
) -> Proposal {
    let mut proposal = Proposal::default();
    propose_font(&mut proposal, entries);
    propose_window(&mut proposal, entries);
    propose_behaviour(&mut proposal, env, entries);
    propose_themes(&mut proposal, entries, dark, light);
    skip_unsupported(&mut proposal, entries);
    proposal.sorted()
}

fn propose_font(proposal: &mut Proposal, entries: &Entries) {
    if let Some(entry) = entries.last("font_family").filter(|e| !e.value.is_empty()) {
        // Newer Kitty writes `family="Fira Code" style=…`; the older form is the name itself.
        let family = shell::words(&entry.value)
            .iter()
            .find_map(|word| word.strip_prefix("family=").map(str::to_string))
            .unwrap_or_else(|| entry.value.clone());
        if !matches!(family.as_str(), "monospace" | "auto" | "") {
            proposal.propose(
                Setting::FontFamily,
                &entry.raw,
                SettingValue::Font(FontName::family_name(family)),
            );
        }
    }
    let mut size = DEFAULT_FONT_SIZE;
    if let Some(entry) = entries.last("font_size") {
        match units::number(&entry.value) {
            Some(points) => {
                size = units::font_size(points);
                proposal.propose(Setting::FontSize, &entry.raw, SettingValue::Number(size));
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    // `modify_font cell_height 110%` is a share of the natural height; `2px` (device pixels) and
    // `2` (points, the default unit) are additions.
    let height = entries.all("modify_font").filter(|e| e.value.starts_with("cell_height")).last();
    if let Some(entry) = height {
        let amount = entry.value.trim_start_matches("cell_height").trim();
        let multiplier = match amount.strip_suffix('%') {
            Some(percent) => units::number(percent).map(|p| p / 100.0),
            None => match amount.strip_suffix("px") {
                Some(pixels) => units::number(pixels).map(|n| n / units::PIXELS_PER_POINT),
                None => units::number(amount.strip_suffix("pt").unwrap_or(amount)),
            }
            .map(|delta| units::added_height_multiplier(delta, size)),
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
    if let Some(entry) = entries.last("disable_ligatures") {
        // Only `always` turns them off everywhere; `cursor` only under the cursor.
        match entry.value.as_str() {
            "always" => proposal.propose(Setting::Ligatures, &entry.raw, SettingValue::Flag(false)),
            "never" | "cursor" => proposal.propose(Setting::Ligatures, &entry.raw, SettingValue::Flag(true)),
            _ => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
}

fn propose_window(proposal: &mut Proposal, entries: &Entries) {
    if let Some(entry) = entries.last("background_opacity") {
        match units::number(&entry.value) {
            Some(value) => {
                proposal.propose(Setting::Opacity, &entry.raw, SettingValue::Number(units::opacity(value)))
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    if let Some(entry) = entries.last("background_blur") {
        match units::number(&entry.value) {
            Some(radius) => proposal.propose(Setting::Blur, &entry.raw, SettingValue::Flag(radius > 0.0)),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    if let Some(entry) = entries.last("scrollback_lines") {
        // A negative number is as much as there is memory for.
        match entry.value.parse::<i64>() {
            Ok(lines) if lines < 0 => proposal.propose(
                Setting::ScrollbackLines,
                &entry.raw,
                SettingValue::Count(units::unlimited_scrollback()),
            ),
            Ok(lines) => proposal.propose(
                Setting::ScrollbackLines,
                &entry.raw,
                SettingValue::Count(units::scrollback(usize::try_from(lines).unwrap_or(0))),
            ),
            Err(_) => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
}

fn propose_behaviour(proposal: &mut Proposal, env: &Env, entries: &Entries) {
    if let Some(entry) = entries.last("macos_option_as_alt") {
        let meta = match entry.value.as_str() {
            "no" => Some(OptionAsMeta::Off),
            "left" => Some(OptionAsMeta::Left),
            "right" => Some(OptionAsMeta::Right),
            "both" | "yes" => Some(OptionAsMeta::Both),
            _ => None,
        };
        match meta {
            Some(meta) => proposal.propose(Setting::OptionAsMeta, &entry.raw, SettingValue::Meta(meta)),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    if let Some(entry) = entries.last("enable_audio_bell") {
        match truthy(&entry.value) {
            Some(on) => proposal.propose(Setting::BellSound, &entry.raw, SettingValue::Flag(on)),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    if let Some(entry) = entries.last("visual_bell_duration") {
        // The duration in seconds, then an optional easing.
        let seconds = entry.value.split_whitespace().next().and_then(units::number);
        match seconds {
            Some(seconds) => {
                proposal.propose(Setting::BellVisual, &entry.raw, SettingValue::Flag(seconds > 0.0))
            }
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    // `.` is the default: the login shell.
    if let Some(entry) = entries.last("shell").filter(|e| !e.value.is_empty() && e.value != ".") {
        match shell::plain_words(&entry.value) {
            Some(words) => shell::propose(proposal, env, Source::Kitty, &entry.raw, &words),
            None => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
}

/// Words a config file uses for on and off.
fn truthy(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// The colours of an entry list, with the line they are best shown by.
fn palette<'a>(entries: &'a Entries, proposal: &mut Proposal) -> Option<(Builder, &'a Entry)> {
    let mut builder = Builder::default();
    let mut shown: Option<&Entry> = None;
    for entry in entries.iter() {
        let slot = match entry.key.as_str() {
            "foreground" => Some(&mut builder.foreground),
            "background" => Some(&mut builder.background),
            "selection_foreground" => Some(&mut builder.selection_foreground),
            "selection_background" => Some(&mut builder.selection_background),
            "cursor" => Some(&mut builder.cursor),
            "cursor_text_color" => Some(&mut builder.cursor_text),
            key => key
                .strip_prefix("color")
                .and_then(|n| n.parse::<usize>().ok())
                .and_then(|n| builder.palette.get_mut(n)),
        };
        let Some(slot) = slot else { continue };
        match parse_hex(&entry.value) {
            Ok(color) => {
                *slot = Some(color);
                if shown.is_none() || entry.key == "background" {
                    shown = Some(entry);
                }
            }
            // `none` and `background` mean "as the cell is drawn": no colour to take.
            Err(_) if matches!(entry.value.as_str(), "none" | "background" | "foreground") => {}
            Err(_) => proposal.skip(&entry.raw, Reason::UnsupportedValue),
        }
    }
    shown.map(|entry| (builder, entry))
}

fn propose_themes(
    proposal: &mut Proposal,
    entries: &Entries,
    dark: Option<&Entries>,
    light: Option<&Entries>,
) {
    let brand = Source::Kitty.brand();
    let mut slots: Vec<(Setting, &Entries, String)> = Vec::new();
    match (dark, light) {
        (None, None) => slots.push((Setting::Theme, entries, brand.to_string())),
        (dark, light) => {
            slots.push((Setting::Theme, dark.unwrap_or(entries), format!("{brand} dark")));
            slots.extend(light.map(|light| (Setting::ThemeLight, light, format!("{brand} light"))));
        }
    }
    for (setting, source, label) in slots {
        if let Some((builder, entry)) = palette(source, proposal) {
            proposal.propose_palette(setting, &entry.raw, &label, builder);
        }
    }
}

fn skip_unsupported(proposal: &mut Proposal, entries: &Entries) {
    for entry in entries.iter() {
        let kind = match entry.key.as_str() {
            "cursor_shape" | "cursor_shape_unfocused" => SkippedKind::CursorShape,
            "cursor_blink_interval" => SkippedKind::CursorBlink,
            "window_padding_width" => SkippedKind::Padding,
            "copy_on_select" => SkippedKind::CopyOnSelect,
            "confirm_os_window_close" => SkippedKind::ConfirmClose,
            "map" | "mouse_map" | "clear_all_shortcuts" | "clear_all_mouse_actions" => {
                SkippedKind::Keybinding
            }
            "env" => SkippedKind::Environment,
            "bold_font" | "italic_font" | "bold_italic_font" | "font_features" | "symbol_map" => {
                SkippedKind::FontVariant
            }
            "watcher" => {
                proposal.skip(&entry.raw, Reason::Executes);
                continue;
            }
            _ => continue,
        };
        proposal.skip_kind(&entry.raw, kind);
    }
}
