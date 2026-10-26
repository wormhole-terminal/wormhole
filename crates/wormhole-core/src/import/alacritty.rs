//! Alacritty: the first of its config files that exists, `alacritty.toml` and then the YAML name it
//! had before 0.13 (which is no longer read). `import` lists are merged under the file that names
//! them, which wins, five files deep at most. `bell.command` runs a program and is never taken.

use std::path::Path;

use toml::{Table, Value};

use super::env::Env;
use super::locations;
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::Reader;
use super::shell;
use super::source::Source;
use super::table::{at, float, shown};
use super::units;
use crate::config::OptionAsMeta;
use crate::theme;

/// What Alacritty draws when `font.size` is not set.
const DEFAULT_FONT_SIZE: f32 = 11.25;

/// How many files an `import` chain may nest, the main file counted (Alacritty's own limit).
pub(crate) const MAX_DEPTH: usize = 6;

/// Read Alacritty's config and its imports and say what they set.
pub(crate) fn import(env: &Env, files: &mut Reader) -> Proposal {
    let Some(path) = locations::find(Source::Alacritty, env).into_iter().next() else {
        return Proposal::default();
    };
    if matches!(path.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")) {
        files.seen(&path);
        files.note(&path, Reason::OldFormat);
        return Proposal::default();
    }
    match load(env, files, &path) {
        Some(table) => propose(env, &table),
        None => Proposal::default(),
    }
}

/// A file's table with its imports merged under it.
fn load(env: &Env, files: &mut Reader, path: &Path) -> Option<Table> {
    files.nested(path, |files, text| {
        let table: Table = match text.parse() {
            Ok(table) => table,
            Err(err) => {
                tracing::debug!("import: {}: {err}", path.display());
                files.note(path, Reason::Unreadable);
                return Table::new();
            }
        };
        let dir = path.parent().unwrap_or(Path::new("/"));
        let mut merged = Table::new();
        for name in imports(&table) {
            if let Some(imported) = load(env, files, &env.include_path(&name, dir)) {
                merge(&mut merged, imported);
            }
        }
        merge(&mut merged, table);
        merged
    })
}

/// The `import` list: `general.import`, or the top-level `import` older files have. A list that is
/// not all strings is not used, as Alacritty ignores it.
fn imports(table: &Table) -> Vec<String> {
    let list = table
        .get("general")
        .and_then(Value::as_table)
        .and_then(|general| general.get("import"))
        .or_else(|| table.get("import"));
    let Some(Value::Array(items)) = list else { return Vec::new() };
    items.iter().map(|item| item.as_str().map(str::to_string)).collect::<Option<Vec<_>>>().unwrap_or_default()
}

/// `over` laid over `base`: tables merge key by key, anything else is replaced.
fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(over)) => merge(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// What the merged table sets.
pub(crate) fn propose(env: &Env, table: &Table) -> Proposal {
    let mut proposal = Proposal::default();

    if let Some(family) = at(table, "font.normal.family").filter(|v| v.is_str()) {
        let name = family.as_str().unwrap_or_default();
        proposal.propose(
            Setting::FontFamily,
            shown("font.normal.family", family),
            SettingValue::Font(FontName::family_name(name)),
        );
    }
    let mut size = DEFAULT_FONT_SIZE;
    if let Some(value) = at(table, "font.size") {
        match float(value) {
            Some(points) => {
                size = points;
                proposal.propose(
                    Setting::FontSize,
                    shown("font.size", value),
                    SettingValue::Number(units::font_size(points)),
                );
            }
            None => proposal.skip("font.size", Reason::UnsupportedValue),
        }
    }
    // `font.offset.y` adds pixels to the cell's height, which is not a multiplier.
    if let Some(value) = at(table, "font.offset.y") {
        match float(value) {
            Some(delta) => proposal.propose(
                Setting::LineHeight,
                shown("font.offset.y", value),
                SettingValue::Number(units::line_height(units::added_height_multiplier(delta, size))),
            ),
            None => proposal.skip("font.offset.y", Reason::UnsupportedValue),
        }
    }

    propose_colors(&mut proposal, table);
    propose_window(&mut proposal, table);
    propose_shell(&mut proposal, env, table);

    if let Some(value) = at(table, "scrolling.history") {
        match value.as_integer().and_then(|n| usize::try_from(n).ok()) {
            Some(lines) => proposal.propose(
                Setting::ScrollbackLines,
                shown("scrolling.history", value),
                SettingValue::Count(units::scrollback(lines)),
            ),
            None => proposal.skip("scrolling.history", Reason::UnsupportedValue),
        }
    }
    // Alacritty has no sound of its own; a visual bell lasts `duration` milliseconds, 0 for none.
    if let Some(value) = at(table, "bell.duration") {
        match value.as_integer() {
            Some(ms) => proposal.propose(
                Setting::BellVisual,
                shown("bell.duration", value),
                SettingValue::Flag(ms > 0),
            ),
            None => proposal.skip("bell.duration", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "bell.command") {
        proposal.skip(shown("bell.command", value), Reason::Executes);
    }
    skip_unsupported(&mut proposal, table);
    proposal.sorted()
}

fn propose_colors(proposal: &mut Proposal, table: &Table) {
    let Some(colors) = table.get("colors").and_then(Value::as_table).filter(|colors| !colors.is_empty())
    else {
        return;
    };
    let only_colors = Table::from_iter([("colors".to_string(), Value::Table(colors.clone()))]);
    let text = toml::to_string(&only_colors).unwrap_or_default();
    match theme::parse_alacritty(Source::Alacritty.brand(), &text) {
        Ok(theme) => proposal.propose(Setting::Theme, "[colors]", SettingValue::Palette(theme)),
        Err(_) => proposal.skip("colors", Reason::UnsupportedValue),
    }
}

fn propose_window(proposal: &mut Proposal, table: &Table) {
    if let Some(value) = at(table, "window.opacity") {
        match float(value) {
            Some(opacity) => proposal.propose(
                Setting::Opacity,
                shown("window.opacity", value),
                SettingValue::Number(units::opacity(opacity)),
            ),
            None => proposal.skip("window.opacity", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "window.blur") {
        match value.as_bool() {
            Some(on) => proposal.propose(Setting::Blur, shown("window.blur", value), SettingValue::Flag(on)),
            None => proposal.skip("window.blur", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "window.option_as_alt") {
        let meta = match value.as_str() {
            Some("OnlyLeft") => Some(OptionAsMeta::Left),
            Some("OnlyRight") => Some(OptionAsMeta::Right),
            Some("Both") => Some(OptionAsMeta::Both),
            Some("None") => Some(OptionAsMeta::Off),
            _ => None,
        };
        match meta {
            Some(meta) => proposal.propose(
                Setting::OptionAsMeta,
                shown("window.option_as_alt", value),
                SettingValue::Meta(meta),
            ),
            None => proposal.skip("window.option_as_alt", Reason::UnsupportedValue),
        }
    }
}

/// `terminal.shell`, or the top-level `shell` older files have: a program, or a program and
/// arguments.
fn propose_shell(proposal: &mut Proposal, env: &Env, table: &Table) {
    let (path, value) = match at(table, "terminal.shell") {
        Some(value) => ("terminal.shell", value),
        None => match table.get("shell") {
            Some(value) => ("shell", value),
            None => return,
        },
    };
    let words: Option<Vec<String>> = match value {
        Value::String(program) => Some(vec![program.clone()]),
        Value::Table(shell) => shell.get("program").and_then(Value::as_str).map(|program| {
            let args = shell.get("args").and_then(Value::as_array).into_iter().flatten();
            std::iter::once(program.to_string())
                .chain(args.filter_map(|a| a.as_str().map(str::to_string)))
                .collect()
        }),
        _ => None,
    };
    match words {
        Some(words) => shell::propose(proposal, env, Source::Alacritty, &shown(path, value), &words),
        None => proposal.skip(path, Reason::UnsupportedValue),
    }
}

fn skip_unsupported(proposal: &mut Proposal, table: &Table) {
    let kinds: [(&str, SkippedKind); 15] = [
        ("cursor.style", SkippedKind::CursorShape),
        ("cursor.blink_interval", SkippedKind::CursorBlink),
        ("window.padding", SkippedKind::Padding),
        ("window.dynamic_padding", SkippedKind::Padding),
        ("selection.save_to_clipboard", SkippedKind::CopyOnSelect),
        ("general.working_directory", SkippedKind::WorkingDirectory),
        ("working_directory", SkippedKind::WorkingDirectory),
        ("font.normal.style", SkippedKind::FontVariant),
        ("font.bold", SkippedKind::FontVariant),
        ("font.italic", SkippedKind::FontVariant),
        ("font.bold_italic", SkippedKind::FontVariant),
        ("font.offset.x", SkippedKind::CellWidth),
        ("font.glyph_offset", SkippedKind::CellWidth),
        ("keyboard.bindings", SkippedKind::Keybinding),
        ("mouse.bindings", SkippedKind::Keybinding),
    ];
    for (path, kind) in kinds {
        if let Some(value) = at(table, path) {
            proposal.skip_kind(shown(path, value), kind);
        }
    }
    // The older names for the same.
    for (path, kind) in
        [("key_bindings", SkippedKind::Keybinding), ("mouse_bindings", SkippedKind::Keybinding)]
    {
        if let Some(value) = table.get(path) {
            proposal.skip_kind(shown(path, value), kind);
        }
    }
    if let Some(Value::Table(env)) = table.get("env") {
        for (name, value) in env {
            proposal.skip_kind(format!("env.{name} = {value}"), SkippedKind::Environment);
        }
    }
}
