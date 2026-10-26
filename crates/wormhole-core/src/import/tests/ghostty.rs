//! Ghostty: its four files, includes at the end of a file, themes, and the unit conversions.

use super::{Fixture, count, flag, font, meta, number, palette, settings, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

const ROSE_PINE: &str = "palette = 0=#26233a\npalette = 1=#eb6f92\nbackground = #191724\nforeground = #e0def4\n\
cursor-color = #e0def4\nselection-background = #403d52\n";
const ROSE_PINE_DAWN: &str = "palette = 0=#f2e9e1\nbackground = #faf4ed\nforeground = #575279\n";

const FULL: &str = "\
# my config
font-family = \"JetBrains Mono\"
font-family = Menlo
font-size = 13.5
adjust-cell-height = 20%
font-feature = -calt
theme = light:Rose Pine Dawn,dark:Rose Pine
background-opacity = 0.9
background-blur = 20
macos-option-as-alt = left
scrollback-limit = 10000000
bell-features = no-audio,system,border
command = /bin/zsh
cursor-style = bar
cursor-style-blink = false
window-padding-x = 8,8
copy-on-select = clipboard
confirm-close-surface = false
working-directory = ~/Projects
keybind = super+d=new_split:right
keybind = ctrl+a>n=next_tab
initial-command = htop
font-thicken = true
";

fn ghostty() -> Fixture {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/themes/Rose Pine", ROSE_PINE);
    fixture.home(".config/ghostty/themes/Rose Pine Dawn", ROSE_PINE_DAWN);
    fixture
}

#[test]
fn a_full_config_becomes_a_proposal() {
    let fixture = ghostty();
    fixture.home("Library/Application Support/com.mitchellh.ghostty/config.ghostty", FULL);
    let found = fixture.found(Source::Ghostty);
    let p = &found.proposal;

    assert_eq!(font(p), ("JetBrains Mono".to_string(), false));
    assert_eq!(number(p, Setting::FontSize), 13.5);
    assert!((number(p, Setting::LineHeight) - 1.2).abs() < 1e-6);
    assert!(!flag(p, Setting::Ligatures));
    assert_eq!(number(p, Setting::Opacity), 0.9);
    assert!(flag(p, Setting::Blur));
    assert_eq!(meta(p), OptionAsMeta::Left);
    // Bytes are not lines: 10 MB at about 640 bytes a line.
    assert_eq!(count(p, Setting::ScrollbackLines), 15_625);
    assert!(flag(p, Setting::BellSound));
    assert!(flag(p, Setting::BellVisual));
    assert_eq!(p.get(Setting::DefaultShell).map(|s| s.default_on), Some(true));

    let dark = palette(p, Setting::Theme);
    assert_eq!(dark.name, "Ghostty Rose Pine");
    assert_eq!((dark.background, dark.palette[1]), (0x191724, 0xeb6f92));
    let light = palette(p, Setting::ThemeLight);
    assert_eq!((light.name.as_str(), light.background), ("Ghostty Rose Pine Dawn", 0xfaf4ed));

    for (needle, kind) in [
        ("cursor-style = bar", SkippedKind::CursorShape),
        ("cursor-style-blink", SkippedKind::CursorBlink),
        ("window-padding-x", SkippedKind::Padding),
        ("copy-on-select", SkippedKind::CopyOnSelect),
        ("confirm-close-surface", SkippedKind::ConfirmClose),
        ("working-directory", SkippedKind::WorkingDirectory),
        ("super+d", SkippedKind::Keybinding),
        ("ctrl+a>n", SkippedKind::Keybinding),
        ("font-thicken", SkippedKind::FontVariant),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
    // A command to run at start is shown and never taken.
    assert!(skipped(p, "initial-command", &Reason::Executes));
}

#[test]
fn an_empty_and_a_comment_only_config_say_nothing() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "");
    assert!(fixture.found(Source::Ghostty).proposal.is_empty());
    fixture.home(".config/ghostty/config.ghostty", "# nothing here\n\n   # still nothing\n");
    let found = fixture.found(Source::Ghostty);
    assert!(found.proposal.is_empty());
    assert_eq!(found.files.len(), 2);
}

#[test]
fn nothing_found_is_absent() {
    assert!(crate::import::scan_one(&Fixture::new().env, Source::Ghostty).is_none());
}

#[test]
fn files_merge_in_order_and_the_last_wins() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "font-size = 10\nfont-family = Menlo\n");
    fixture.home(".config/ghostty/config.ghostty", "font-size = 11\n");
    fixture.home("Library/Application Support/com.mitchellh.ghostty/config", "font-size = 12\n");
    fixture.home("Library/Application Support/com.mitchellh.ghostty/config.ghostty", "font-size = 14\n");
    let found = fixture.found(Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 14.0);
    assert_eq!(font(&found.proposal).0, "Menlo");
    assert_eq!(found.files.len(), 4);
}

#[test]
fn an_include_is_read_at_the_end_of_its_file_so_it_overrides() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "font-size = 10\nconfig-file = extra.conf\nfont-size = 11\n");
    fixture.home(".config/ghostty/extra.conf", "font-size = 15\nconfig-file = ?missing.conf\n");
    let found = fixture.found(Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 15.0);
    assert!(found.files.iter().any(|f| f.ends_with("extra.conf")));
    // An optional include that is not there is no problem.
    assert!(found.proposal.skipped.is_empty());
}

#[test]
fn an_include_cycle_is_noted_and_does_not_loop() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "font-size = 10\nconfig-file = a.conf\n");
    fixture.home(".config/ghostty/a.conf", "font-size = 12\nconfig-file = config\n");
    let found = fixture.found(Source::Ghostty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeCycle));
}

#[test]
fn an_empty_value_starts_font_family_over() {
    let fixture = Fixture::new();
    fixture
        .home(".config/ghostty/config", "font-family = A\nfont-family =\nfont-family = B\nfont-family = C\n");
    assert_eq!(font(&fixture.found(Source::Ghostty).proposal).0, "B");
}

#[test]
fn units_and_aliases() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/ghostty/config",
        "adjust-cell-height = 3\nscrollback-limit-lines = 5000\nbackground-blur = false\n\
         macos-option-as-alt = true\nbell-features = no-system,no-audio\nbackground-opacity = 0.1\n\
         font-feature = calt=1, ss01\n",
    );
    let p = fixture.found(Source::Ghostty).proposal;
    // 3 points on a 13 point font whose line is about 1.2 times its size.
    assert!((number(&p, Setting::LineHeight) - (1.0 + 3.0 / 15.6)).abs() < 1e-5);
    assert_eq!(count(&p, Setting::ScrollbackLines), 5000);
    assert!(!flag(&p, Setting::Blur));
    assert_eq!(meta(&p), OptionAsMeta::Both);
    assert!(!flag(&p, Setting::BellSound));
    // Wormhole's lowest opacity.
    assert_eq!(number(&p, Setting::Opacity), 0.3);
    assert!(flag(&p, Setting::Ligatures));
    // Values come in the order of the settings.
    let order = settings(&p);
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
}

#[test]
fn a_theme_that_is_not_on_disk_is_a_built_in_and_inline_colours_still_count() {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", "theme = Dracula\nbackground = #000000\n");
    let p = fixture.found(Source::Ghostty).proposal;
    assert!(p.get(Setting::Theme).is_none());
    assert!(skipped(&p, "theme", &Reason::BuiltInTheme("Dracula".into())));

    // The app's own themes are found in its bundle.
    fixture.bytes(
        &fixture.root().join("Applications/Ghostty.app/Contents/Resources/ghostty/themes/Dracula"),
        b"background = #282a36\nforeground = #f8f8f2\npalette = 1=#ff5555\n",
    );
    let p = fixture.found(Source::Ghostty).proposal;
    let theme = palette(&p, Setting::Theme);
    // The config's own colour lines are over the theme's.
    assert_eq!((theme.background, theme.foreground, theme.palette[1]), (0x000000, 0xf8f8f2, 0xff5555));
}

#[test]
fn colours_with_no_theme_make_a_palette_and_named_colours_are_unsupported() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/ghostty/config",
        "background = #101010\nforeground = #eeeeee\npalette = 4=#0000ff\ncursor-color = cell-foreground\n",
    );
    let p = fixture.found(Source::Ghostty).proposal;
    let theme = palette(&p, Setting::Theme);
    assert_eq!(theme.name, "Ghostty");
    assert_eq!(theme.palette[4], 0x0000ff);
    assert!(skipped(&p, "cursor-color", &Reason::UnsupportedValue));
}

#[test]
fn a_theme_name_cannot_climb_out_of_the_themes_folder() {
    let fixture = Fixture::new();
    fixture.home(".config/secret", "background = #123456\nforeground = #654321\n");
    fixture.home(".config/ghostty/config", "theme = ../secret\n");
    let p = fixture.found(Source::Ghostty).proposal;
    assert!(p.get(Setting::Theme).is_none());
}

#[test]
fn values_that_are_not_numbers_are_unsupported() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/ghostty/config",
        "font-size = big\nbackground-opacity = lots\nmacos-option-as-alt = maybe\n",
    );
    let p = fixture.found(Source::Ghostty).proposal;
    assert!(p.values.is_empty());
    assert_eq!(p.skipped.len(), 3);
    assert!(p.skipped.iter().all(|s| s.reason == Reason::UnsupportedValue));
}

#[test]
fn a_theme_pair_may_use_an_equals_sign_in_either_order() {
    let fixture = ghostty();
    fixture.home(".config/ghostty/config", "theme = dark=Rose Pine,light=Rose Pine Dawn\n");
    let p = fixture.found(Source::Ghostty).proposal;
    assert_eq!(palette(&p, Setting::Theme).background, 0x191724);
    assert_eq!(palette(&p, Setting::ThemeLight).background, 0xfaf4ed);
}

/// What `bell-features = <line>` proposes: the sound and the visual cue, `None` where it says nothing.
fn bell(line: &str) -> (Option<bool>, Option<bool>) {
    let fixture = Fixture::new();
    fixture.home(".config/ghostty/config", &format!("bell-features = {line}\n"));
    let p = fixture.found(Source::Ghostty).proposal;
    let get = |setting| {
        p.get(setting).map(|proposed| match proposed.value {
            SettingValue::Flag(on) => on,
            _ => panic!("not a flag"),
        })
    };
    (get(Setting::BellSound), get(Setting::BellVisual))
}

#[test]
fn bell_features_are_flags_with_no_prefixes() {
    // On macOS `system` is the alert sound: `no-audio` turns off only the custom file.
    assert_eq!(bell("no-audio,system,border"), (Some(true), Some(true)));
    assert_eq!(bell("system"), (Some(true), None));
    assert_eq!(bell("audio"), (Some(true), None));
    assert_eq!(bell("no-system,no-audio"), (Some(false), None));
    assert_eq!(bell("no-audio"), (Some(false), None));
    // The custom sound alone is a sound.
    assert_eq!(bell("no-system,audio"), (Some(true), None));
    // A later flag overrides an earlier one of the same name.
    assert_eq!(bell("audio,no-audio"), (Some(false), None));
    assert_eq!(bell("no-border,border"), (None, Some(true)));
    // Flags that are not about sound or the border say nothing about either.
    assert_eq!(bell("attention,title"), (None, None));
    assert_eq!(bell("no-border"), (None, Some(false)));
    // All of them at once.
    assert_eq!(bell("false"), (Some(false), Some(false)));
    assert_eq!(bell("true"), (Some(true), Some(true)));
}
