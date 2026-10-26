//! Taking a proposal: settings, a palette saved as a theme file, shell profiles, and what the app
//! needs to call in one settings update.

use super::Fixture;
use crate::config::{OptionAsMeta, Settings, ShellProfile};
use crate::import::{FontName, Proposal, Proposed, Reason, Setting, SettingValue, apply, save_themes};
use crate::theme::{self, Theme};

fn proposed(setting: Setting, value: SettingValue) -> Proposed {
    Proposed { setting, from: String::new(), value, default_on: true }
}

/// A proposal that offers `rows`.
fn proposal_of(rows: &[Proposed]) -> Proposal {
    Proposal { values: rows.to_vec(), skipped: Vec::new() }
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
            SettingValue::Font(FontName { name: "SF Mono".into(), postscript: false }),
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
    apply(&mut settings, &Proposal::default(), &chosen, &Default::default());
    assert_eq!(settings.font_family, "SF Mono");
    assert_eq!((settings.font_size, settings.line_height, settings.opacity), (15.0, 1.4, 0.7));
    assert!(!settings.font_ligatures && !settings.blur && !settings.bell_sound && settings.bell_visual);
    assert_eq!((settings.option_as_meta, settings.scrollback_lines), (OptionAsMeta::Right, 4000));
    assert_eq!(settings.default_shell, "/bin/bash");
}

#[test]
fn what_is_not_chosen_is_left_alone() {
    let mut settings = Settings::default();
    apply(&mut settings, &Proposal::default(), &[], &Default::default());
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
        &Proposal::default(),
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
    apply(&mut settings, &proposal_of(&chosen), &chosen, &saved);
    assert_eq!((settings.theme, settings.theme_light), (dark, light));
}

#[test]
fn unticking_the_light_palette_of_a_pair_leaves_light_mode_alone() {
    let fixture = Fixture::new();
    let themes = fixture.root().join("themes");
    let dark = proposed(Setting::Theme, SettingValue::Palette(theme("Night", 0x101010)));
    let light = proposed(Setting::ThemeLight, SettingValue::Palette(theme("Day", 0xf0f0f0)));
    // The source offers both; only the dark one is chosen.
    let offered = proposal_of(&[dark.clone(), light]);
    let chosen = vec![dark];
    let saved = save_themes(&themes, &chosen).expect("saved");
    let mut settings = Settings::default();
    let light_before = settings.theme_light.clone();
    apply(&mut settings, &offered, &chosen, &saved);
    assert_eq!(Some(settings.theme), saved.dark);
    assert_eq!(settings.theme_light, light_before);
}

#[test]
fn a_single_palette_is_set_for_light_and_dark() {
    let fixture = Fixture::new();
    let themes = fixture.root().join("themes");
    let chosen = vec![proposed(Setting::Theme, SettingValue::Palette(theme("Only One", 0x101010)))];
    let saved = save_themes(&themes, &chosen).expect("saved");
    let path = saved.dark.clone().expect("dark");
    let mut settings = Settings::default();
    apply(&mut settings, &proposal_of(&chosen), &chosen, &saved);
    assert_eq!((settings.theme, settings.theme_light), (path.clone(), path));
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

#[test]
fn a_long_profile_name_does_not_fail_the_save() {
    let themes = tempfile::tempdir().expect("temp dir");
    // Two bytes a character: whole, the name is longer than a file name may be.
    let chosen = vec![proposed(Setting::Theme, SettingValue::Palette(theme(&"é".repeat(300), 0x101010)))];
    let saved = save_themes(themes.path(), &chosen).expect("a name this long is cut, not refused");
    let path = std::path::PathBuf::from(saved.dark.expect("saved"));
    let file = path.file_name().expect("a file").to_string_lossy().into_owned();
    assert_eq!(file.chars().count(), 100, "cut on a character, never in the middle of one");
    assert!(path.is_file());
}

#[test]
fn a_postscript_name_is_never_applied_as_a_family() {
    let mut settings = Settings::default();
    let before = settings.font_family.clone();
    let chosen = vec![proposed(
        Setting::FontFamily,
        SettingValue::Font(FontName { name: "JetBrainsMono-Regular".into(), postscript: true }),
    )];
    apply(&mut settings, &Proposal::default(), &chosen, &Default::default());
    assert_eq!(settings.font_family, before);
}

#[test]
fn postscript_fonts_are_resolved_or_left_out() {
    let font = |name: &str| SettingValue::Font(FontName { name: name.into(), postscript: true });
    let mut proposal = Proposal::default();
    proposal.propose(Setting::FontFamily, "Normal Font = Gone-Regular 14", font("Gone-Regular"));
    proposal.propose(Setting::FontSize, "14", SettingValue::Number(14.0));
    let mut installed = proposal.clone();
    installed.values[0].value = font("JetBrainsMono-Regular");

    // Installed: the family the system names, not one cut from the name.
    installed.resolve_fonts(|name| (name == "JetBrainsMono-Regular").then(|| "JetBrains Mono".to_string()));
    assert_eq!(
        installed.get(Setting::FontFamily).map(|p| p.value.clone()),
        Some(SettingValue::Font(FontName { name: "JetBrains Mono".into(), postscript: false }))
    );
    assert!(installed.skipped.is_empty());

    // Not installed: not offered, and said why.
    proposal.resolve_fonts(|_| None);
    assert!(proposal.get(Setting::FontFamily).is_none());
    assert!(proposal.get(Setting::FontSize).is_some());
    assert_eq!(proposal.skipped.len(), 1);
    assert_eq!(proposal.skipped[0].key, "Normal Font = Gone-Regular 14");
    assert_eq!(proposal.skipped[0].reason, Reason::FontNotInstalled("Gone-Regular".into()));
}
