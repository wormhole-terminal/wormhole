//! Warp: `~/.warp/settings.toml` (or `~/.warp-preview/`), with a custom theme read from its YAML
//! file. Warp's own themes are inside the app, so a theme chosen by name is not in any file. Older
//! Warps kept their settings in the preferences, which is not read.

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
use super::warp_theme;
use crate::config::OptionAsMeta;

/// Read Warp's settings file and its custom themes and say what they set.
pub(crate) fn import(env: &Env, files: &mut Reader) -> Proposal {
    let Some(path) = locations::find(Source::Warp, env).into_iter().next() else {
        return Proposal::default();
    };
    let Some(text) = files.text(&path) else { return Proposal::default() };
    let table: Table = match text.parse() {
        Ok(table) => table,
        Err(err) => {
            tracing::debug!("import: {}: {err}", path.display());
            files.note(&path, Reason::Unreadable);
            return Proposal::default();
        }
    };
    let themes_dir = path.parent().unwrap_or(Path::new("/")).join("themes");
    let mut proposal = propose(env, &table, &mut |theme: &Table| {
        // A custom theme names its file; a relative one is in the themes folder.
        let file = theme.get("path").and_then(Value::as_str)?;
        let full = env.include_path(file, &themes_dir);
        files.text(&full)
    });
    // The themes come in by the closure; put the settings in order after.
    proposal.values.sort_by_key(|proposed| proposed.setting);
    proposal
}

/// What the settings table says. `custom_theme` reads a custom theme's YAML given its table.
pub(crate) fn propose(
    env: &Env,
    table: &Table,
    custom_theme: &mut dyn FnMut(&Table) -> Option<String>,
) -> Proposal {
    let mut proposal = Proposal::default();

    if let Some(name) = at(table, "appearance.text.font_name").filter(|v| v.is_str()) {
        let family = FontName::family_name(name.as_str().unwrap_or_default());
        proposal.propose(
            Setting::FontFamily,
            shown("appearance.text.font_name", name),
            SettingValue::Font(family),
        );
    }
    if let Some(value) = at(table, "appearance.text.font_size") {
        match float(value) {
            Some(points) => proposal.propose(
                Setting::FontSize,
                shown("appearance.text.font_size", value),
                SettingValue::Number(units::font_size(points)),
            ),
            None => proposal.skip("appearance.text.font_size", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "appearance.text.line_height_ratio") {
        match float(value) {
            Some(ratio) => proposal.propose(
                Setting::LineHeight,
                shown("appearance.text.line_height_ratio", value),
                SettingValue::Number(units::line_height(ratio)),
            ),
            None => proposal.skip("appearance.text.line_height_ratio", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "appearance.text.ligature_rendering_enabled")
        && let Some(on) = value.as_bool()
    {
        proposal.propose(
            Setting::Ligatures,
            shown("appearance.text.ligature_rendering_enabled", value),
            SettingValue::Flag(on),
        );
    }

    propose_themes(&mut proposal, table, custom_theme);

    // The window's opacity is a percentage, 1 to 100.
    if let Some(value) = at(table, "appearance.window.override_opacity") {
        match float(value) {
            Some(percent) => proposal.propose(
                Setting::Opacity,
                shown("appearance.window.override_opacity", value),
                SettingValue::Number(units::opacity(percent / 100.0)),
            ),
            None => proposal.skip("appearance.window.override_opacity", Reason::UnsupportedValue),
        }
    }
    if let Some(value) = at(table, "appearance.window.override_blur")
        && let Some(radius) = float(value)
    {
        proposal.propose(
            Setting::Blur,
            shown("appearance.window.override_blur", value),
            SettingValue::Flag(radius > 0.0),
        );
    }
    if let Some(value) = at(table, "terminal.input.extra_meta_keys") {
        let key =
            |name| value.as_table().and_then(|keys| keys.get(name)).and_then(Value::as_bool).unwrap_or(false);
        let meta = match (key("left_alt"), key("right_alt")) {
            (true, true) => OptionAsMeta::Both,
            (true, false) => OptionAsMeta::Left,
            (false, true) => OptionAsMeta::Right,
            (false, false) => OptionAsMeta::Off,
        };
        proposal.propose(
            Setting::OptionAsMeta,
            shown("terminal.input.extra_meta_keys", value),
            SettingValue::Meta(meta),
        );
    }
    if let Some(value) = at(table, "terminal.use_audible_bell")
        && let Some(on) = value.as_bool()
    {
        proposal.propose(
            Setting::BellSound,
            shown("terminal.use_audible_bell", value),
            SettingValue::Flag(on),
        );
    }
    // The docs call this the grid's size in rows without saying it is the scrollback, so it is
    // offered, not ticked.
    if let Some(value) = at(table, "terminal.maximum_grid_size")
        && let Some(rows) = value.as_integer().and_then(|n| usize::try_from(n).ok())
    {
        proposal.propose_unticked(
            Setting::ScrollbackLines,
            shown("terminal.maximum_grid_size", value),
            SettingValue::Count(units::scrollback(rows)),
        );
    }
    propose_shell(&mut proposal, env, table);
    skip_unsupported(&mut proposal, table);
    proposal.sorted()
}

/// A theme setting is a built-in's name, or `{ custom = { name, path } }`.
fn propose_themes(
    proposal: &mut Proposal,
    table: &Table,
    custom_theme: &mut dyn FnMut(&Table) -> Option<String>,
) {
    let mut one = |proposal: &mut Proposal, setting: Setting, path: &str, value: &Value| {
        let from = shown(path, value);
        match value {
            Value::String(name) => proposal.skip(path, Reason::BuiltInTheme(name.clone())),
            Value::Table(choice) => {
                let custom = choice.get("custom").and_then(Value::as_table);
                let Some(custom) = custom else { return proposal.skip(path, Reason::UnsupportedValue) };
                let Some(text) = custom_theme(custom) else { return proposal.skip(path, Reason::Unreadable) };
                let named = custom.get("name").and_then(Value::as_str).map(str::to_string);
                let name = named.or_else(|| warp_theme::name(&text)).unwrap_or_default();
                let label = format!("{} {name}", Source::Warp.brand());
                match warp_theme::palette(label.trim(), &text) {
                    Some(theme) => proposal.propose(setting, from, SettingValue::Palette(theme)),
                    None => proposal.skip(path, Reason::UnsupportedValue),
                }
            }
            _ => proposal.skip(path, Reason::UnsupportedValue),
        }
    };
    let system = at(table, "appearance.themes.system_theme").and_then(Value::as_bool) == Some(true);
    if system {
        for (setting, mode) in [(Setting::Theme, "dark"), (Setting::ThemeLight, "light")] {
            let path = format!("appearance.themes.selected_system_themes.{mode}");
            if let Some(value) = at(table, &path) {
                one(proposal, setting, &path, value);
            }
        }
    } else if let Some(value) = at(table, "appearance.themes.theme") {
        one(proposal, Setting::Theme, "appearance.themes.theme", value);
    }
}

/// The shell a new session starts with: `new_session_shell_override`, else the startup one.
fn propose_shell(proposal: &mut Proposal, env: &Env, table: &Table) {
    for path in ["session.new_session_shell_override", "session.startup_shell_override"] {
        let Some(value) = at(table, path) else { continue };
        let words = match value {
            Value::String(text) if text == "system_default" => continue,
            Value::String(program) => Some(vec![program.clone()]),
            Value::Table(choice) => match (choice.get("executable"), choice.get("custom")) {
                (Some(Value::String(program)), _) => Some(vec![program.clone()]),
                (_, Some(Value::String(line))) => shell::plain_words(line),
                _ => None,
            },
            _ => None,
        };
        match words {
            Some(words) => shell::propose(proposal, env, Source::Warp, &shown(path, value), &words),
            None => proposal.skip(path, Reason::UnsupportedValue),
        }
        return;
    }
}

fn skip_unsupported(proposal: &mut Proposal, table: &Table) {
    for (path, kind) in [
        ("appearance.cursor.cursor_display_type", SkippedKind::CursorShape),
        ("appearance.cursor.cursor_blink", SkippedKind::CursorBlink),
        ("session.working_directory_config", SkippedKind::WorkingDirectory),
        ("terminal.copy_on_select", SkippedKind::CopyOnSelect),
        ("general.should_confirm_close_session", SkippedKind::ConfirmClose),
        ("global_hotkey", SkippedKind::Keybinding),
    ] {
        if let Some(value) = at(table, path) {
            proposal.skip_kind(shown(path, value), kind);
        }
    }
}
