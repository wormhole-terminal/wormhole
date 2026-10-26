//! VS Code's terminal: flat dotted keys, pixel sizes, CSS font lists, colour overrides, profiles.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

const FULL: &str = r##"
{
  // editor
  "editor.fontFamily": "Menlo, monospace",
  "workbench.colorTheme": "Default Dark Modern",
  "workbench.colorCustomizations": {
    "terminal.background": "#1e1e2eff",
    "terminal.foreground": "#cdd6f4",
    "terminal.ansiRed": "#f38ba8",
    "terminal.ansiBrightWhite": "#ffffff",
    "terminalCursor.foreground": "#f5e0dc",
    "editor.background": "#000000"
  },
  "terminal.integrated.fontFamily": "'Fira Code', Menlo, monospace",
  "terminal.integrated.fontSize": 14,
  "terminal.integrated.lineHeight": 1.2,
  "terminal.integrated.fontLigatures.enabled": true,
  "terminal.integrated.scrollback": 25000,
  "terminal.integrated.macOptionIsMeta": true,
  "terminal.integrated.enableVisualBell": true,
  "accessibility.signals.terminalBell": { "sound": "off" },
  "terminal.integrated.defaultProfile.osx": "zsh login",
  "terminal.integrated.profiles.osx": {
    "zsh login": { "path": "zsh", "args": ["-l"] }
  },
  "terminal.integrated.cursorStyle": "line",
  "terminal.integrated.cursorBlinking": true,
  "terminal.integrated.cwd": "${workspaceFolder}",
  "terminal.integrated.copyOnSelection": true,
  "terminal.integrated.confirmOnExit": "always",
  "terminal.integrated.letterSpacing": 1,
  "terminal.integrated.env.osx": { "FOO": "bar" },
}
"##;

#[test]
fn a_full_settings_file() {
    let fixture = Fixture::new();
    fixture.home("Library/Application Support/Code/User/settings.json", FULL);
    let found = fixture.found(Source::VsCode);
    let p = &found.proposal;

    assert_eq!(font(p).0, "Fira Code");
    // CSS pixels are points on a Mac.
    assert_eq!(number(p, Setting::FontSize), 14.0);
    assert!((number(p, Setting::LineHeight) - 1.2).abs() < 1e-6);
    assert!(flag(p, Setting::Ligatures));
    assert_eq!(count(p, Setting::ScrollbackLines), 25_000);
    assert_eq!(meta(p), OptionAsMeta::Both);
    assert!(flag(p, Setting::BellVisual));
    assert!(!flag(p, Setting::BellSound));

    let theme = palette(p, Setting::Theme);
    assert_eq!(theme.name, "VS Code Default Dark Modern");
    assert_eq!(
        (theme.background, theme.foreground, theme.palette[1], theme.palette[15], theme.cursor),
        (0x1e1e2e, 0xcdd6f4, 0xf38ba8, 0xffffff, 0xf5e0dc)
    );
    // The colours of the theme underneath are in its extension, not here.
    assert!(skipped(p, "workbench.colorTheme", &Reason::BuiltInTheme("Default Dark Modern".into())));

    // `zsh` is a shell the system lists, but with `-l` it is a command: not ticked.
    let profile = p.get(Setting::ShellProfile).expect("profile");
    assert!(!profile.default_on);
    match &profile.value {
        SettingValue::Profile(shell) => assert_eq!(
            (shell.command.as_str(), shell.args.clone()),
            ("/bin/zsh", Some(vec!["-l".to_string()]))
        ),
        other => panic!("{other:?}"),
    }

    for (needle, kind) in [
        ("cursorStyle", SkippedKind::CursorShape),
        ("cursorBlinking", SkippedKind::CursorBlink),
        ("integrated.cwd", SkippedKind::WorkingDirectory),
        ("copyOnSelection", SkippedKind::CopyOnSelect),
        ("confirmOnExit", SkippedKind::ConfirmClose),
        ("letterSpacing", SkippedKind::CellWidth),
        ("env.osx.FOO", SkippedKind::Environment),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
}

#[test]
fn the_editors_font_is_the_terminals_and_the_line_height_is_only_a_multiplier() {
    let fixture = Fixture::new();
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r#"{ "editor.fontFamily": "monospace, 'JetBrains Mono'", "terminal.integrated.fontSize": 10, "terminal.integrated.lineHeight": 1.5 }"#,
    );
    let p = fixture.found(Source::VsCode).proposal;
    // The generic family is not a font.
    assert_eq!(font(&p).0, "JetBrains Mono");
    assert!((number(&p, Setting::LineHeight) - 1.5).abs() < 1e-6);

    // 15 is 15 times the font, not 15 pixels: it can only be Wormhole's tallest.
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r#"{ "terminal.integrated.fontSize": 10, "terminal.integrated.lineHeight": 15 }"#,
    );
    let p = fixture.found(Source::VsCode).proposal;
    assert_eq!(number(&p, Setting::LineHeight), 2.0);
}

#[test]
fn a_few_colour_overrides_are_not_a_palette_and_a_built_in_profile_is_a_shell() {
    let fixture = Fixture::new();
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r##"{ "workbench.colorCustomizations": { "terminal.ansiRed": "#ff0000" }, "terminal.integrated.defaultProfile.osx": "bash", "terminal.integrated.enableBell": true }"##,
    );
    let p = fixture.found(Source::VsCode).proposal;
    assert!(p.get(Setting::Theme).is_none());
    assert_eq!(
        p.get(Setting::DefaultShell).map(|s| &s.value),
        Some(&SettingValue::Shell("/bin/bash".into()))
    );
    assert!(flag(&p, Setting::BellSound));
}

#[test]
fn an_unknown_profile_and_variables_in_a_path_are_unsupported() {
    let fixture = Fixture::new();
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r#"{ "terminal.integrated.defaultProfile.osx": "PowerShell" }"#,
    );
    assert!(skipped(&fixture.found(Source::VsCode).proposal, "defaultProfile", &Reason::UnsupportedValue));
    fixture.home(
        "Library/Application Support/Code/User/settings.json",
        r#"{ "terminal.integrated.defaultProfile.osx": "x", "terminal.integrated.profiles.osx": { "x": { "path": "${env:HOME}/bin/fish" } } }"#,
    );
    assert!(skipped(&fixture.found(Source::VsCode).proposal, "defaultProfile", &Reason::UnsupportedValue));
}

#[test]
fn insiders_is_read_when_code_is_not_installed_and_empty_says_nothing() {
    let fixture = Fixture::new();
    fixture.home("Library/Application Support/Code - Insiders/User/settings.json", "{}");
    assert!(fixture.found(Source::VsCode).proposal.is_empty());
}
