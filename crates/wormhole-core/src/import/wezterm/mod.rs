//! WezTerm: `wezterm.lua`, which is a Lua program. It is never run. A small lexer
//! (`lexer`) reads the file and `literal` the values that are written out (`config.font_size = 13`,
//! `config.color_scheme = 'Rose Pine'`, `wezterm.font('Name')`, tables of those); everything
//! computed (variables, calls, conditions, `require`d modules) is reported as dynamic Lua and left
//! alone.

mod lexer;
mod literal;

use toml::Table;

use self::lexer::{Kind, Token};
use self::literal::{Cursor, Lit};
use super::env::Env;
use super::locations;
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::Reader;
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::theme::{Builder, parse_hex};

/// The two lists of eight colours a scheme holds: the normal ANSI colours, then the bright ones.
const ANSI_KEYS: [&str; 2] = ["ansi", "brights"];

/// One `config.<key> = <value>` of the file.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Assignment {
    /// The key below `config`, with a dot for a nested one (`colors.foreground`).
    pub(crate) key: String,
    pub(crate) lit: Lit,
    /// Written inside a function or a condition, so it may not happen.
    pub(crate) conditional: bool,
}

/// What the file sets, in the order it says.
#[derive(Debug, Default)]
pub(crate) struct Extracted {
    pub(crate) assignments: Vec<Assignment>,
    /// Modules the file `require`s, which are not read.
    pub(crate) requires: Vec<String>,
}

/// Read `wezterm.lua` and say what it sets by what it writes out.
pub(crate) fn import(env: &Env, files: &mut Reader) -> Proposal {
    let Some(path) = locations::find(Source::WezTerm, env).into_iter().next() else {
        return Proposal::default();
    };
    let Some(text) = files.text(&path) else { return Proposal::default() };
    let extracted = extract(&lexer::tokenize(&text));
    // A colour scheme by name is a file of the person's, when they have it.
    let scheme = scheme_name(&extracted).and_then(|name| {
        if name.contains('/') || name.contains("..") {
            return None;
        }
        let file = locations::wezterm_colors_dir(&path, env).join(format!("{name}.toml"));
        files.text(&file)
    });
    propose(env, &extracted, scheme.as_deref())
}

/// The scheme `color_scheme` names, when it is written out.
fn scheme_name(extracted: &Extracted) -> Option<String> {
    match effective(extracted, "color_scheme")? {
        Value::Lit(Lit::Str(name)) => Some(name.clone()),
        _ => None,
    }
}

/// The assignments of a source: the keys under `config` (or the table it returns).
pub(crate) fn extract(tokens: &[Token]) -> Extracted {
    let mut out = Extracted::default();
    let mut cursor = Cursor::new(tokens);
    // Names that hold the config: `config`, and what `wezterm.config_builder()` is assigned to.
    let mut configs = vec!["config".to_string()];
    let mut block = 0usize;
    while let Some(kind) = cursor.peek() {
        let Kind::Name(name) = kind else {
            cursor.bump();
            continue;
        };
        match name.as_str() {
            "function" | "if" | "do" | "repeat" => {
                block += 1;
                cursor.bump();
            }
            "end" | "until" => {
                block = block.saturating_sub(1);
                cursor.bump();
            }
            "local" => {
                if let (Some(Kind::Name(var)), Some(Kind::Sym(eq)), Some(Kind::Name(w)), Some(Kind::Name(f))) =
                    (cursor.peek_at(1), cursor.peek_at(2), cursor.peek_at(3), cursor.peek_at(5))
                    && eq == "="
                    && w == "wezterm"
                    && f == "config_builder"
                {
                    configs.push(var.clone());
                }
                cursor.bump();
            }
            "return" if block == 0 && matches!(cursor.peek_at(1), Some(Kind::Sym(s)) if s == "{") => {
                cursor.bump();
                if let Lit::Table(fields) = cursor.table() {
                    for (key, lit) in fields {
                        if let Some(key) = key {
                            out.assignments.push(Assignment { key, lit, conditional: false });
                        }
                    }
                }
            }
            "require" => {
                cursor.bump();
                // `require 'x'` or `require('x')`
                let module = match (cursor.peek(), cursor.peek_at(1)) {
                    (Some(Kind::Str(module)), _) | (Some(Kind::Sym(_)), Some(Kind::Str(module))) => {
                        module.clone()
                    }
                    _ => String::new(),
                };
                // Every config starts with `require 'wezterm'`, which is the API and not a module of the person's.
                if module != "wezterm" {
                    out.requires.push(module);
                }
            }
            var if configs.iter().any(|c| c == var)
                && matches!(cursor.peek_at(1), Some(Kind::Sym(s)) if s == ".") =>
            {
                cursor.bump();
                let mut path = Vec::new();
                while matches!(cursor.peek(), Some(Kind::Sym(s)) if s == ".") {
                    cursor.bump();
                    if let Some(Kind::Name(part)) = cursor.peek() {
                        path.push(part.clone());
                        cursor.bump();
                    } else {
                        break;
                    }
                }
                if !path.is_empty() && matches!(cursor.peek(), Some(Kind::Sym(s)) if s == "=") {
                    cursor.bump();
                    let lit = cursor.value();
                    out.assignments.push(Assignment { key: path.join("."), lit, conditional: block > 0 });
                }
            }
            _ => cursor.bump(),
        }
    }
    out
}

/// A key's value after all assignments: what was last written, with the nested keys of a table
/// (`config.colors.foreground`) laid into it.
enum Value<'a> {
    Lit(&'a Lit),
    /// The last assignment was conditional or computed.
    Dynamic,
    /// A table built from `config.<key> = {…}` and the `config.<key>.<sub> = …` after it.
    Merged(Vec<(String, &'a Lit)>),
}

fn effective<'a>(extracted: &'a Extracted, key: &str) -> Option<Value<'a>> {
    let own = extracted.assignments.iter().rev().find(|a| a.key == key);
    let prefix = format!("{key}.");
    let nested: Vec<(String, &Lit)> = extracted
        .assignments
        .iter()
        .filter_map(|a| a.key.strip_prefix(&prefix).map(|sub| (sub.to_string(), &a.lit)))
        .collect();
    match (own, nested.is_empty()) {
        (Some(a), _) if a.conditional || a.lit == Lit::Dynamic => Some(Value::Dynamic),
        (Some(a), true) => Some(Value::Lit(&a.lit)),
        (Some(a), false) => {
            let mut fields: Vec<(String, &Lit)> = match &a.lit {
                Lit::Table(items) => items.iter().filter_map(|(k, v)| Some((k.clone()?, v))).collect(),
                _ => Vec::new(),
            };
            fields.extend(nested);
            Some(Value::Merged(fields))
        }
        (None, false) => Some(Value::Merged(nested)),
        (None, true) => None,
    }
}

/// A scalar setting: its value written out, or `None` after noting why not.
fn literal<'a>(proposal: &mut Proposal, extracted: &'a Extracted, key: &str) -> Option<&'a Lit> {
    match effective(extracted, key)? {
        Value::Lit(lit) if *lit != Lit::Nil => Some(lit),
        Value::Lit(_) => None,
        Value::Dynamic | Value::Merged(_) => {
            proposal.skip(key, Reason::DynamicLua);
            None
        }
    }
}

/// `key = value`, for the review list.
fn shown(key: &str, lit: &Lit) -> String {
    format!("{key} = {}", lit.show())
}

/// What the file writes out. `scheme` is the text of the colour scheme file `color_scheme` names, when
/// the person has one.
pub(crate) fn propose(env: &Env, extracted: &Extracted, scheme: Option<&str>) -> Proposal {
    let mut proposal = Proposal::default();
    propose_font(&mut proposal, extracted);
    propose_window(&mut proposal, extracted);
    propose_behaviour(&mut proposal, env, extracted);
    propose_theme(&mut proposal, extracted, scheme);
    skip_unsupported(&mut proposal, extracted);
    for module in &extracted.requires {
        proposal.skip(format!("require {module}"), Reason::DynamicLua);
    }
    proposal.sorted()
}

fn propose_font(proposal: &mut Proposal, extracted: &Extracted) {
    if let Some(lit) = literal(proposal, extracted, "font") {
        match lit {
            Lit::Font(names) if !names.is_empty() => proposal.propose(
                Setting::FontFamily,
                shown("font", lit),
                SettingValue::Font(FontName::family_name(&names[0])),
            ),
            _ => proposal.skip("font", Reason::UnsupportedValue),
        }
    }
    if let Some(lit) = literal(proposal, extracted, "font_size") {
        match lit {
            Lit::Num(points) => proposal.propose(
                Setting::FontSize,
                shown("font_size", lit),
                SettingValue::Number(units::font_size(*points as f32)),
            ),
            _ => proposal.skip("font_size", Reason::UnsupportedValue),
        }
    }
    if let Some(lit) = literal(proposal, extracted, "line_height") {
        match lit {
            Lit::Num(multiplier) => proposal.propose(
                Setting::LineHeight,
                shown("line_height", lit),
                SettingValue::Number(units::line_height(*multiplier as f32)),
            ),
            _ => proposal.skip("line_height", Reason::UnsupportedValue),
        }
    }
    // Ligatures come from the `calt`, `clig` and `liga` features; turning one off (`calt=0`) turns
    // them off.
    if let Some(lit) = literal(proposal, extracted, "harfbuzz_features") {
        let features: Vec<&str> = lit
            .list()
            .into_iter()
            .filter_map(|f| if let Lit::Str(s) = f { Some(s.as_str()) } else { None })
            .collect();
        let ligature = |f: &str| matches!(f.split('=').next(), Some("calt" | "clig" | "liga"));
        if features.iter().any(|f| ligature(f) && f.ends_with("=0")) {
            proposal.propose(Setting::Ligatures, shown("harfbuzz_features", lit), SettingValue::Flag(false));
        } else if features.iter().any(|f| ligature(f)) {
            proposal.propose(Setting::Ligatures, shown("harfbuzz_features", lit), SettingValue::Flag(true));
        }
    }
}

fn propose_window(proposal: &mut Proposal, extracted: &Extracted) {
    if let Some(lit) = literal(proposal, extracted, "window_background_opacity") {
        match lit {
            Lit::Num(opacity) => proposal.propose(
                Setting::Opacity,
                shown("window_background_opacity", lit),
                SettingValue::Number(units::opacity(*opacity as f32)),
            ),
            _ => proposal.skip("window_background_opacity", Reason::UnsupportedValue),
        }
    }
    if let Some(lit) = literal(proposal, extracted, "macos_window_background_blur") {
        match lit {
            Lit::Num(radius) => {
                proposal.propose(
                    Setting::Blur,
                    shown("macos_window_background_blur", lit),
                    SettingValue::Flag(*radius > 0.0),
                );
            }
            _ => proposal.skip("macos_window_background_blur", Reason::UnsupportedValue),
        }
    }
    if let Some(lit) = literal(proposal, extracted, "scrollback_lines") {
        match lit {
            Lit::Num(lines) if *lines >= 0.0 => proposal.propose(
                Setting::ScrollbackLines,
                shown("scrollback_lines", lit),
                SettingValue::Count(units::scrollback(*lines as usize)),
            ),
            _ => proposal.skip("scrollback_lines", Reason::UnsupportedValue),
        }
    }
}

fn propose_behaviour(proposal: &mut Proposal, env: &Env, extracted: &Extracted) {
    // The key names say whether the character the layout composes is typed, so false is Meta. The
    // left key composes by default no longer than the right key does not: WezTerm's defaults are
    // false for the left and true for the right.
    let left_key = "send_composed_key_when_left_alt_is_pressed";
    let right_key = "send_composed_key_when_right_alt_is_pressed";
    let left = literal(proposal, extracted, left_key);
    let right = literal(proposal, extracted, right_key);
    if left.is_some() || right.is_some() {
        let composes = |lit: Option<&Lit>, default: bool| match lit {
            Some(Lit::Bool(on)) => *on,
            _ => default,
        };
        let meta = match (!composes(left, false), !composes(right, true)) {
            (true, true) => OptionAsMeta::Both,
            (true, false) => OptionAsMeta::Left,
            (false, true) => OptionAsMeta::Right,
            (false, false) => OptionAsMeta::Off,
        };
        let from: Vec<String> = [(left_key, left), (right_key, right)]
            .iter()
            .filter_map(|(k, l)| l.map(|l| shown(k, l)))
            .collect();
        proposal.propose(Setting::OptionAsMeta, from.join(", "), SettingValue::Meta(meta));
    }
    if let Some(lit) = literal(proposal, extracted, "audible_bell") {
        match lit {
            Lit::Str(mode) if mode == "SystemBeep" => {
                proposal.propose(Setting::BellSound, shown("audible_bell", lit), SettingValue::Flag(true))
            }
            Lit::Str(mode) if mode == "Disabled" => {
                proposal.propose(Setting::BellSound, shown("audible_bell", lit), SettingValue::Flag(false))
            }
            _ => proposal.skip("audible_bell", Reason::UnsupportedValue),
        }
    }
    // A visual bell fades in and out over a number of milliseconds; none is none.
    if let Some(lit) = literal(proposal, extracted, "visual_bell") {
        let duration = |name| matches!(lit.field(name), Some(Lit::Num(ms)) if *ms > 0.0);
        let on = duration("fade_in_duration_ms") || duration("fade_out_duration_ms");
        proposal.propose(Setting::BellVisual, shown("visual_bell", lit), SettingValue::Flag(on));
    }
    if let Some(lit) = literal(proposal, extracted, "default_prog") {
        let words: Vec<String> = lit
            .list()
            .into_iter()
            .filter_map(|w| if let Lit::Str(s) = w { Some(s.clone()) } else { None })
            .collect();
        if words.is_empty() {
            proposal.skip("default_prog", Reason::UnsupportedValue);
        } else {
            shell::propose(proposal, env, Source::WezTerm, &shown("default_prog", lit), &words);
        }
    }
}

/// A WezTerm colour (`#rrggbb`, `#rgb`) as `0xRRGGBB`; a colour by name or by function is not taken.
fn color(lit: &Lit) -> Option<u32> {
    match lit {
        Lit::Str(text) if text.starts_with('#') => parse_hex(text).ok(),
        _ => None,
    }
}

/// Lay one colour table (`colors`, or a scheme file's `[colors]`) over a palette; keys that are not
/// colours Wormhole can take are said.
fn lay_colors(builder: &mut Builder, fields: &[(String, &Lit)], proposal: &mut Proposal) {
    for (name, lit) in fields {
        let slot = match name.as_str() {
            "foreground" => Some(&mut builder.foreground),
            "background" => Some(&mut builder.background),
            "cursor_bg" => Some(&mut builder.cursor),
            "cursor_fg" => Some(&mut builder.cursor_text),
            "selection_bg" => Some(&mut builder.selection_background),
            "selection_fg" => Some(&mut builder.selection_foreground),
            _ => None,
        };
        if let Some(slot) = slot {
            match color(lit) {
                Some(rgb) => *slot = Some(rgb),
                None if matches!(lit, Lit::Str(s) if s == "none") => {}
                None => proposal.skip(
                    format!("colors.{name}"),
                    if **lit == Lit::Dynamic { Reason::DynamicLua } else { Reason::UnsupportedValue },
                ),
            }
        } else if let Some(base) = ANSI_KEYS.iter().position(|k| k == name) {
            for (i, entry) in lit.list().into_iter().enumerate().take(8) {
                if let Some(rgb) = color(entry) {
                    builder.palette[base * 8 + i] = Some(rgb);
                }
            }
        }
    }
}

fn propose_theme(proposal: &mut Proposal, extracted: &Extracted, scheme_text: Option<&str>) {
    let brand = Source::WezTerm.brand();
    let mut builder = Builder::default();
    let mut label = brand.to_string();
    let mut from = String::new();
    let mut complete = false;

    // The scheme first, then `colors` over it.
    let scheme = literal(proposal, extracted, "color_scheme").and_then(|lit| match lit {
        Lit::Str(name) => Some((name.clone(), shown("color_scheme", lit))),
        _ => None,
    });
    if let Some((name, shown_scheme)) = scheme {
        match scheme_text.and_then(|text| text.parse::<Table>().ok()) {
            Some(table) => {
                let Some(colors) = table.get("colors").and_then(|c| c.as_table()) else {
                    proposal.skip("color_scheme", Reason::Unreadable);
                    return;
                };
                let pick =
                    |key: &str| colors.get(key).and_then(|v| v.as_str()).and_then(|s| parse_hex(s).ok());
                builder.foreground = pick("foreground");
                builder.background = pick("background");
                builder.cursor = pick("cursor_bg");
                builder.cursor_text = pick("cursor_fg");
                builder.selection_background = pick("selection_bg");
                builder.selection_foreground = pick("selection_fg");
                for (base, key) in ANSI_KEYS.iter().enumerate() {
                    let list = colors.get(*key).and_then(|v| v.as_array());
                    for (i, entry) in list.into_iter().flatten().enumerate().take(8) {
                        builder.palette[base * 8 + i] = entry.as_str().and_then(|s| parse_hex(s).ok());
                    }
                }
                label = format!("{brand} {name}");
                from = shown_scheme;
                complete = true;
            }
            None => proposal.skip("color_scheme", Reason::BuiltInTheme(name)),
        }
    }

    match effective(extracted, "colors") {
        Some(Value::Merged(fields)) => {
            lay_colors(&mut builder, &fields, proposal);
            if from.is_empty() {
                from = "colors".to_string();
            }
        }
        Some(Value::Lit(Lit::Table(items))) => {
            let fields: Vec<(String, &Lit)> =
                items.iter().filter_map(|(k, v)| Some((k.clone()?, v))).collect();
            lay_colors(&mut builder, &fields, proposal);
            if from.is_empty() {
                from = "colors".to_string();
            }
        }
        Some(Value::Dynamic) => proposal.skip("colors", Reason::DynamicLua),
        _ => {}
    }

    // Colours laid over a scheme that is not on disk would be a few colours on the defaults.
    let own = builder.background.is_some() && builder.foreground.is_some();
    if complete || own {
        proposal.propose(Setting::Theme, from, SettingValue::Palette(builder.finish(&label)));
    }
}

fn skip_unsupported(proposal: &mut Proposal, extracted: &Extracted) {
    let keys: [(&str, SkippedKind); 13] = [
        ("default_cursor_style", SkippedKind::CursorShape),
        ("cursor_blink_rate", SkippedKind::CursorBlink),
        ("default_cwd", SkippedKind::WorkingDirectory),
        ("window_padding", SkippedKind::Padding),
        ("window_close_confirmation", SkippedKind::ConfirmClose),
        ("skip_close_confirmation_for_processes_named", SkippedKind::ConfirmClose),
        ("keys", SkippedKind::Keybinding),
        ("key_tables", SkippedKind::Keybinding),
        ("mouse_bindings", SkippedKind::CopyOnSelect),
        ("leader", SkippedKind::Keybinding),
        ("set_environment_variables", SkippedKind::Environment),
        ("font_rules", SkippedKind::FontVariant),
        ("cell_width", SkippedKind::CellWidth),
    ];
    for (key, kind) in keys {
        match effective(extracted, key) {
            Some(Value::Dynamic) => proposal.skip(key, Reason::DynamicLua),
            Some(Value::Lit(lit)) => proposal.skip_kind(shown(key, lit), kind),
            Some(Value::Merged(_)) => proposal.skip_kind(key, kind),
            None => {}
        }
    }
}
