//! Zed's terminal: `~/.config/zed/settings.json` (JSON with comments), its `terminal` object with
//! the editor's `buffer_font_*` settings behind it, and the theme named in `theme`, read from the
//! theme files on disk. Zed's own themes are inside the app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::env::Env;
use super::locations;
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::Reader;
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::storage::jsonc;
use crate::theme::{Builder, parse_hex};

/// The ANSI colours' names in palette order, normal then bright.
const ANSI: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

/// Read Zed's settings and the theme files they point at and say what they set.
pub(crate) fn import(env: &Env, files: &mut Reader) -> Proposal {
    let Some(path) = locations::find(Source::Zed, env).into_iter().next() else {
        return Proposal::default();
    };
    let Some(text) = files.text(&path) else { return Proposal::default() };
    let root: Value = match jsonc::parse(&text) {
        Ok(root) => root,
        Err(err) => {
            tracing::debug!("import: {}: {err}", path.display());
            files.note(&path, Reason::Unreadable);
            return Proposal::default();
        }
    };
    let wanted = ThemeChoice::of(&root).names();
    let found = find_themes(env, files, &wanted);
    propose(env, &root, &found)
}

/// The theme setting: one name, or one per appearance.
struct ThemeChoice {
    dark: Option<String>,
    light: Option<String>,
}

impl ThemeChoice {
    fn of(root: &Value) -> Self {
        match root.get("theme") {
            Some(Value::String(name)) => Self { dark: Some(name.clone()), light: None },
            Some(Value::Object(choice)) => {
                let name = |key: &str| choice.get(key).and_then(Value::as_str).map(str::to_string);
                Self { dark: name("dark"), light: name("light") }
            }
            _ => Self { dark: None, light: None },
        }
    }

    fn names(&self) -> Vec<String> {
        self.dark.iter().chain(&self.light).cloned().collect()
    }
}

/// The themes called `wanted`, from the theme files of the person's own folder and of the installed
/// extensions, by name. Reading stops when all are found or the file limit is reached.
fn find_themes(env: &Env, files: &mut Reader, wanted: &[String]) -> BTreeMap<String, Value> {
    let mut found = BTreeMap::new();
    if wanted.is_empty() {
        return found;
    }
    for path in theme_files(env) {
        if found.len() == wanted.len() || !files.has_room() {
            break;
        }
        let Some(text) = files.text(&path) else { continue };
        let Ok(file) = jsonc::parse::<Value>(&text) else { continue };
        for theme in file.get("themes").and_then(Value::as_array).into_iter().flatten() {
            let name = theme.get("name").and_then(Value::as_str);
            if let Some(name) = name.filter(|name| wanted.iter().any(|w| w == name)) {
                found.entry(name.to_string()).or_insert_with(|| theme.clone());
            }
        }
    }
    found
}

/// `*.json` in the person's themes folder, then in each installed extension's `themes` folder.
fn theme_files(env: &Env) -> Vec<PathBuf> {
    let [own, extensions] = <[PathBuf; 2]>::try_from(locations::zed_theme_dirs(env)).unwrap_or_default();
    let mut out = json_files(&own);
    let mut installed: Vec<PathBuf> = std::fs::read_dir(&extensions)
        .map(|read| read.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    installed.sort();
    for extension in installed {
        out.extend(json_files(&extension.join("themes")));
    }
    out
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|read| {
            read.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(root, |value, part| value.get(part))
}

/// `path = value`, for the review list.
fn shown(path: &str, value: &Value) -> String {
    format!("{path} = {value}")
}

/// What the settings say; `themes` holds the theme objects found by name.
pub(crate) fn propose(env: &Env, root: &Value, themes: &BTreeMap<String, Value>) -> Proposal {
    let mut proposal = Proposal::default();
    // The terminal's own setting, else the editor's.
    let pick = |own: &str, editor: &str| -> Option<(String, &Value)> {
        let path = if at(root, own).is_some() { own } else { editor };
        at(root, path).map(|value| (path.to_string(), value))
    };

    if let Some((path, value)) = pick("terminal.font_family", "buffer_font_family")
        && let Some(name) = value.as_str().filter(|name| !name.trim().is_empty())
    {
        proposal.propose(
            Setting::FontFamily,
            shown(&path, value),
            SettingValue::Font(FontName::family_name(name)),
        );
    }
    if let Some((path, value)) = pick("terminal.font_size", "buffer_font_size") {
        match value.as_f64() {
            Some(size) => proposal.propose(
                Setting::FontSize,
                shown(&path, value),
                SettingValue::Number(units::font_size(size as f32)),
            ),
            None => proposal.skip(path, Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(root, "terminal.line_height") {
        let multiplier = match value {
            Value::String(name) if name == "comfortable" => Some(1.618),
            Value::String(name) if name == "standard" => Some(1.3),
            other => other.get("custom").and_then(Value::as_f64).map(|m| m as f32),
        };
        match multiplier {
            Some(m) => proposal.propose(
                Setting::LineHeight,
                shown("terminal.line_height", value),
                SettingValue::Number(units::line_height(m)),
            ),
            None => proposal.skip("terminal.line_height", Reason::UnsupportedValue),
        }
    }
    // Ligatures are on unless the `calt` feature is turned off.
    if let Some((path, value)) = pick("terminal.font_features", "buffer_font_features")
        && let Some(calt) = value.get("calt")
    {
        let on = calt.as_bool().unwrap_or_else(|| calt.as_i64().is_some_and(|n| n != 0));
        proposal.propose(Setting::Ligatures, shown(&path, value), SettingValue::Flag(on));
    }

    if let Some(value) = at(root, "terminal.option_as_meta")
        && let Some(on) = value.as_bool()
    {
        let meta = if on { OptionAsMeta::Both } else { OptionAsMeta::Off };
        proposal.propose(
            Setting::OptionAsMeta,
            shown("terminal.option_as_meta", value),
            SettingValue::Meta(meta),
        );
    }
    if let Some(value) = at(root, "terminal.bell") {
        match value.as_str() {
            Some("system") => {
                proposal.propose(Setting::BellSound, shown("terminal.bell", value), SettingValue::Flag(true))
            }
            Some("off") => {
                proposal.propose(Setting::BellSound, shown("terminal.bell", value), SettingValue::Flag(false))
            }
            _ => proposal.skip("terminal.bell", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(root, "terminal.max_scroll_history_lines") {
        match value.as_u64().and_then(|n| usize::try_from(n).ok()) {
            Some(lines) => proposal.propose(
                Setting::ScrollbackLines,
                shown("terminal.max_scroll_history_lines", value),
                SettingValue::Count(units::scrollback(lines)),
            ),
            None => proposal.skip("terminal.max_scroll_history_lines", Reason::UnsupportedValue),
        }
    }
    propose_shell(&mut proposal, env, root);
    propose_themes(&mut proposal, root, themes);
    skip_unsupported(&mut proposal, root);
    proposal.sorted()
}

/// `terminal.shell`: `"system"`, `{ "program": … }` or `{ "with_arguments": { "program", "args" } }`.
fn propose_shell(proposal: &mut Proposal, env: &Env, root: &Value) {
    let Some(value) = at(root, "terminal.shell") else { return };
    let words = match value {
        Value::String(_) => return,
        Value::Object(choice) => match (choice.get("program"), choice.get("with_arguments")) {
            (Some(Value::String(program)), _) => Some(vec![program.clone()]),
            (_, Some(Value::Object(with))) => with.get("program").and_then(Value::as_str).map(|program| {
                let args = with.get("args").and_then(Value::as_array).into_iter().flatten();
                std::iter::once(program.to_string())
                    .chain(args.filter_map(|a| a.as_str().map(str::to_string)))
                    .collect()
            }),
            _ => None,
        },
        _ => None,
    };
    match words {
        Some(words) => shell::propose(proposal, env, Source::Zed, &shown("terminal.shell", value), &words),
        None => proposal.skip("terminal.shell", Reason::UnsupportedValue),
    }
}

/// A theme's terminal colours, with the person's `theme_overrides` for it laid over.
fn propose_themes(proposal: &mut Proposal, root: &Value, themes: &BTreeMap<String, Value>) {
    let choice = ThemeChoice::of(root);
    let from = root.get("theme").map(|value| shown("theme", value)).unwrap_or_default();
    for (setting, name) in [(Setting::Theme, choice.dark), (Setting::ThemeLight, choice.light)] {
        let Some(name) = name else { continue };
        let Some(theme) = themes.get(&name) else {
            proposal.skip("theme", Reason::BuiltInTheme(name));
            continue;
        };
        let overrides = |key: &str| root.get(key).and_then(|all| all.get(&name));
        let style = |key: &str| {
            overrides("theme_overrides")
                .and_then(|o| o.get(key))
                .or_else(|| overrides("experimental.theme_overrides").and_then(|o| o.get(key)))
                .or_else(|| theme.get("style").and_then(|s| s.get(key)))
                .and_then(Value::as_str)
        };
        // Zed's colours may carry an alpha (`#rrggbbaa`); the terminal draws them opaque.
        let color = |key: String| {
            style(&key).and_then(|c| parse_hex(c.get(..7).filter(|_| c.len() == 9).unwrap_or(c)).ok())
        };
        let mut builder = Builder {
            foreground: color("terminal.foreground".into()),
            background: color("terminal.background".into()),
            ..Builder::default()
        };
        for (i, ansi) in ANSI.iter().enumerate() {
            builder.palette[i] = color(format!("terminal.ansi.{ansi}"));
            builder.palette[i + 8] = color(format!("terminal.ansi.bright_{ansi}"));
        }
        if builder.background.is_none() && builder.foreground.is_none() {
            proposal.skip("theme", Reason::UnsupportedValue);
            continue;
        }
        let label = format!("{} {name}", Source::Zed.brand());
        proposal.propose(setting, from.clone(), SettingValue::Palette(builder.finish(&label)));
    }
}

fn skip_unsupported(proposal: &mut Proposal, root: &Value) {
    for (path, kind) in [
        ("terminal.cursor_shape", SkippedKind::CursorShape),
        ("terminal.blinking", SkippedKind::CursorBlink),
        ("terminal.working_directory", SkippedKind::WorkingDirectory),
        ("terminal.copy_on_select", SkippedKind::CopyOnSelect),
        ("terminal.font_weight", SkippedKind::FontVariant),
        ("terminal.font_fallbacks", SkippedKind::FontVariant),
    ] {
        if let Some(value) = at(root, path) {
            proposal.skip_kind(shown(path, value), kind);
        }
    }
    if let Some(Value::Object(env)) = at(root, "terminal.env") {
        for (name, value) in env {
            proposal.skip_kind(format!("terminal.env.{name} = {value}"), SkippedKind::Environment);
        }
    }
}
