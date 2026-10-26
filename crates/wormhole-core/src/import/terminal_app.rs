//! Terminal.app: `com.apple.Terminal.plist`, the profile `Default Window Settings` names. A profile
//! only holds what differs from Terminal's defaults, so a plain one says little.

use std::path::PathBuf;

use plist::{Dictionary, Value};

use super::archive::Archive;
use super::env::Env;
use super::plist_value::{boolean, integer, read_dictionary, real, shown, string};
use super::proposal::{FontName, Proposal, Reason, Setting, SettingValue, SkippedKind};
use super::read::Reader;
use super::shell;
use super::source::Source;
use super::units;
use crate::config::OptionAsMeta;
use crate::theme::Builder;

/// Read Terminal.app's preferences and say what its default profile sets.
pub(crate) fn import(env: &Env, files: &mut Reader, located: &[PathBuf]) -> Proposal {
    let Some(root) = located.first().and_then(|path| read_dictionary(files, path)) else {
        return Proposal::default();
    };
    let profiles = root.get("Window Settings").and_then(Value::as_dictionary);
    let named = ["Default Window Settings", "Startup Window Settings"]
        .iter()
        .find_map(|key| string(&root, key).filter(|name| profiles.is_some_and(|p| p.contains_key(name))))
        .unwrap_or("Basic");
    match profiles.and_then(|p| p.get(named)).and_then(Value::as_dictionary) {
        Some(profile) => propose(env, named, profile),
        None => Proposal::default(),
    }
}

/// The archive behind a profile key.
fn archived(profile: &Dictionary, key: &str) -> Option<Archive> {
    Archive::open(profile.get(key)?.as_data()?)
}

/// What a profile sets.
pub(crate) fn propose(env: &Env, name: &str, profile: &Dictionary) -> Proposal {
    let mut proposal = Proposal::default();

    if let Some((postscript, size)) = archived(profile, "Font").and_then(|archive| archive.font()) {
        let from = format!("Font = {postscript} {size}");
        proposal.propose(
            Setting::FontFamily,
            &from,
            SettingValue::Font(FontName::postscript_name(postscript)),
        );
        proposal.propose(Setting::FontSize, &from, SettingValue::Number(units::font_size(size)));
    }
    if let Some(spacing) = real(profile, "FontHeightSpacing") {
        proposal.propose(
            Setting::LineHeight,
            shown(profile, "FontHeightSpacing"),
            SettingValue::Number(units::line_height(spacing as f32)),
        );
    }
    propose_colors(&mut proposal, name, profile);
    if let Some(blur) = real(profile, "BackgroundBlur") {
        proposal.propose(Setting::Blur, shown(profile, "BackgroundBlur"), SettingValue::Flag(blur > 0.0));
    }

    if let Some(on) = boolean(profile, "useOptionAsMetaKey") {
        let meta = if on { OptionAsMeta::Both } else { OptionAsMeta::Off };
        proposal.propose(
            Setting::OptionAsMeta,
            shown(profile, "useOptionAsMetaKey"),
            SettingValue::Meta(meta),
        );
    }
    if let Some(on) = boolean(profile, "Bell") {
        proposal.propose(Setting::BellSound, shown(profile, "Bell"), SettingValue::Flag(on));
    }
    if let Some(on) = boolean(profile, "VisualBell") {
        proposal.propose(Setting::BellVisual, shown(profile, "VisualBell"), SettingValue::Flag(on));
    }
    // Scrollback is limited only while "limit number of lines" is on.
    if boolean(profile, "ShouldLimitScrollback") == Some(false) {
        proposal.propose(
            Setting::ScrollbackLines,
            shown(profile, "ShouldLimitScrollback"),
            SettingValue::Count(units::unlimited_scrollback()),
        );
    } else if let Some(lines) = integer(profile, "ScrollbackLines") {
        proposal.propose(
            Setting::ScrollbackLines,
            shown(profile, "ScrollbackLines"),
            SettingValue::Count(units::scrollback(usize::try_from(lines).unwrap_or(0))),
        );
    }
    if let Some(command) = string(profile, "CommandString").filter(|c| !c.trim().is_empty()) {
        let from = shown(profile, "CommandString");
        match shell::plain_words(command) {
            Some(words) => shell::propose(&mut proposal, env, Source::TerminalApp, &from, &words),
            None => proposal.skip(from, Reason::UnsupportedValue),
        }
    }

    skip_unsupported(&mut proposal, profile);
    proposal.sorted()
}

/// The palette, and the window's opacity, which Terminal keeps in the background colour's alpha.
fn propose_colors(proposal: &mut Proposal, name: &str, profile: &Dictionary) {
    let color = |key: &str| archived(profile, key).and_then(|archive| archive.color());
    let mut builder = Builder::default();
    let background = color("BackgroundColor");
    builder.background = background.map(|c| c.packed());
    builder.foreground = color("TextColor").map(|c| c.packed());
    builder.cursor = color("CursorColor").map(|c| c.packed());
    builder.selection_background = color("SelectionColor").map(|c| c.packed());
    // The bright ones are `ANSIBright…`.
    builder.fill_ansi(|bright, ansi| {
        let ansi = units::capitalized(ansi);
        let prefix = if bright { "ANSIBright" } else { "ANSI" };
        color(&format!("{prefix}{ansi}Color")).map(|c| c.packed())
    });
    let label = format!("{} {name}", Source::TerminalApp.brand());
    proposal.propose_palette(Setting::Theme, "BackgroundColor", &label, builder);
    // The opacity is the background colour's, whether the colours make a palette or not.
    if let Some(background) = background {
        proposal.propose(
            Setting::Opacity,
            format!("BackgroundColor alpha = {}", background.alpha),
            SettingValue::Number(units::opacity(background.alpha)),
        );
    }
}

fn skip_unsupported(proposal: &mut Proposal, profile: &Dictionary) {
    for (key, kind) in [
        ("CursorType", SkippedKind::CursorShape),
        ("CursorBlink", SkippedKind::CursorBlink),
        ("warnOnShellCloseAction", SkippedKind::ConfirmClose),
    ] {
        if profile.contains_key(key) {
            proposal.skip_kind(shown(profile, key), kind);
        }
    }
    if real(profile, "FontWidthSpacing").is_some_and(|spacing| (spacing - 1.0).abs() > 0.01) {
        proposal.skip_kind(shown(profile, "FontWidthSpacing"), SkippedKind::CellWidth);
    }
    if let Some(Value::Dictionary(keys)) = profile.get("keyMapBoundKeys") {
        for key in keys.keys() {
            proposal.skip_kind(format!("keyMapBoundKeys {key}"), SkippedKind::Keybinding);
        }
    }
}
