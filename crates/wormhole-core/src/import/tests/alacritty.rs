//! Alacritty: the first of its files, imports merged under the importing file, the old top-level
//! keys, and a bell command that is never run.

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

const FULL: &str = r##"
[general]
import = ["~/themes/night.toml"]
working_directory = "~/Projects"

[font]
size = 13.0
normal = { family = "JetBrains Mono", style = "Regular" }
offset = { x = 0, y = 2 }

[window]
opacity = 0.9
blur = true
option_as_alt = "OnlyLeft"
padding = { x = 8, y = 8 }

[scrolling]
history = 50000

[cursor]
style = { shape = "Beam", blinking = "On" }

[selection]
save_to_clipboard = true

[bell]
duration = 150
command = "afplay /System/Library/Sounds/Ping.aiff"

[terminal.shell]
program = "/bin/zsh"
args = ["-l"]

[env]
TERM = "xterm-256color"

[[keyboard.bindings]]
key = "T"
mods = "Command"
action = "CreateNewTab"
"##;

const NIGHT: &str = r##"
[colors.primary]
background = "#1a1b26"
foreground = "0xc0caf5"

[colors.normal]
black = "#15161e"
red = "#f7768e"

[colors.bright]
white = "#c0caf5"
"##;

#[test]
fn a_full_config_with_an_import() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", FULL);
    fixture.home("themes/night.toml", NIGHT);
    let found = fixture.found(Source::Alacritty);
    let p = &found.proposal;

    assert_eq!(font(p).0, "JetBrains Mono");
    assert_eq!(number(p, Setting::FontSize), 13.0);
    // Two pixels on a 13 point font whose line is about 1.2 times its size.
    assert!((number(p, Setting::LineHeight) - (1.0 + 2.0 / 15.6)).abs() < 1e-5);
    assert_eq!(number(p, Setting::Opacity), 0.9);
    assert!(flag(p, Setting::Blur));
    assert_eq!(meta(p), OptionAsMeta::Left);
    assert_eq!(count(p, Setting::ScrollbackLines), 50_000);
    assert!(flag(p, Setting::BellVisual));
    let theme = palette(p, Setting::Theme);
    assert_eq!(
        (theme.background, theme.foreground, theme.palette[1], theme.palette[15]),
        (0x1a1b26, 0xc0caf5, 0xf7768e, 0xc0caf5)
    );
    // `/bin/zsh -l` has an argument, so it is a command and not ticked.
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);
    assert!(found.files.iter().any(|f| f.ends_with("night.toml")));

    for (needle, kind) in [
        ("cursor.style", SkippedKind::CursorShape),
        ("window.padding", SkippedKind::Padding),
        ("save_to_clipboard", SkippedKind::CopyOnSelect),
        ("working_directory", SkippedKind::WorkingDirectory),
        ("keyboard.bindings", SkippedKind::Keybinding),
        ("env.TERM", SkippedKind::Environment),
        ("font.normal.style", SkippedKind::FontVariant),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
    // The bell command runs a program: shown, never taken, never run.
    assert!(skipped(p, "bell.command", &Reason::Executes));
}

#[test]
fn empty_and_sparse() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", "");
    assert!(fixture.found(Source::Alacritty).proposal.is_empty());
    fixture.home(".config/alacritty/alacritty.toml", "[font]\nsize = 9\n");
    let p = fixture.found(Source::Alacritty).proposal;
    assert_eq!(p.values.len(), 1);
    assert_eq!(number(&p, Setting::FontSize), 9.0);
}

#[test]
fn the_importing_file_wins_and_a_later_import_beats_an_earlier() {
    let fixture = Fixture::new();
    fixture
        .home(".config/alacritty/alacritty.toml", "import = [\"a.toml\", \"b.toml\"]\n[font]\nsize = 15\n");
    fixture.home(".config/alacritty/a.toml", "[font]\nsize = 10\n[window]\nopacity = 0.5\nblur = false\n");
    fixture.home(".config/alacritty/b.toml", "[window]\nopacity = 0.6\n");
    let p = fixture.found(Source::Alacritty).proposal;
    assert_eq!(number(&p, Setting::FontSize), 15.0);
    assert_eq!(number(&p, Setting::Opacity), 0.6);
    // Tables merge key by key.
    assert!(!flag(&p, Setting::Blur));
}

#[test]
fn an_import_cycle_and_a_depth_limit() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.toml", "import = [\"a.toml\"]\n[font]\nsize = 11\n");
    fixture.home(".config/alacritty/a.toml", "import = [\"alacritty.toml\"]\n");
    let found = fixture.found(Source::Alacritty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 11.0);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeCycle));

    // Alacritty's limit is five imports deep: a chain of seven is cut.
    let fixture = Fixture::new();
    for n in 0..7 {
        let next = if n < 6 { format!("import = [\"c{}.toml\"]\n", n + 1) } else { String::new() };
        let name = if n == 0 { "alacritty.toml".to_string() } else { format!("c{n}.toml") };
        fixture
            .home(&format!(".config/alacritty/{name}"), &format!("{next}[window]\nopacity = 0.{}\n", n + 3));
    }
    let found = fixture.found(Source::Alacritty);
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::IncludeLimit));
}

#[test]
fn a_missing_import_is_skipped_quietly_and_a_broken_file_is_noted() {
    let fixture = Fixture::new();
    fixture.home(
        ".config/alacritty/alacritty.toml",
        "import = [\"nope.toml\", \"bad.toml\"]\n[font]\nsize = 12\n",
    );
    fixture.home(".config/alacritty/bad.toml", "this is = = not toml");
    let found = fixture.found(Source::Alacritty);
    assert_eq!(number(&found.proposal, Setting::FontSize), 12.0);
    assert_eq!(found.proposal.skipped.len(), 1);
    assert_eq!(found.proposal.skipped[0].reason, Reason::Unreadable);
}

#[test]
fn the_older_names_still_read() {
    let fixture = Fixture::new();
    fixture.home(
        ".alacritty.toml",
        "shell = \"/bin/bash\"\nworking_directory = \"/tmp\"\n[[key_bindings]]\nkey = \"N\"\nchars = \"x\"\n",
    );
    let p = fixture.found(Source::Alacritty).proposal;
    // A shell the system lists, with no arguments, is the default shell.
    assert_eq!(
        p.get(Setting::DefaultShell).map(|s| &s.value),
        Some(&SettingValue::Shell("/bin/bash".into()))
    );
    assert!(skipped(&p, "key_bindings", &Reason::NoSuchSetting(SkippedKind::Keybinding)));
    assert!(skipped(&p, "working_directory", &Reason::NoSuchSetting(SkippedKind::WorkingDirectory)));
}

#[test]
fn the_yaml_file_is_not_read_but_noted() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty/alacritty.yml", "font:\n  size: 12\n");
    let found = fixture.found(Source::Alacritty);
    assert!(found.proposal.values.is_empty());
    assert_eq!(found.proposal.skipped[0].reason, Reason::OldFormat);
    assert_eq!(found.files.len(), 1);
    // The TOML file, when there is one, is the one used.
    fixture.home(".config/alacritty/alacritty.toml", "[font]\nsize = 9\n");
    assert_eq!(number(&fixture.found(Source::Alacritty).proposal, Setting::FontSize), 9.0);
}

#[test]
fn where_it_looks() {
    let fixture = Fixture::new();
    fixture.home(".config/alacritty.toml", "[font]\nsize = 8.5\n");
    // Only with XDG_CONFIG_HOME set is that name in the config folder looked at.
    assert!(crate::import::scan_one(&fixture.env, Source::Alacritty).is_none());
    let mut env = fixture.env.clone();
    env.xdg_config_home = Some(fixture.env.home.join(".config"));
    assert_eq!(
        number(
            &crate::import::scan_one(&env, Source::Alacritty).expect("alacritty").proposal,
            Setting::FontSize
        ),
        8.5
    );
    let mut env = fixture.env.clone();
    env.etc = Some(fixture.root().join("etc"));
    fixture.bytes(&fixture.root().join("etc/alacritty/alacritty.toml"), b"[font]\nsize = 9.5\n");
    assert_eq!(
        number(
            &crate::import::scan_one(&env, Source::Alacritty).expect("alacritty").proposal,
            Setting::FontSize
        ),
        9.5
    );
}

#[test]
fn the_top_level_import_is_read_before_general_import() {
    let fixture = Fixture::new();
    fixture.home("a.toml", "[font]\nsize = 15\n");
    fixture.home("b.toml", "[font]\nsize = 18\n");
    fixture.home(
        ".config/alacritty/alacritty.toml",
        "import = [\"~/a.toml\"]\n[general]\nimport = [\"~/b.toml\"]\n",
    );
    let p = fixture.found(Source::Alacritty).proposal;
    assert_eq!(number(&p, Setting::FontSize), 15.0);
}
