//! Zed's terminal: JSON with comments, the editor's fonts behind the terminal's, themes from files.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

const FULL: &str = r##"
// Zed settings
{
  "buffer_font_family": "Zed Plex Mono",
  "buffer_font_size": 15,
  "theme": { "mode": "system", "light": "Light One", "dark": "Night" },
  "theme_overrides": { "Night": { "terminal.background": "#000000ff" } },
  "terminal": {
    "font_family": "JetBrains Mono",
    "line_height": { "custom": 1.4 },
    "font_features": { "calt": false },
    "option_as_meta": true,
    "bell": "off",
    "max_scroll_history_lines": 30000,
    "shell": { "with_arguments": { "program": "/bin/zsh", "args": ["--login"] } },
    "cursor_shape": "bar",
    "blinking": "on",
    "working_directory": "always_home",
    "copy_on_select": true,
    "env": { "EDITOR": "zed" },
    "font_weight": 500,
  },
}
"##;

const THEMES: &str = r##"{
  "name": "Mine", "author": "me",
  "themes": [
    { "name": "Night", "appearance": "dark", "style": {
      "terminal.background": "#1a1b26ff", "terminal.foreground": "#c0caf5ff",
      "terminal.ansi.red": "#f7768e", "terminal.ansi.bright_white": "#ffffff" } },
    { "name": "Light One", "appearance": "light", "style": {
      "terminal.background": "#fafafa", "terminal.foreground": "#383a42" } }
  ]
}"##;

#[test]
fn a_full_settings_file_and_its_theme_files() {
    let fixture = Fixture::new();
    fixture.home(".config/zed/settings.json", FULL);
    fixture.home(".config/zed/themes/mine.json", THEMES);
    let found = fixture.found(Source::Zed);
    let p = &found.proposal;

    // The terminal's own font wins over the editor's; the editor's size is behind it.
    assert_eq!(font(p).0, "JetBrains Mono");
    assert_eq!(number(p, Setting::FontSize), 15.0);
    assert!((number(p, Setting::LineHeight) - 1.4).abs() < 1e-6);
    assert!(!flag(p, Setting::Ligatures));
    assert_eq!(meta(p), OptionAsMeta::Both);
    assert!(!flag(p, Setting::BellSound));
    assert_eq!(count(p, Setting::ScrollbackLines), 30_000);
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);

    let dark = palette(p, Setting::Theme);
    assert_eq!(dark.name, "Zed Night");
    // The person's override is over the theme, and the alpha is dropped.
    assert_eq!(
        (dark.background, dark.foreground, dark.palette[1], dark.palette[15]),
        (0x000000, 0xc0caf5, 0xf7768e, 0xffffff)
    );
    assert_eq!(palette(p, Setting::ThemeLight).background, 0xfafafa);

    for (needle, kind) in [
        ("cursor_shape", SkippedKind::CursorShape),
        ("blinking", SkippedKind::CursorBlink),
        ("working_directory", SkippedKind::WorkingDirectory),
        ("copy_on_select", SkippedKind::CopyOnSelect),
        ("terminal.env.EDITOR", SkippedKind::Environment),
        ("font_weight", SkippedKind::FontVariant),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
}

#[test]
fn the_editor_font_is_the_terminals_when_it_has_none_and_zeds_own_themes_are_built_in() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/zed/settings.json",
        "{ \"buffer_font_family\": \"Fira Code\", \"buffer_font_size\": 12.5, \"theme\": \"One Dark\" }",
    );
    let p = fixture.found(Source::Zed).proposal;
    assert_eq!(font(&p).0, "Fira Code");
    assert_eq!(number(&p, Setting::FontSize), 12.5);
    assert!(skipped(&p, "theme", &Reason::BuiltInTheme("One Dark".into())));
}

#[test]
fn themes_of_installed_extensions_are_found_by_name() {
    let fixture = Fixture::new();
    fixture.home(".config/zed/settings.json", "{ \"theme\": \"Night\" }");
    fixture.home("Library/Application Support/Zed/extensions/installed/night/themes/night.json", THEMES);
    assert_eq!(palette(&fixture.found(Source::Zed).proposal, Setting::Theme).background, 0x1a1b26);
}

#[test]
fn standard_and_comfortable_line_heights_and_a_system_shell() {
    let fixture = Fixture::new();
    fixture.home(".config/zed/settings.json", "{ \"terminal\": { \"line_height\": \"standard\", \"shell\": { \"program\": \"/bin/bash\" }, \"bell\": \"system\" } }");
    let p = fixture.found(Source::Zed).proposal;
    assert!((number(&p, Setting::LineHeight) - 1.3).abs() < 1e-6);
    assert_eq!(
        p.get(Setting::DefaultShell).map(|s| &s.value),
        Some(&SettingValue::Shell("/bin/bash".into()))
    );
    assert!(flag(&p, Setting::BellSound));
}

#[test]
fn empty_and_broken() {
    let fixture = Fixture::new();
    fixture.home(".config/zed/settings.json", "{}");
    assert!(fixture.found(Source::Zed).proposal.is_empty());
    fixture.home(".config/zed/settings.json", "{ \"terminal\": ");
    assert_eq!(fixture.found(Source::Zed).proposal.skipped[0].reason, Reason::Unreadable);
}
