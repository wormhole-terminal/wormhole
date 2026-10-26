//! iTerm2: `com.googlecode.iterm2.plist` (or the custom folder it points to), the profile named by
//! `Default Bookmark Guid`, found among `New Bookmarks` or the Dynamic Profiles.

use std::path::{Path, PathBuf};

use plist::{Dictionary, Value};

use super::env::Env;
use super::locations;
use super::plist_value::{
    self, boolean, integer, read_dictionary, read_dictionary_in_home, real, show, shown, string,
};
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::{MAX_FILE_BYTES, Reader};
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::theme::iterm_builder;

/// The most Dynamic Profiles files looked into for a default profile that is not among the bookmarks.
const MAX_DYNAMIC_FILES: usize = 16;

/// The colours of a profile, which the palette is made of.
const COLOR_KEYS: [&str; 6] = [
    "Foreground Color",
    "Background Color",
    "Cursor Color",
    "Cursor Text Color",
    "Selection Color",
    "Selected Text Color",
];

/// Read iTerm2's preferences and say what its default profile sets.
pub(crate) fn import(env: &Env, files: &mut Reader, located: &[PathBuf]) -> Proposal {
    let Some(mut root) = located.first().and_then(|path| read_dictionary(files, path)) else {
        return Proposal::default();
    };
    // "Load preferences from a custom folder" moves the whole file; the main one then only says where.
    // The folder is named by the file, so it is read only inside the home folder.
    if let Some(custom) = custom_file(env, files, &root)
        && let Some(moved) = read_dictionary_in_home(files, &custom)
    {
        root = moved;
    }
    let Some(profile) = default_profile(env, files, &root) else { return Proposal::default() };
    propose(env, &root, &profile)
}

/// The preferences file `PrefsCustomFolder` names: a folder with the file in it, or the file. A
/// URL is not read, and neither is a relative path, which iTerm2 itself does not write.
fn custom_file(env: &Env, files: &mut Reader, root: &Dictionary) -> Option<PathBuf> {
    if !boolean(root, "LoadPrefsFromCustomFolder")? {
        return None;
    }
    let folder = string(root, "PrefsCustomFolder")?;
    if folder.contains("://") {
        return None;
    }
    let path = env.expand_tilde(folder);
    if !path.is_absolute() {
        files.note(Path::new("PrefsCustomFolder"), Reason::UnsupportedValue);
        return None;
    }
    Some(if path.extension().is_some_and(|e| e == "plist") {
        path
    } else {
        path.join("com.googlecode.iterm2.plist")
    })
}

fn profiles(root: &Dictionary) -> Vec<&Dictionary> {
    root.get("New Bookmarks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_dictionary)
        .collect()
}

/// The profile new windows use: the bookmark whose `Guid` is the default's, else a Dynamic
/// Profile with it, else the first bookmark.
fn default_profile(env: &Env, files: &mut Reader, root: &Dictionary) -> Option<Dictionary> {
    let list = profiles(root);
    let guid = string(root, "Default Bookmark Guid");
    if let Some(found) = guid.and_then(|guid| list.iter().find(|p| string(p, "Guid") == Some(guid))) {
        return Some((*found).clone());
    }
    if let Some(guid) = guid
        && let Some(found) = dynamic_profile(env, files, guid, &list)
    {
        return Some(found);
    }
    list.first().map(|first| (*first).clone())
}

/// A Dynamic Profile by `Guid`: a file of JSON or a property list holding `{"Profiles": […]}`. It
/// may name a parent profile, whose keys it overrides.
fn dynamic_profile(
    env: &Env,
    files: &mut Reader,
    guid: &str,
    bookmarks: &[&Dictionary],
) -> Option<Dictionary> {
    let dir = locations::iterm_dynamic_profiles(env);
    let mut names: Vec<_> = std::fs::read_dir(&dir).ok()?.filter_map(Result::ok).map(|e| e.path()).collect();
    names.sort();
    let mut found: Option<Dictionary> = None;
    for path in names.iter().filter(|p| p.is_file()).take(MAX_DYNAMIC_FILES) {
        let Some(bytes) = files.bytes(path, MAX_FILE_BYTES) else { continue };
        let value = if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
            serde_json::from_slice::<serde_json::Value>(&bytes).ok().map(|json| from_json(&json))
        } else {
            plist_value::parse(&bytes)
        };
        let Some(Value::Dictionary(file)) = value else { continue };
        let listed = file.get("Profiles").and_then(Value::as_array).into_iter().flatten();
        // Files are read in name order and the last profile with a Guid is the one in effect.
        if let Some(profile) =
            listed.filter_map(Value::as_dictionary).find(|p| string(p, "Guid") == Some(guid))
        {
            found = Some(profile.clone());
        }
    }
    let profile = found?;
    let parent = string(&profile, "Dynamic Profile Parent Name")
        .and_then(|name| bookmarks.iter().find(|p| string(p, "Name") == Some(name)));
    Some(match parent {
        Some(parent) => {
            let mut merged = (*parent).clone();
            for (key, value) in &profile {
                merged.insert(key.clone(), value.clone());
            }
            merged
        }
        None => profile,
    })
}

fn from_json(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Bool(on) => Value::Boolean(*on),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(int) => Value::Integer(int.into()),
            None => Value::Real(n.as_f64().unwrap_or(0.0)),
        },
        serde_json::Value::String(text) => Value::String(text.clone()),
        serde_json::Value::Array(items) => Value::Array(items.iter().map(from_json).collect()),
        serde_json::Value::Object(map) => {
            Value::Dictionary(map.iter().map(|(key, value)| (key.clone(), from_json(value))).collect())
        }
        serde_json::Value::Null => Value::String(String::new()),
    }
}

/// What a profile (and the preferences it is in) sets.
pub(crate) fn propose(env: &Env, root: &Dictionary, profile: &Dictionary) -> Proposal {
    let mut proposal = Proposal::default();
    propose_font(&mut proposal, profile);
    propose_window(&mut proposal, profile);
    propose_behaviour(&mut proposal, env, profile);
    propose_themes(&mut proposal, profile);
    skip_unsupported(&mut proposal, root, profile);
    proposal.sorted()
}

fn propose_font(proposal: &mut Proposal, profile: &Dictionary) {
    // "<PostScript name> <size>": the size is what follows the last space.
    if let Some(font) = string(profile, "Normal Font") {
        let from = shown(profile, "Normal Font");
        let (name, size) = match font.rsplit_once(' ') {
            Some((name, size)) => (name, units::number(size)),
            None => (font, None),
        };
        proposal.propose(
            Setting::FontFamily,
            &from,
            SettingValue::Font(FontName::postscript_name(name.trim())),
        );
        if let Some(size) = size {
            proposal.propose(Setting::FontSize, &from, SettingValue::Number(units::font_size(size)));
        }
    }
    if let Some(spacing) = real(profile, "Vertical Spacing") {
        proposal.propose(
            Setting::LineHeight,
            shown(profile, "Vertical Spacing"),
            SettingValue::Number(units::line_height(spacing as f32)),
        );
    }
    if let Some(on) = boolean(profile, "ASCII Ligatures") {
        proposal.propose(Setting::Ligatures, shown(profile, "ASCII Ligatures"), SettingValue::Flag(on));
    }
}

fn propose_window(proposal: &mut Proposal, profile: &Dictionary) {
    // Transparency counts how clear the window is, the other way round from opacity.
    if let Some(clear) = real(profile, "Transparency") {
        proposal.propose(
            Setting::Opacity,
            shown(profile, "Transparency"),
            SettingValue::Number(units::opacity(1.0 - clear as f32)),
        );
    }
    if let Some(on) = boolean(profile, "Blur") {
        proposal.propose(Setting::Blur, shown(profile, "Blur"), SettingValue::Flag(on));
    }
    if boolean(profile, "Unlimited Scrollback") == Some(true) {
        proposal.propose(
            Setting::ScrollbackLines,
            shown(profile, "Unlimited Scrollback"),
            SettingValue::Count(units::unlimited_scrollback()),
        );
    } else if let Some(lines) = integer(profile, "Scrollback Lines") {
        proposal.propose(
            Setting::ScrollbackLines,
            shown(profile, "Scrollback Lines"),
            SettingValue::Count(units::scrollback(usize::try_from(lines).unwrap_or(0))),
        );
    }
}

fn propose_behaviour(proposal: &mut Proposal, env: &Env, profile: &Dictionary) {
    // Each Option key sends 0 itself, 1 Meta or 2 Esc+; the last two are what Wormhole calls Meta.
    let sends = |key| integer(profile, key);
    if sends("Option Key Sends").is_some() || sends("Right Option Key Sends").is_some() {
        let left = sends("Option Key Sends").unwrap_or(0) > 0;
        let right = sends("Right Option Key Sends").unwrap_or(0) > 0;
        let meta = match (left, right) {
            (true, true) => OptionAsMeta::Both,
            (true, false) => OptionAsMeta::Left,
            (false, true) => OptionAsMeta::Right,
            (false, false) => OptionAsMeta::Off,
        };
        let from =
            format!("{}, {}", shown(profile, "Option Key Sends"), shown(profile, "Right Option Key Sends"));
        proposal.propose(Setting::OptionAsMeta, from, SettingValue::Meta(meta));
    }
    if let Some(silent) = boolean(profile, "Silence Bell") {
        proposal.propose(Setting::BellSound, shown(profile, "Silence Bell"), SettingValue::Flag(!silent));
    }
    // Both of iTerm2's visual cues (the bell icon, the flash of the screen) are the one glow here.
    let cues: Vec<&str> =
        ["Visual Bell", "Flashing Bell"].into_iter().filter(|key| boolean(profile, key).is_some()).collect();
    if !cues.is_empty() {
        let on = cues.iter().any(|key| boolean(profile, key) == Some(true));
        let from: Vec<String> = cues.iter().map(|key| shown(profile, key)).collect();
        proposal.propose(Setting::BellVisual, from.join(", "), SettingValue::Flag(on));
    }

    // `Custom Command` says what `Command` is: "Yes" a command line, "Custom Shell" a shell's path.
    if let Some(kind) = string(profile, "Custom Command") {
        let from = format!("{}, {}", shown(profile, "Custom Command"), shown(profile, "Command"));
        match (kind, string(profile, "Command")) {
            // A command line is words; a custom shell is one path, spaces and all.
            ("Yes", Some(command)) => match shell::plain_words(command) {
                Some(words) => shell::propose(proposal, env, Source::ITerm2, &from, &words),
                None => proposal.skip(from, Reason::UnsupportedValue),
            },
            ("Custom Shell", Some(path)) => {
                shell::propose(proposal, env, Source::ITerm2, &from, &[path.to_string()]);
            }
            ("No", _) => {}
            _ => proposal.skip(shown(profile, "Custom Command"), Reason::UnsupportedValue),
        }
    }
}

/// The profile's colours under one set of key names: with `suffix` the light or dark variant
/// (falling back to the plain key), converted to sRGB.
fn colours(profile: &Dictionary, suffix: Option<&str>) -> Dictionary {
    let names =
        COLOR_KEYS.iter().map(|name| name.to_string()).chain((0..16).map(|i| format!("Ansi {i} Color")));
    let mut out = Dictionary::new();
    for name in names {
        let specific = suffix.and_then(|suffix| profile.get(&format!("{name} ({suffix})")));
        if let Some(Value::Dictionary(color)) = specific.or_else(|| profile.get(&name)) {
            out.insert(name, Value::Dictionary(srgb(color)));
        }
    }
    out
}

/// A colour in sRGB. iTerm2 stores P3 on newer Macs; anything else (calibrated, device) is read
/// as sRGB, as its own themes do.
fn srgb(color: &Dictionary) -> Dictionary {
    if string(color, "Color Space") != Some("P3") {
        return color.clone();
    }
    let channel = |key| real(color, key).unwrap_or(0.0);
    let decode = |c: f64| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    let encode = |c: f64| {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
    };
    let (r, g, b) = (
        decode(channel("Red Component")),
        decode(channel("Green Component")),
        decode(channel("Blue Component")),
    );
    // Display P3 to sRGB, both linear, through XYZ (D65).
    let linear = [
        1.224_940_2 * r - 0.224_940_4 * g,
        -0.042_056_9 * r + 1.042_057_1 * g,
        -0.019_637_6 * r - 0.078_636_1 * g + 1.098_273_5 * b,
    ];
    let mut out = Dictionary::new();
    for (key, value) in ["Red Component", "Green Component", "Blue Component"].iter().zip(linear) {
        out.insert((*key).to_string(), Value::Real(encode(value)));
    }
    out
}

fn propose_themes(proposal: &mut Proposal, profile: &Dictionary) {
    let label = match string(profile, "Name") {
        Some(name) if !name.trim().is_empty() => format!("{} {}", Source::ITerm2.brand(), name.trim()),
        _ => Source::ITerm2.brand().to_string(),
    };
    let from = "Background Color";
    let separate = boolean(profile, "Use Separate Colors for Light and Dark Mode") == Some(true);
    let variants: Vec<(Setting, Option<&str>, String, String)> = if separate {
        vec![
            (Setting::Theme, Some("Dark"), format!("{label} dark"), format!("{from} (Dark)")),
            (Setting::ThemeLight, Some("Light"), format!("{label} light"), format!("{from} (Light)")),
        ]
    } else {
        vec![(Setting::Theme, None, label, from.to_string())]
    };
    for (setting, suffix, label, shown_as) in variants {
        let colours = colours(profile, suffix);
        match iterm_builder(&label, &colours) {
            Ok(builder) => proposal.propose_palette(setting, shown_as, &label, builder),
            Err(_) => proposal.skip(shown_as, Reason::UnsupportedValue),
        }
    }
}

fn skip_unsupported(proposal: &mut Proposal, root: &Dictionary, profile: &Dictionary) {
    for (key, kind) in [
        ("Cursor Type", SkippedKind::CursorShape),
        ("Blinking Cursor", SkippedKind::CursorBlink),
        ("Prompt Before Closing 2", SkippedKind::ConfirmClose),
    ] {
        if profile.contains_key(key) {
            proposal.skip_kind(shown(profile, key), kind);
        }
    }
    if let Some(kind) = string(profile, "Custom Directory").filter(|kind| *kind != "No") {
        proposal.skip_kind(format!("Custom Directory = {kind}"), SkippedKind::WorkingDirectory);
    }
    for (key, kind) in [
        ("TerminalMargin", SkippedKind::Padding),
        ("TerminalVMargin", SkippedKind::Padding),
        ("CopySelection", SkippedKind::CopyOnSelect),
        ("PromptOnQuit", SkippedKind::ConfirmClose),
    ] {
        if root.contains_key(key) {
            proposal.skip_kind(shown(root, key), kind);
        }
    }
    if boolean(profile, "Use Non-ASCII Font") == Some(true) {
        proposal.skip_kind(shown(profile, "Non Ascii Font"), SkippedKind::FontVariant);
    }
    if real(profile, "Horizontal Spacing").is_some_and(|spacing| (spacing - 1.0).abs() > f64::EPSILON) {
        proposal.skip_kind(shown(profile, "Horizontal Spacing"), SkippedKind::CellWidth);
    }
    for (name, map) in
        [("Keyboard Map", profile.get("Keyboard Map")), ("GlobalKeyMap", root.get("GlobalKeyMap"))]
    {
        if let Some(Value::Dictionary(map)) = map {
            for key in map.keys() {
                proposal.skip_kind(format!("{name} {key}"), SkippedKind::Keybinding);
            }
        }
    }
    // Triggers run commands and send text when output matches; the initial text is typed into every
    // new session. Neither is run or taken.
    if profile.get("Triggers").and_then(Value::as_array).is_some_and(|triggers| !triggers.is_empty()) {
        proposal.skip("Triggers", Reason::Executes);
    }
    if string(profile, "Initial Text").is_some_and(|text| !text.is_empty()) {
        proposal.skip(
            format!("Initial Text = {}", profile.get("Initial Text").map(show).unwrap_or_default()),
            Reason::Executes,
        );
    }
}
