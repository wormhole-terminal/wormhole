//! WezTerm: a Lua file read for what it writes out and never run.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SkippedKind, Source};

const ROSE_PINE: &str = r##"
[colors]
foreground = "#e0def4"
background = "#191724"
cursor_bg = "#524f67"
selection_bg = "#2a283e"
ansi = ["#26233a", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4"]
brights = ["#6e6a86", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4"]

[metadata]
name = "Rose Pine"
"##;

const FULL: &str = r##"
local wezterm = require 'wezterm'
local config = wezterm.config_builder()

-- Fonts
config.font = wezterm.font_with_fallback { 'JetBrains Mono', 'Menlo' }
config.font_size = 13.0
config.line_height = 1.2
config.harfbuzz_features = { 'calt=0', 'clig=0', 'liga=0' }

config.color_scheme = 'Rose Pine'
config.colors = {
  foreground = '#cccccc',
  cursor_bg = 'red',
}
config.window_background_opacity = 0.9
config.macos_window_background_blur = 20
config.send_composed_key_when_right_alt_is_pressed = false
config.default_prog = { '/opt/homebrew/bin/fish', '-l' }
config.scrollback_lines = 10000
config.audible_bell = 'Disabled'
config.visual_bell = { fade_in_duration_ms = 75, fade_out_duration_ms = 75 }
config.default_cursor_style = 'BlinkingBar'
config.window_padding = { left = 8, right = 8, top = 4, bottom = 4 }
config.default_cwd = '/tmp'
config.window_close_confirmation = 'NeverPrompt'
config.keys = {
  { key = 'c', mods = 'CMD', action = wezterm.action.CopyTo 'Clipboard' },
}
wezterm.on('format-tab-title', function(tab)
  config.font_size = 99
  return tab.title
end)

return config
"##;

fn wezterm(lua: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.home(".config/wezterm/wezterm.lua", lua);
    fixture
}

#[test]
fn a_full_config_is_read_for_what_it_writes_out() {
    let fixture = wezterm(FULL);
    fixture.home(".config/wezterm/colors/Rose Pine.toml", ROSE_PINE);
    let found = fixture.found(Source::WezTerm);
    let p = &found.proposal;

    // The first family of the fallback list.
    assert_eq!(font(p), ("JetBrains Mono".to_string(), false));
    // A `config.font_size = 99` inside a handler is a conditional one: it is not what the file sets.
    assert!(skipped(p, "font_size", &Reason::DynamicLua));
    assert!((number(p, Setting::LineHeight) - 1.2).abs() < 1e-6);
    assert!(!flag(p, Setting::Ligatures));
    assert_eq!(number(p, Setting::Opacity), 0.9);
    assert!(flag(p, Setting::Blur));
    // The left Option composes nothing by default, the right is set not to: both are Alt.
    assert_eq!(meta(p), OptionAsMeta::Both);
    assert_eq!(count(p, Setting::ScrollbackLines), 10_000);
    assert!(!flag(p, Setting::BellSound));
    assert!(flag(p, Setting::BellVisual));
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);

    let theme = palette(p, Setting::Theme);
    assert_eq!(theme.name, "WezTerm Rose Pine");
    // The scheme, with the config's `colors` over it; a colour by name is not taken.
    assert_eq!(
        (theme.background, theme.foreground, theme.palette[1], theme.palette[8]),
        (0x191724, 0xcccccc, 0xeb6f92, 0x6e6a86)
    );
    assert_eq!(theme.cursor, 0x524f67);
    assert!(skipped(p, "colors.cursor_bg", &Reason::UnsupportedValue));

    for (needle, kind) in [
        ("default_cursor_style", SkippedKind::CursorShape),
        ("window_padding", SkippedKind::Padding),
        ("default_cwd", SkippedKind::WorkingDirectory),
        ("window_close_confirmation", SkippedKind::ConfirmClose),
        ("keys", SkippedKind::Keybinding),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
}

#[test]
fn nothing_is_executed_and_computed_values_are_dynamic_lua() {
    let marker = Fixture::new().root().join("ran");
    let lua = format!(
        "local wezterm = require 'wezterm'\nlocal config = {{}}\n\
         os.execute('touch {}')\n\
         local size = 14\n\
         config.font_size = size * 2\n\
         config.color_scheme = scheme_for(wezterm.gui.get_appearance())\n\
         config.window_background_opacity = wezterm.target_triple == 'x' and 0.5 or 1.0\n\
         config.line_height = 1.1 + 0.1\n\
         config.font = wezterm.font(font_name)\n\
         config.scrollback_lines = 5000\n\
         local mine = require('my_module')\n\
         return config\n",
        marker.display()
    );
    let fixture = wezterm(&lua);
    let p = fixture.found(Source::WezTerm).proposal;
    assert!(!marker.exists(), "the Lua ran");
    for key in ["font_size", "color_scheme", "window_background_opacity", "line_height", "font"] {
        assert!(skipped(&p, key, &Reason::DynamicLua), "{key}");
    }
    assert!(skipped(&p, "require my_module", &Reason::DynamicLua));
    // The one literal in the file is still read.
    assert_eq!(count(&p, Setting::ScrollbackLines), 5000);
    assert_eq!(p.values.len(), 1);
}

#[test]
fn comments_and_long_strings_are_not_code() {
    let fixture = wezterm(
        "local config = wezterm.config_builder()\n-- config.font_size = 98\n--[==[\nconfig.font_size = 97\n]==]\n\
         local text = [[ config.font_size = 96 ]]\nconfig.font_size = 12 -- config.font_size = 95\nreturn config\n",
    );
    assert_eq!(number(&fixture.found(Source::WezTerm).proposal, Setting::FontSize), 12.0);
}

#[test]
fn a_returned_table_and_nested_assignments() {
    let fixture = wezterm(
        "local c = wezterm.config_builder()\nc.colors = { foreground = '#111111' }\nc.colors.background = '#fefefe'\n\
         c.font = wezterm.font 'Fira Code'\nc.font_size = -1\nreturn { scrollback_lines = 123, line_height = 1.5 }\n",
    );
    let p = fixture.found(Source::WezTerm).proposal;
    assert_eq!(font(&p).0, "Fira Code");
    let theme = palette(&p, Setting::Theme);
    assert_eq!((theme.foreground, theme.background), (0x111111, 0xfefefe));
    assert_eq!(number(&p, Setting::FontSize), 8.0);
    assert_eq!(count(&p, Setting::ScrollbackLines), 123);
    assert_eq!(number(&p, Setting::LineHeight), 1.5);
}

#[test]
fn a_font_with_attributes_and_a_scheme_that_is_not_on_disk() {
    let fixture = wezterm(
        "local config = {}\nconfig.font = wezterm.font('Iosevka', { weight = 'Bold' })\nconfig.color_scheme = 'Dracula (Official)'\n\
         config.send_composed_key_when_left_alt_is_pressed = true\nreturn config\n",
    );
    let p = fixture.found(Source::WezTerm).proposal;
    assert_eq!(font(&p).0, "Iosevka");
    assert!(p.get(Setting::Theme).is_none());
    assert!(skipped(&p, "color_scheme", &Reason::BuiltInTheme("Dracula (Official)".into())));
    // Composing on the left, Meta on the right only if the right is set not to compose, and by default it composes.
    assert_eq!(meta(&p), OptionAsMeta::Off);
}

#[test]
fn empty_and_the_other_places_it_looks() {
    let fixture = wezterm("");
    assert!(fixture.found(Source::WezTerm).proposal.is_empty());
    let home_file = Fixture::new();
    home_file.home(".wezterm.lua", "return { font_size = 11 }");
    assert_eq!(number(&home_file.found(Source::WezTerm).proposal, Setting::FontSize), 11.0);

    let mut env = home_file.env.clone();
    env.wezterm_config_file = Some(home_file.root().join("elsewhere.lua"));
    home_file.bytes(&home_file.root().join("elsewhere.lua"), b"return { font_size = 12 }");
    let found = crate::import::scan_one(&env, Source::WezTerm).expect("wezterm");
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
}

#[test]
fn a_scheme_name_cannot_climb_out_of_the_colours_folder() {
    let fixture = wezterm("return { color_scheme = '../secret' }");
    fixture.home(".config/wezterm/secret.toml", ROSE_PINE);
    assert!(fixture.found(Source::WezTerm).proposal.get(Setting::Theme).is_none());
}
