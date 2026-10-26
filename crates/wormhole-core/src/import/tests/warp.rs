//! Warp: `settings.toml` and a custom theme's YAML.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

const FULL: &str = r##"
[appearance.text]
font_name = "JetBrains Mono"
font_size = 14.0
line_height_ratio = 1.3
ligature_rendering_enabled = true

[appearance.themes]
theme = { custom = { name = "Mine", path = "~/.warp/themes/mine.yaml" } }

[appearance.cursor]
cursor_display_type = "block"
cursor_blink = "disabled"

[appearance.window]
override_opacity = 90
override_blur = 5

[terminal]
maximum_grid_size = 20000
copy_on_select = false
use_audible_bell = true

[terminal.input]
extra_meta_keys = { left_alt = true, right_alt = true }

[session]
new_session_shell_override = { executable = "/bin/zsh" }

[session.working_directory_config]
advanced_mode = false

[general]
should_confirm_close_session = false

[global_hotkey.show_hide]
keybinding = "cmd-shift-a"
"##;

const THEME: &str = "\
name: Mine
accent: '#ff00ff'   # a comment after a value
cursor: '#00ff00'
background: '#101010'
foreground: '#eeeeee'
details: darker
terminal_colors:
  normal:
    black: '#000000'
    red: '#ff0000'
  bright: { black: '#111111', white: '#ffffff' }
";

#[test]
fn a_full_settings_file_with_a_custom_theme() {
    let fixture = Fixture::new();
    fixture.home(".warp/settings.toml", FULL);
    fixture.home(".warp/themes/mine.yaml", THEME);
    let found = fixture.found(Source::Warp);
    let p = &found.proposal;

    assert_eq!(font(p), ("JetBrains Mono".to_string(), false));
    assert_eq!(number(p, Setting::FontSize), 14.0);
    assert!((number(p, Setting::LineHeight) - 1.3).abs() < 1e-6);
    assert!(flag(p, Setting::Ligatures));
    assert!((number(p, Setting::Opacity) - 0.9).abs() < 1e-6);
    assert!(flag(p, Setting::Blur));
    assert_eq!(meta(p), OptionAsMeta::Both);
    assert!(flag(p, Setting::BellSound));
    assert_eq!(p.get(Setting::DefaultShell).map(|s| &s.value), Some(&SettingValue::Shell("/bin/zsh".into())));
    // The docs do not say this is the scrollback, so it is offered and not ticked.
    assert_eq!(count(p, Setting::ScrollbackLines), 20_000);
    assert!(!p.get(Setting::ScrollbackLines).expect("scrollback").default_on);

    let theme = palette(p, Setting::Theme);
    assert_eq!(theme.name, "Warp Mine");
    assert_eq!((theme.background, theme.foreground, theme.cursor), (0x101010, 0xeeeeee, 0x00ff00));
    assert_eq!((theme.palette[1], theme.palette[8], theme.palette[15]), (0xff0000, 0x111111, 0xffffff));

    for (needle, kind) in [
        ("cursor_display_type", SkippedKind::CursorShape),
        ("cursor_blink", SkippedKind::CursorBlink),
        ("copy_on_select", SkippedKind::CopyOnSelect),
        ("should_confirm_close_session", SkippedKind::ConfirmClose),
        ("working_directory_config", SkippedKind::WorkingDirectory),
        ("global_hotkey", SkippedKind::Keybinding),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
}

#[test]
fn a_built_in_theme_is_not_in_any_file_and_a_custom_command_is_not_ticked() {
    let fixture = Fixture::new();
    fixture.home(
        ".warp/settings.toml",
        "[appearance.themes]\ntheme = \"dracula\"\n[session]\nnew_session_shell_override = { custom = \"tmux new -A\" }\n",
    );
    let p = fixture.found(Source::Warp).proposal;
    assert!(skipped(&p, "appearance.themes.theme", &Reason::BuiltInTheme("dracula".into())));
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);
}

#[test]
fn light_and_dark_system_themes() {
    let fixture = Fixture::new();
    fixture.home(
        ".warp/settings.toml",
        "[appearance.themes]\nsystem_theme = true\n[appearance.themes.selected_system_themes]\n\
         dark = { custom = { name = \"D\", path = \"d.yaml\" } }\nlight = { custom = { name = \"L\", path = \"l.yaml\" } }\n",
    );
    fixture.home(".warp/themes/d.yaml", "background: '#000000'\nforeground: '#ffffff'\n");
    fixture.home(".warp/themes/l.yaml", "background: '#ffffff'\nforeground: '#000000'\n");
    let p = fixture.found(Source::Warp).proposal;
    assert_eq!(palette(&p, Setting::Theme).background, 0x000000);
    assert_eq!(palette(&p, Setting::ThemeLight).background, 0xffffff);
}

#[test]
fn empty_broken_and_preview() {
    let fixture = Fixture::new();
    fixture.home(".warp/settings.toml", "");
    assert!(fixture.found(Source::Warp).proposal.is_empty());
    fixture.home(".warp/settings.toml", "= = nope");
    assert_eq!(fixture.found(Source::Warp).proposal.skipped[0].reason, Reason::Unreadable);

    let preview = Fixture::new();
    preview.home(".warp-preview/settings.toml", "[appearance.text]\nfont_size = 10\n");
    assert_eq!(number(&preview.found(Source::Warp).proposal, Setting::FontSize), 10.0);
}

#[test]
fn a_custom_theme_file_that_is_missing_is_unreadable() {
    let fixture = Fixture::new();
    fixture.home(
        ".warp/settings.toml",
        "[appearance.themes]\ntheme = { custom = { name = \"X\", path = \"gone.yaml\" } }\n",
    );
    assert!(skipped(&fixture.found(Source::Warp).proposal, "appearance.themes.theme", &Reason::Unreadable));
}
