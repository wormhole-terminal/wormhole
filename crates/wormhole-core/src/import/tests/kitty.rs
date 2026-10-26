//! Kitty: `kitty.conf` where Kitty looks for it, includes in place, the themes kitten's files, and
//! `geninclude`, which is never followed.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SkippedKind, Source};

const FULL: &str = "\
# kitty.conf
font_family      JetBrains Mono
font_size        13.0
modify_font      cell_height 110%
disable_ligatures always
include current-theme.conf
background_opacity 0.85
background_blur 30
macos_option_as_alt left
scrollback_lines 20000
enable_audio_bell no
visual_bell_duration 0.5 ease-in
shell /opt/homebrew/bin/fish --login
cursor_shape beam
cursor_blink_interval 0
window_padding_width 8 12
copy_on_select clipboard
confirm_os_window_close 0
map kitty_mod+t new_tab
mouse_map left click ungrabbed mouse_handle_click
env FOO=bar
";

const THEME: &str = "\
## name: Tokyo Night
foreground #c0caf5
background #1a1b26
selection_foreground #c0caf5
selection_background #33467c
cursor #c0caf5
cursor_text_color #1a1b26
color0 #15161e
color1 #f7768e
color15 #c0caf5
";

#[test]
fn a_full_config() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", FULL);
    fixture.home(".config/kitty/current-theme.conf", THEME);
    let found = fixture.found(Source::Kitty);
    let p = &found.proposal;

    assert_eq!(font(p), ("JetBrains Mono".to_string(), false));
    assert_eq!(number(p, Setting::FontSize), 13.0);
    assert!((number(p, Setting::LineHeight) - 1.1).abs() < 1e-6);
    assert!(!flag(p, Setting::Ligatures));
    assert_eq!(number(p, Setting::Opacity), 0.85);
    assert!(flag(p, Setting::Blur));
    assert_eq!(meta(p), OptionAsMeta::Left);
    assert_eq!(count(p, Setting::ScrollbackLines), 20_000);
    assert!(!flag(p, Setting::BellSound));
    assert!(flag(p, Setting::BellVisual));

    let theme = palette(p, Setting::Theme);
    assert_eq!(
        (theme.background, theme.foreground, theme.palette[1], theme.palette[15]),
        (0x1a1b26, 0xc0caf5, 0xf7768e, 0xc0caf5)
    );
    assert_eq!(
        (theme.cursor, theme.cursor_text, theme.selection_background),
        (0xc0caf5, Some(0x1a1b26), 0x33467c)
    );

    // A shell with arguments is a command to run in every terminal, so it is not ticked.
    let shell = p.get(Setting::ShellProfile).expect("shell profile");
    assert!(!shell.default_on);

    for (needle, kind) in [
        ("cursor_shape", SkippedKind::CursorShape),
        ("cursor_blink_interval", SkippedKind::CursorBlink),
        ("window_padding_width", SkippedKind::Padding),
        ("copy_on_select", SkippedKind::CopyOnSelect),
        ("confirm_os_window_close", SkippedKind::ConfirmClose),
        ("map kitty_mod+t", SkippedKind::Keybinding),
        ("mouse_map", SkippedKind::Keybinding),
        ("env FOO", SkippedKind::Environment),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
}

#[test]
fn empty_and_comment_only() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "# nothing\n\n");
    assert!(fixture.found(Source::Kitty).proposal.is_empty());
}

#[test]
fn a_hash_after_a_value_is_part_of_the_value_and_continuation_lines_join() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/kitty/kitty.conf",
        "font_size 12 # note\nbackground #112233\nforeground #ff\n\\eeff\n",
    );
    let p = fixture.found(Source::Kitty).proposal;
    // `12 # note` is not a number.
    assert!(skipped(&p, "font_size", &Reason::UnsupportedValue));
    assert!(p.get(Setting::FontSize).is_none());
    // A palette needs a colour that parses; `#ffeeff` after the join does.
    assert_eq!(palette(&p, Setting::Theme).background, 0x112233);
    assert_eq!(palette(&p, Setting::Theme).foreground, 0xffeeff);
}

#[test]
fn includes_are_read_in_place_and_later_lines_win() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/kitty/kitty.conf",
        "font_size 10\ninclude sub/a.conf\nfont_size 11\nglobinclude extra/*.conf\n",
    );
    fixture.home(".config/kitty/sub/a.conf", "font_size 20\nbackground_opacity 0.5\n");
    fixture.home(".config/kitty/extra/1.conf", "background_opacity 0.6\n");
    fixture.home(".config/kitty/extra/2.conf", "background_opacity 0.7\n");
    fixture.home(".config/kitty/extra/skip.txt", "background_opacity 0.9\n");
    let found = fixture.found(Source::Kitty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 11.0);
    assert_eq!(number(&found.proposal, Setting::Opacity), 0.7);
    assert_eq!(found.files.len(), 4);
}

#[test]
fn an_include_cycle_is_noted() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "include a.conf\n");
    fixture.home(".config/kitty/a.conf", "include kitty.conf\nfont_size 9\n");
    let found = fixture.found(Source::Kitty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 9.0);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeCycle));
}

#[test]
fn geninclude_runs_a_program_and_is_never_followed() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "geninclude gen.py\nenvinclude KITTY_*\nfont_size 12\n");
    // Even if the file it would write is there, it is not read.
    fixture.home(".config/kitty/gen.py", "font_size 99\n");
    let found = fixture.found(Source::Kitty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
    assert!(skipped(&found.proposal, "geninclude gen.py", &Reason::Executes));
    assert!(skipped(&found.proposal, "envinclude", &Reason::UnsupportedValue));
    assert!(!found.files.iter().any(|f| f.ends_with("gen.py")));
}

#[test]
fn watcher_is_a_program_too() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "watcher my_watcher.py\n");
    assert!(skipped(&fixture.found(Source::Kitty).proposal, "watcher", &Reason::Executes));
}

#[test]
fn the_config_folder_is_the_first_that_has_a_file() {
    let fixture = Fixture::new();
    fixture.home("Library/Preferences/kitty/kitty.conf", "font_size 9\n");
    assert_eq!(number(&fixture.found(Source::Kitty).proposal, Setting::FontSize), 9.0);
    fixture.home(".config/kitty/kitty.conf", "font_size 10\n");
    assert_eq!(number(&fixture.found(Source::Kitty).proposal, Setting::FontSize), 10.0);

    let mut env = fixture.env.clone();
    env.xdg_config_home = Some(fixture.root().join("xdg"));
    fixture.bytes(&fixture.root().join("xdg/kitty/kitty.conf"), b"font_size 11\n");
    assert_eq!(
        number(&crate::import::scan_one(&env, Source::Kitty).expect("kitty").proposal, Setting::FontSize),
        11.0
    );
    env.kitty_config_directory = Some(fixture.root().join("direct"));
    fixture.bytes(&fixture.root().join("direct/kitty.conf"), b"font_size 12\n");
    assert_eq!(
        number(&crate::import::scan_one(&env, Source::Kitty).expect("kitty").proposal, Setting::FontSize),
        12.0
    );
}

#[test]
fn light_and_dark_theme_files_make_a_pair() {
    let fixture = Fixture::new();
    fixture.home(".config/kitty/kitty.conf", "font_size 12\n");
    fixture.home(".config/kitty/dark-theme.auto.conf", "background #000000\nforeground #ffffff\n");
    fixture.home(".config/kitty/light-theme.auto.conf", "background #ffffff\nforeground #000000\n");
    let p = fixture.found(Source::Kitty).proposal;
    assert_eq!(palette(&p, Setting::Theme).background, 0x000000);
    assert_eq!(palette(&p, Setting::ThemeLight).background, 0xffffff);
}

#[test]
fn units_and_aliases() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/kitty/kitty.conf",
        "modify_font cell_height 4px\nscrollback_lines -1\nmacos_option_as_alt yes\nbackground_blur 0\n\
         disable_ligatures cursor\nfont_family family=\"Fira Code\" style=Retina\nshell .\n",
    );
    let p = fixture.found(Source::Kitty).proposal;
    // 4 px on an 11 point font.
    assert!((number(&p, Setting::LineHeight) - (1.0 + 4.0 / 13.2)).abs() < 1e-5);
    assert_eq!(count(&p, Setting::ScrollbackLines), 200_000);
    assert_eq!(meta(&p), OptionAsMeta::Both);
    assert!(!flag(&p, Setting::Blur));
    assert!(flag(&p, Setting::Ligatures));
    assert_eq!(font(&p).0, "Fira Code");
    // `.` is the login shell.
    assert!(p.get(Setting::DefaultShell).is_none() && p.get(Setting::ShellProfile).is_none());
}
