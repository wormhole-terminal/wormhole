//! Terminal.app: archived fonts and colours inside `<data>`, built here as `NSKeyedArchiver` plists,
//! the default profile, alpha as opacity, and sparse profiles.

use plist::{Dictionary, Uid, Value};

use super::iterm::dict;
use super::{Fixture, count, flag, font, meta, number, palette, skipped};
use crate::config::OptionAsMeta;
use crate::import::{Reason, Setting, SkippedKind, Source};

/// An `NSKeyedArchiver` archive holding `root` (an object that may point at the objects after it).
fn archive(root: Dictionary, others: Vec<Value>) -> Value {
    let mut objects = vec![Value::String("$null".into()), Value::Dictionary(root)];
    objects.extend(others);
    let top = dict(vec![("root", Value::Uid(Uid::new(1)))]);
    let archive = dict(vec![
        ("$archiver", Value::String("NSKeyedArchiver".into())),
        ("$objects", Value::Array(objects)),
        ("$top", Value::Dictionary(top)),
        ("$version", Value::Integer(100_000.into())),
    ]);
    let mut bytes = Vec::new();
    Value::Dictionary(archive).to_writer_binary(&mut bytes).expect("archive");
    Value::Data(bytes)
}

fn font_data(postscript: &str, size: f64) -> Value {
    archive(
        dict(vec![
            ("NSName", Value::Uid(Uid::new(2))),
            ("NSSize", Value::Real(size)),
            ("NSfFlags", Value::Integer(16.into())),
        ]),
        vec![Value::String(postscript.into())],
    )
}

/// A colour as Terminal stores it: floats as text in a NUL-ended byte string.
fn rgb_data(red: f64, green: f64, blue: f64, alpha: Option<f64>) -> Value {
    let mut text = format!("{red} {green} {blue}");
    if let Some(alpha) = alpha {
        text.push_str(&format!(" {alpha}"));
    }
    text.push('\0');
    archive(
        dict(vec![("NSColorSpace", Value::Integer(1.into())), ("NSRGB", Value::Data(text.into_bytes()))]),
        vec![],
    )
}

fn white_data(white: f64, alpha: f64) -> Value {
    let text = format!("{white} {alpha}\0");
    archive(
        dict(vec![("NSColorSpace", Value::Integer(3.into())), ("NSWhite", Value::Data(text.into_bytes()))]),
        vec![],
    )
}

fn write(fixture: &Fixture, default: &str, profiles: Vec<(&str, Dictionary)>) {
    let windows: Dictionary =
        profiles.into_iter().map(|(name, p)| (name.to_string(), Value::Dictionary(p))).collect();
    let root = dict(vec![
        ("Default Window Settings", Value::String(default.into())),
        ("Window Settings", Value::Dictionary(windows)),
    ]);
    let mut bytes = Vec::new();
    Value::Dictionary(root).to_writer_binary(&mut bytes).expect("plist");
    fixture.bytes(&fixture.env.home.join("Library/Preferences/com.apple.Terminal.plist"), &bytes);
}

#[test]
fn the_default_profile_with_archived_font_and_colours() {
    let fixture = Fixture::new();
    let pro = dict(vec![
        ("name", Value::String("Pro".into())),
        ("Font", font_data("SFMono-Regular", 12.0)),
        ("FontHeightSpacing", Value::Real(1.1)),
        ("BackgroundColor", white_data(0.0, 0.85)),
        ("TextColor", rgb_data(0.9, 0.9, 0.9, None)),
        ("ANSIRedColor", rgb_data(1.0, 0.0, 0.0, Some(1.0))),
        ("ANSIBrightWhiteColor", rgb_data(1.0, 1.0, 1.0, None)),
        ("BackgroundBlur", Value::Real(0.5)),
        ("useOptionAsMetaKey", Value::Boolean(true)),
        ("Bell", Value::Boolean(false)),
        ("CursorType", Value::Integer(2.into())),
        ("CursorBlink", Value::Boolean(true)),
        ("ShouldLimitScrollback", Value::Boolean(true)),
        ("ScrollbackLines", Value::Integer(8000.into())),
        ("CommandString", Value::String("/bin/bash".into())),
    ]);
    let other = dict(vec![("name", Value::String("Other".into())), ("Font", font_data("Monaco", 20.0))]);
    write(&fixture, "Pro", vec![("Other", other), ("Pro", pro)]);
    let found = fixture.found(Source::TerminalApp);
    let p = &found.proposal;

    assert_eq!(font(p), ("SFMono-Regular".to_string(), true));
    assert_eq!(number(p, Setting::FontSize), 12.0);
    assert!((number(p, Setting::LineHeight) - 1.1).abs() < 1e-6);
    let theme = palette(p, Setting::Theme);
    assert_eq!(theme.name, "Terminal Pro");
    assert_eq!(
        (theme.background, theme.foreground, theme.palette[1], theme.palette[15]),
        (0x000000, 0xe6e6e6, 0xff0000, 0xffffff)
    );
    // The window's opacity is the background colour's alpha.
    assert!((number(p, Setting::Opacity) - 0.85).abs() < 1e-6);
    assert!(flag(p, Setting::Blur));
    assert_eq!(meta(p), OptionAsMeta::Both);
    assert!(!flag(p, Setting::BellSound));
    assert_eq!(count(p, Setting::ScrollbackLines), 8000);
    // `/bin/bash` is a shell the system lists, so it is the default shell, ticked.
    assert!(p.get(Setting::DefaultShell).expect("shell").default_on);
    assert!(skipped(p, "CursorType", &Reason::NoSuchSetting(SkippedKind::CursorShape)));
    assert!(skipped(p, "CursorBlink", &Reason::NoSuchSetting(SkippedKind::CursorBlink)));
}

#[test]
fn a_sparse_profile_says_little() {
    let fixture = Fixture::new();
    let basic = dict(vec![
        ("name", Value::String("Basic".into())),
        ("Font", font_data("Monaco", 11.0)),
        ("type", Value::String("Window Settings".into())),
    ]);
    write(&fixture, "Basic", vec![("Basic", basic)]);
    let p = fixture.found(Source::TerminalApp).proposal;
    assert_eq!(font(&p).0, "Monaco");
    assert_eq!(number(&p, Setting::FontSize), 11.0);
    assert!(p.get(Setting::Theme).is_none() && p.get(Setting::Opacity).is_none());
}

#[test]
fn a_missing_default_falls_back_to_basic_and_an_unreadable_font_is_left_out() {
    let fixture = Fixture::new();
    let basic = dict(vec![
        ("Font", Value::Data(b"not an archive".to_vec())),
        ("ScrollbackLines", Value::Integer(50.into())),
        ("ShouldLimitScrollback", Value::Boolean(false)),
    ]);
    write(&fixture, "Gone", vec![("Basic", basic)]);
    let p = fixture.found(Source::TerminalApp).proposal;
    assert!(p.get(Setting::FontFamily).is_none());
    // Not limiting the scrollback means as much as Wormhole keeps.
    assert_eq!(count(&p, Setting::ScrollbackLines), 200_000);
}

#[test]
fn a_file_that_is_not_a_plist_is_noted() {
    let fixture = Fixture::new();
    fixture.bytes(&fixture.env.home.join("Library/Preferences/com.apple.Terminal.plist"), b"garbage");
    let found = fixture.found(Source::TerminalApp);
    assert_eq!(found.proposal.skipped[0].reason, Reason::Unreadable);
}
