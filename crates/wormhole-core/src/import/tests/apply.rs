//! Taking a proposal: settings, a palette saved as a theme file, shell profiles, and what the app
//! needs to call in one settings update.

use super::Fixture;
use crate::config::{OptionAsMeta, Settings, ShellProfile};
use crate::import::{FontName, Proposed, Setting, SettingValue, apply, save_themes};
use crate::theme::{self, Theme};

fn proposed(setting: Setting, value: SettingValue) -> Proposed {
    Proposed { setting, from: String::new(), value, default_on: true }
}

fn theme(name: &str, background: u32) -> Theme {
    theme::parse_ghostty(
        name,
        &format!("background = #{background:06x}\nforeground = #ffffff\npalette = 1=#ff0000\n"),
    )
    .expect("theme")
}

#[test]
fn plain_settings_are_set() {
    let mut settings = Settings::default();
    let chosen = vec![
        proposed(
            Setting::FontFamily,
            SettingValue::Font(FontName { name: "SFMono-Regular".into(), postscript: true }),
        ),
        proposed(Setting::FontSize, SettingValue::Number(15.0)),
        proposed(Setting::LineHeight, SettingValue::Number(1.4)),
        proposed(Setting::Ligatures, SettingValue::Flag(false)),
        proposed(Setting::Opacity, SettingValue::Number(0.7)),
        proposed(Setting::Blur, SettingValue::Flag(false)),
        proposed(Setting::OptionAsMeta, SettingValue::Meta(OptionAsMeta::Right)),
        proposed(Setting::ScrollbackLines, SettingValue::Count(4000)),
        proposed(Setting::BellSound, SettingValue::Flag(false)),
        proposed(Setting::BellVisual, SettingValue::Flag(true)),
        proposed(Setting::DefaultShell, SettingValue::Shell("/bin/bash".into())),
    ];
    apply(&mut settings, &chosen, &Default::default());
    assert_eq!(settings.font_family, "SFMono");
    assert_eq!((settings.font_size, settings.line_height, settings.opacity), (15.0, 1.4, 0.7));
    assert!(!settings.font_ligatures && !settings.blur && !settings.bell_sound && settings.bell_visual);
    assert_eq!((settings.option_as_meta, settings.scrollback_lines), (OptionAsMeta::Right, 4000));
    assert_eq!(settings.default_shell, "/bin/bash");
}

#[test]
fn what_is_not_chosen_is_left_alone() {
    let mut settings = Settings::default();
    apply(&mut settings, &[], &Default::default());
    assert_eq!(settings, Settings::default());
}

#[test]
fn a_shell_profile_is_added_and_one_of_the_same_name_is_replaced() {
    let mut settings = Settings::default();
    settings.shells.push(ShellProfile { name: "fish (Ghostty)".into(), command: "old".into(), args: None });
    settings.shells.push(ShellProfile { name: "other".into(), command: "x".into(), args: None });
    let profile = ShellProfile {
        name: "fish (Ghostty)".into(),
        command: "/opt/homebrew/bin/fish".into(),
        args: Some(vec!["-l".into()]),
    };
    apply(
        &mut settings,
        &[proposed(Setting::ShellProfile, SettingValue::Profile(profile.clone()))],
        &Default::default(),
    );
    assert_eq!(settings.shells.len(), 2);
    assert!(settings.shells.contains(&profile));
    // A profile is offered, not made the default.
    assert_eq!(settings.default_shell, "");
}

#[test]
fn palettes_are_saved_as_theme_files_and_selected_by_path() {
    let fixture = Fixture::new();
    let themes = fixture.root().join("themes");
    let chosen = vec![
        proposed(Setting::Theme, SettingValue::Palette(theme("Ghostty Night", 0x101010))),
        proposed(Setting::ThemeLight, SettingValue::Palette(theme("Ghostty Day", 0xf0f0f0))),
    ];
    let saved = save_themes(&themes, &chosen).expect("saved");
    let (dark, light) = (saved.dark.clone().expect("dark"), saved.light.clone().expect("light"));
    assert!(dark.ends_with("Ghostty Night") && light.ends_with("Ghostty Day"));

    // The theme folder reads it back as a theme of that name, with the colours it had.
    let listed = crate::theme::registry::UserThemes::read(&themes);
    assert!(listed.themes.iter().any(|t| t.name == "Ghostty Night"));
    let read = theme::load_file(std::path::Path::new(&dark)).expect("palette file");
    assert_eq!((read.background, read.foreground, read.palette[1]), (0x101010, 0xffffff, 0xff0000));

    let mut settings = Settings::default();
    apply(&mut settings, &chosen, &saved);
    assert_eq!((settings.theme, settings.theme_light), (dark, light));
}

#[test]
fn the_same_palette_again_reuses_its_file_and_another_with_the_name_gets_a_new_one() {
    let fixture = Fixture::new();
    let themes = fixture.root().join("themes");
    let first = save_themes(
        &themes,
        &[proposed(Setting::Theme, SettingValue::Palette(theme("Same Name", 0x111111)))],
    )
    .expect("saved");
    let again = save_themes(
        &themes,
        &[proposed(Setting::Theme, SettingValue::Palette(theme("Same Name", 0x111111)))],
    )
    .expect("saved");
    assert_eq!(first, again);
    let other = save_themes(
        &themes,
        &[proposed(Setting::Theme, SettingValue::Palette(theme("Same Name", 0x222222)))],
    )
    .expect("saved");
    assert_ne!(first.dark, other.dark);
    // The first stays as it was.
    let kept = theme::load_file(std::path::Path::new(first.dark.as_deref().expect("path"))).expect("kept");
    assert_eq!(kept.background, 0x111111);
}

#[test]
fn a_theme_name_with_a_slash_stays_in_the_themes_folder() {
    let fixture = Fixture::new();
    let themes = fixture.root().join("themes");
    let saved = save_themes(
        &themes,
        &[proposed(Setting::Theme, SettingValue::Palette(theme("../../evil/name", 0x333333)))],
    )
    .expect("saved");
    let path = std::path::PathBuf::from(saved.dark.expect("path"));
    assert_eq!(path.parent(), Some(themes.as_path()));
}

#[test]
fn a_palette_is_saved_whatever_the_source_and_round_trips_through_the_theme_format() {
    let t = Theme {
        cursor_text: Some(0x010203),
        selection_foreground: Some(0x040506),
        ..theme("Round Trip", 0x0a0b0c)
    };
    let text = crate::import::apply::palette_text(&t);
    assert_eq!(theme::parse_ghostty("Round Trip", &text).expect("parses"), t);
}
