//! VS Code's integrated terminal: `~/Library/Application Support/Code/User/settings.json` (JSON
//! with comments; Insiders and VSCodium are read the same way). The terminal's colours are the
//! person's own `workbench.colorCustomizations`; the colours of the active colour theme are in the
//! theme's extension and are not read.

use std::path::PathBuf;

use serde_json::Value;

use super::env::Env;
use super::json_value::{at, shown};
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::Reader;
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::storage::jsonc;
use crate::theme::{Builder, parse_hex};

/// Read VS Code's user settings and say what they set for the terminal.
pub(crate) fn import(env: &Env, files: &mut Reader, located: &[PathBuf]) -> Proposal {
    let Some(path) = located.first() else { return Proposal::default() };
    let Some(text) = files.text(path) else { return Proposal::default() };
    match jsonc::parse::<Value>(&text) {
        Ok(root) => propose(env, &root),
        Err(err) => {
            tracing::debug!("import: {}: {err}", path.display());
            files.note(path, Reason::Unreadable);
            Proposal::default()
        }
    }
}

/// A setting by its name: the flat key VS Code writes (`terminal.integrated.fontSize`), or the same
/// path as nested objects.
fn get<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    root.get(key).or_else(|| at(root, key))
}

/// The first family of a CSS font list (`'Fira Code', Menlo, monospace`) that names a font and not
/// a generic family.
fn first_family(list: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in list.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c == ',' => parts.push(std::mem::take(&mut current)),
            None => current.push(c),
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .find(|part| !part.is_empty() && !matches!(part.as_str(), "monospace" | "sans-serif" | "serif"))
}

/// What the settings say about the terminal.
pub(crate) fn propose(env: &Env, root: &Value) -> Proposal {
    let mut proposal = Proposal::default();
    let key = |name: &str| format!("terminal.integrated.{name}");

    // The terminal's font family is the editor's when it has none of its own.
    let family = [key("fontFamily"), "editor.fontFamily".to_string()]
        .into_iter()
        .find_map(|k| get(root, &k).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(|s| (k, s)));
    if let Some((k, list)) = family
        && let Some(name) = first_family(list)
    {
        proposal.propose(
            Setting::FontFamily,
            format!("{k} = \"{list}\""),
            SettingValue::Font(FontName::family_name(name)),
        );
    }
    // Sizes are CSS pixels, which are points on a Mac.
    if let Some(value) = get(root, &key("fontSize")) {
        match value.as_f64().and_then(units::finite) {
            Some(px) => proposal.propose(
                Setting::FontSize,
                shown(&key("fontSize"), value),
                SettingValue::Number(units::font_size(px)),
            ),
            None => proposal.skip(key("fontSize"), Reason::UnsupportedValue),
        }
    }
    // The line height is a multiplier of the font size (0 is the default, which is 1).
    if let Some(value) = get(root, &key("lineHeight")) {
        let multiplier = value.as_f64().filter(|n| *n > 0.0).and_then(units::finite);
        match multiplier {
            Some(m) => proposal.propose(
                Setting::LineHeight,
                shown(&key("lineHeight"), value),
                SettingValue::Number(units::line_height(m)),
            ),
            None => proposal.skip(key("lineHeight"), Reason::UnsupportedValue),
        }
    }
    if let Some(value) = get(root, &key("fontLigatures.enabled"))
        && let Some(on) = value.as_bool()
    {
        proposal.propose(
            Setting::Ligatures,
            shown(&key("fontLigatures.enabled"), value),
            SettingValue::Flag(on),
        );
    }
    if let Some(value) = get(root, &key("scrollback")) {
        match value.as_u64().and_then(|n| usize::try_from(n).ok()) {
            Some(lines) => proposal.propose(
                Setting::ScrollbackLines,
                shown(&key("scrollback"), value),
                SettingValue::Count(units::scrollback(lines)),
            ),
            None => proposal.skip(key("scrollback"), Reason::UnsupportedValue),
        }
    }
    if let Some(value) = get(root, &key("macOptionIsMeta"))
        && let Some(on) = value.as_bool()
    {
        let meta = if on { OptionAsMeta::Both } else { OptionAsMeta::Off };
        proposal.propose(
            Setting::OptionAsMeta,
            shown(&key("macOptionIsMeta"), value),
            SettingValue::Meta(meta),
        );
    }
    propose_bell(&mut proposal, root);
    propose_shell(&mut proposal, env, root);
    propose_palette(&mut proposal, root);
    skip_unsupported(&mut proposal, root);
    proposal.sorted()
}

fn propose_bell(proposal: &mut Proposal, root: &Value) {
    if let Some(value) = get(root, "terminal.integrated.enableVisualBell")
        && let Some(on) = value.as_bool()
    {
        proposal.propose(
            Setting::BellVisual,
            shown("terminal.integrated.enableVisualBell", value),
            SettingValue::Flag(on),
        );
    }
    // The sound is the accessibility signal's (`auto` follows the screen reader, which says nothing
    // about a person's choice); the deprecated `enableBell` is the older switch for it.
    let signal = get(root, "accessibility.signals.terminalBell").and_then(|signal| signal.get("sound"));
    match signal.and_then(Value::as_str) {
        Some("on") => proposal.propose(
            Setting::BellSound,
            "accessibility.signals.terminalBell.sound = \"on\"",
            SettingValue::Flag(true),
        ),
        Some("off") => proposal.propose(
            Setting::BellSound,
            "accessibility.signals.terminalBell.sound = \"off\"",
            SettingValue::Flag(false),
        ),
        _ => {
            if let Some(value) = get(root, "terminal.integrated.enableBell")
                && let Some(on) = value.as_bool()
            {
                proposal.propose(
                    Setting::BellSound,
                    shown("terminal.integrated.enableBell", value),
                    SettingValue::Flag(on),
                );
            }
        }
    }
}

/// The default profile: a profile of the person's (`profiles.osx`), or a built-in one by name.
fn propose_shell(proposal: &mut Proposal, env: &Env, root: &Value) {
    let default = "terminal.integrated.defaultProfile.osx";
    let Some(name_value) = get(root, default) else { return };
    let Some(name) = name_value.as_str() else { return };
    let profile = get(root, "terminal.integrated.profiles.osx").and_then(|profiles| profiles.get(name));
    let from = shown(default, name_value);

    let (program, args) = match profile {
        Some(profile) => {
            let path = match profile.get("path") {
                Some(Value::String(path)) => Some(path.clone()),
                Some(Value::Array(paths)) => paths.iter().find_map(|p| p.as_str().map(str::to_string)),
                _ => None,
            };
            let args: Vec<String> = profile
                .get("args")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|a| a.as_str().map(str::to_string))
                .collect();
            (path, args)
        }
        None => (Some(name.to_string()), Vec::new()),
    };
    // A path may use variables (`${env:HOME}`), which belong to VS Code.
    let program = program.filter(|p| !p.contains("${") && !args.iter().any(|a| a.contains("${")));
    let Some(program) = program else { return proposal.skip(default, Reason::UnsupportedValue) };
    // A bare name is a shell VS Code finds itself; the system's list has where it is.
    let program = if program.contains('/') {
        program
    } else {
        env.system_shells
            .iter()
            .find(|shell| shell.rsplit('/').next() == Some(program.as_str()))
            .cloned()
            .unwrap_or(program)
    };
    if profile.is_none() && !program.contains('/') {
        return proposal.skip(default, Reason::UnsupportedValue);
    }
    let words: Vec<String> = std::iter::once(program).chain(args).collect();
    shell::propose(proposal, env, Source::VsCode, &from, &words);
}

/// The palette from the person's own colour overrides. The colours of the theme underneath are
/// not in settings, so an override of a few colours is not a palette; one with both background and
/// foreground is taken, with Wormhole's defaults for the rest.
fn propose_palette(proposal: &mut Proposal, root: &Value) {
    let theme_name = root.get("workbench.colorTheme").and_then(Value::as_str);
    let custom = root.get("workbench.colorCustomizations").and_then(Value::as_object);
    let scoped = theme_name.and_then(|name| custom?.get(&format!("[{name}]"))?.as_object());
    let color = |key: &str| -> Option<u32> {
        let value = scoped.and_then(|s| s.get(key)).or_else(|| custom?.get(key))?.as_str()?;
        // `#rrggbbaa` and `#rgba` carry an alpha; the terminal draws the colour opaque.
        let opaque = match value.len() {
            9 => value.get(..7).unwrap_or(value),
            5 => value.get(..4).unwrap_or(value),
            _ => value,
        };
        parse_hex(opaque).ok()
    };
    let mut builder = Builder {
        foreground: color("terminal.foreground"),
        background: color("terminal.background"),
        cursor: color("terminalCursor.foreground"),
        cursor_text: color("terminalCursor.background"),
        selection_background: color("terminal.selectionBackground"),
        selection_foreground: color("terminal.selectionForeground"),
        ..Builder::default()
    };
    builder.fill_ansi(|bright, ansi| {
        let ansi = units::capitalized(ansi);
        color(&format!("terminal.ansi{}{ansi}", if bright { "Bright" } else { "" }))
    });
    let label = match theme_name {
        Some(name) => format!("{} {name}", Source::VsCode.brand()),
        None => Source::VsCode.brand().to_string(),
    };
    proposal.propose_palette(Setting::Theme, "workbench.colorCustomizations", &label, builder);
    if let Some(name) = theme_name {
        proposal.skip("workbench.colorTheme", Reason::BuiltInTheme(name.to_string()));
    }
}

fn skip_unsupported(proposal: &mut Proposal, root: &Value) {
    for (name, kind) in [
        ("cursorStyle", SkippedKind::CursorShape),
        ("cursorBlinking", SkippedKind::CursorBlink),
        ("cwd", SkippedKind::WorkingDirectory),
        ("splitCwd", SkippedKind::WorkingDirectory),
        ("copyOnSelection", SkippedKind::CopyOnSelect),
        ("confirmOnExit", SkippedKind::ConfirmClose),
        ("confirmOnKill", SkippedKind::ConfirmClose),
        ("letterSpacing", SkippedKind::CellWidth),
        ("fontWeight", SkippedKind::FontVariant),
        ("fontWeightBold", SkippedKind::FontVariant),
    ] {
        let key = format!("terminal.integrated.{name}");
        if let Some(value) = get(root, &key) {
            proposal.skip_kind(shown(&key, value), kind);
        }
    }
    if let Some(Value::Object(env)) = get(root, "terminal.integrated.env.osx") {
        for (name, value) in env {
            proposal
                .skip_kind(format!("terminal.integrated.env.osx.{name} = {value}"), SkippedKind::Environment);
        }
    }
}
