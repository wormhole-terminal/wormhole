//! iTerm2: a binary property list built here, the default profile, the PostScript font, the
//! inverted Transparency, light and dark colours, a custom preferences folder and Dynamic Profiles.

use plist::{Dictionary, Value};

use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SettingValue, SkippedKind, Source};

pub(super) fn dict(entries: Vec<(&str, Value)>) -> Dictionary {
    entries.into_iter().map(|(key, value)| (key.to_string(), value)).collect()
}

fn color(red: f64, green: f64, blue: f64, space: &str) -> Value {
    Value::Dictionary(dict(vec![
        ("Red Component", Value::Real(red)),
        ("Green Component", Value::Real(green)),
        ("Blue Component", Value::Real(blue)),
        ("Alpha Component", Value::Real(1.0)),
        ("Color Space", Value::String(space.into())),
    ]))
}

fn string(text: &str) -> Value {
    Value::String(text.into())
}

fn profile(guid: &str, name: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("Name", string(name)),
        ("Guid", string(guid)),
        ("Normal Font", string("JetBrainsMono-Regular 14")),
        ("Vertical Spacing", Value::Real(1.1)),
        ("ASCII Ligatures", Value::Boolean(true)),
        ("Transparency", Value::Real(0.15)),
        ("Blur", Value::Boolean(true)),
        ("Option Key Sends", Value::Integer(2.into())),
        ("Right Option Key Sends", Value::Integer(0.into())),
        ("Scrollback Lines", Value::Integer(5000.into())),
        ("Silence Bell", Value::Boolean(true)),
        ("Visual Bell", Value::Boolean(true)),
        ("Custom Command", string("Yes")),
        ("Command", string("/opt/homebrew/bin/fish -l")),
        ("Cursor Type", Value::Integer(2.into())),
        ("Blinking Cursor", Value::Boolean(true)),
        ("Custom Directory", string("Yes")),
        ("Working Directory", string("/tmp")),
        ("Foreground Color", color(0.75, 0.79, 0.96, "sRGB")),
        ("Background Color", color(0.10, 0.11, 0.15, "sRGB")),
        ("Ansi 1 Color", color(0.97, 0.46, 0.56, "sRGB")),
        (
            "Keyboard Map",
            Value::Dictionary(dict(vec![(
                "0xf702-0x280000",
                Value::Dictionary(dict(vec![("Action", Value::Integer(10.into()))])),
            )])),
        ),
        (
            "Triggers",
            Value::Array(vec![Value::Dictionary(dict(vec![
                ("regex", string("error")),
                ("action", string("RunCommandTrigger")),
            ]))]),
        ),
        ("Initial Text", string("echo hi")),
    ]
}

fn write(fixture: &Fixture, root: Dictionary) {
    let mut bytes = Vec::new();
    Value::Dictionary(root).to_writer_binary(&mut bytes).expect("binary plist");
    assert!(bytes.starts_with(b"bplist00"));
    fixture.bytes(&fixture.env.home.join("Library/Preferences/com.googlecode.iterm2.plist"), &bytes);
}

fn root(profiles: Vec<Dictionary>, default: &str) -> Dictionary {
    dict(vec![
        ("New Bookmarks", Value::Array(profiles.into_iter().map(Value::Dictionary).collect())),
        ("Default Bookmark Guid", string(default)),
        ("TerminalMargin", Value::Integer(5.into())),
        ("CopySelection", Value::Boolean(true)),
    ])
}

#[test]
fn the_default_profile_of_a_binary_plist() {
    let fixture = Fixture::new();
    let mut other = profile("g2", "Other");
    other[2] = ("Normal Font", string("Monaco 9"));
    write(&fixture, root(vec![dict(other), dict(profile("g1", "Default"))], "g1"));
    let found = fixture.found(Source::ITerm2);
    let p = &found.proposal;

    // The name is a PostScript name until the system says which family it belongs to.
    assert_eq!(font(p), ("JetBrainsMono-Regular".to_string(), true));
    match &p.get(Setting::FontFamily).expect("font").value {
        SettingValue::Font(f) => assert_eq!(f.family(), "JetBrainsMono"),
        other => panic!("{other:?}"),
    }
    assert_eq!(number(p, Setting::FontSize), 14.0);
    assert!((number(p, Setting::LineHeight) - 1.1).abs() < 1e-6);
    assert!(flag(p, Setting::Ligatures));
    // Transparency is how clear, opacity how solid.
    assert!((number(p, Setting::Opacity) - 0.85).abs() < 1e-6);
    assert!(flag(p, Setting::Blur));
    // Esc+ on the left Option key only.
    assert_eq!(meta(p), OptionAsMeta::Left);
    assert_eq!(count(p, Setting::ScrollbackLines), 5000);
    // Silence Bell is the opposite of a sound.
    assert!(!flag(p, Setting::BellSound));
    assert!(flag(p, Setting::BellVisual));
    let theme = palette(p, Setting::Theme);
    assert_eq!(theme.name, "iTerm2 Default");
    assert_eq!((theme.background, theme.palette[1]), (0x1a1c26, 0xf7758f));
    assert!(p.get(Setting::ThemeLight).is_none());
    // A command with arguments is not ticked.
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);

    for (needle, kind) in [
        ("Cursor Type", SkippedKind::CursorShape),
        ("Blinking Cursor", SkippedKind::CursorBlink),
        ("Custom Directory", SkippedKind::WorkingDirectory),
        ("TerminalMargin", SkippedKind::Padding),
        ("CopySelection", SkippedKind::CopyOnSelect),
        ("Keyboard Map 0xf702", SkippedKind::Keybinding),
    ] {
        assert!(skipped(p, needle, &Reason::NoSuchSetting(kind)), "{needle}");
    }
    // Triggers and the text typed at start would run things.
    assert!(skipped(p, "Triggers", &Reason::Executes));
    assert!(skipped(p, "Initial Text", &Reason::Executes));
}

#[test]
fn a_sparse_profile_and_a_missing_default_guid() {
    let fixture = Fixture::new();
    write(&fixture, root(vec![dict(vec![("Name", string("Bare")), ("Guid", string("x"))])], "no-such-guid"));
    let found = fixture.found(Source::ITerm2);
    // The first profile stands in; it says nothing, and the global keys are still reported.
    assert!(found.proposal.values.is_empty());
    assert!(skipped(&found.proposal, "TerminalMargin", &Reason::NoSuchSetting(SkippedKind::Padding)));
}

#[test]
fn an_xml_plist_and_a_file_that_is_not_one() {
    let fixture = Fixture::new();
    let mut xml = Vec::new();
    Value::Dictionary(root(vec![dict(profile("g1", "Default"))], "g1")).to_writer_xml(&mut xml).expect("xml");
    fixture.bytes(&fixture.env.home.join("Library/Preferences/com.googlecode.iterm2.plist"), &xml);
    assert_eq!(number(&fixture.found(Source::ITerm2).proposal, Setting::FontSize), 14.0);

    fixture.bytes(
        &fixture.env.home.join("Library/Preferences/com.googlecode.iterm2.plist"),
        b"not a plist at all",
    );
    let found = fixture.found(Source::ITerm2);
    assert!(found.proposal.values.is_empty());
    assert!(found.proposal.skipped.iter().any(|s| s.reason == Reason::Unreadable));
}

#[test]
fn light_and_dark_colours_make_a_pair_and_p3_is_converted() {
    let fixture = Fixture::new();
    let mut entries = profile("g1", "Default");
    entries.retain(|(key, _)| !matches!(*key, "Foreground Color" | "Background Color" | "Ansi 1 Color"));
    entries.push(("Use Separate Colors for Light and Dark Mode", Value::Boolean(true)));
    entries.push(("Background Color (Dark)", color(0.0, 0.0, 0.0, "sRGB")));
    entries.push(("Foreground Color (Dark)", color(1.0, 1.0, 1.0, "sRGB")));
    entries.push(("Background Color (Light)", color(0.5, 0.5, 0.5, "P3")));
    entries.push(("Foreground Color (Light)", color(0.0, 0.0, 0.0, "P3")));
    write(&fixture, root(vec![dict(entries)], "g1"));
    let p = fixture.found(Source::ITerm2).proposal;
    assert_eq!(palette(&p, Setting::Theme).background, 0x000000);
    let light = palette(&p, Setting::ThemeLight);
    // Mid grey is the same grey in P3 and sRGB.
    assert!((i64::from(light.background) - 0x808080).abs() <= 0x010101, "{:x}", light.background);
}

#[test]
fn a_custom_preferences_folder_takes_over() {
    let fixture = Fixture::new();
    let folder = fixture.root().join("synced");
    write(
        &fixture,
        dict(vec![
            ("LoadPrefsFromCustomFolder", Value::Boolean(true)),
            ("PrefsCustomFolder", string(&folder.to_string_lossy())),
        ]),
    );
    let mut custom = Vec::new();
    Value::Dictionary(root(vec![dict(profile("g1", "Synced"))], "g1"))
        .to_writer_binary(&mut custom)
        .expect("plist");
    fixture.bytes(&folder.join("com.googlecode.iterm2.plist"), &custom);
    let found = fixture.found(Source::ITerm2);
    assert_eq!(number(&found.proposal, Setting::FontSize), 14.0);
    assert_eq!(found.files.len(), 2);
}

#[test]
fn a_dynamic_profile_can_be_the_default_and_inherit_from_a_parent() {
    let fixture = Fixture::new();
    write(&fixture, root(vec![dict(profile("g1", "Default"))], "dyn-1"));
    fixture.home(
        "Library/Application Support/iTerm2/DynamicProfiles/work.json",
        r#"{"Profiles": [{"Name": "Work", "Guid": "dyn-1", "Dynamic Profile Parent Name": "Default", "Normal Font": "Menlo-Regular 18"}]}"#,
    );
    let p = fixture.found(Source::ITerm2).proposal;
    assert_eq!(number(&p, Setting::FontSize), 18.0);
    // Everything else comes from the parent.
    assert_eq!(number(&p, Setting::Opacity), 0.85);
}

#[test]
fn command_types_other_than_a_command_are_not_taken() {
    let fixture = Fixture::new();
    let mut entries = profile("g1", "Default");
    entries.retain(|(key, _)| *key != "Custom Command");
    entries.push(("Custom Command", string("SSH")));
    write(&fixture, root(vec![dict(entries)], "g1"));
    let p = fixture.found(Source::ITerm2).proposal;
    assert!(p.get(Setting::ShellProfile).is_none() && p.get(Setting::DefaultShell).is_none());
    assert!(skipped(&p, "Custom Command", &Reason::UnsupportedValue));
}

#[test]
fn unlimited_scrollback_and_both_option_keys() {
    let fixture = Fixture::new();
    let mut entries = profile("g1", "Default");
    entries.push(("Unlimited Scrollback", Value::Boolean(true)));
    entries.retain(|(key, _)| *key != "Right Option Key Sends");
    entries.push(("Right Option Key Sends", Value::Integer(1.into())));
    write(&fixture, root(vec![dict(entries)], "g1"));
    let p = fixture.found(Source::ITerm2).proposal;
    assert_eq!(count(&p, Setting::ScrollbackLines), 200_000);
    assert_eq!(meta(&p), OptionAsMeta::Both);
}
