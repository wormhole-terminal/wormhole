//! A project's overlay, the primary colour's switch, folder colours and what is cleaned on the way in.

use std::path::Path;

use super::super::{APP_SETTINGS, PROJECT_SETTINGS, Settings, ShellProfile, keys};
use crate::project::ProjectColorChoice;
use crate::storage::jsonc;
use crate::theme::folder_color::FolderColor;
use crate::theme::tokens::Color;

#[test]
fn the_master_notification_switch_is_on_while_any_kind_is() {
    let mut s = Settings::default();
    s.change_notify_kind(|s| s.notify_needs_you = false);
    s.change_notify_kind(|s| s.notify_agent_done = false);
    assert!(s.notifications, "commands still notify");
    s.change_notify_kind(|s| s.notify_command_done = false);
    assert!(s.notifications, "a failing command still notifies");
    s.change_notify_kind(|s| s.notify_command_failed = false);
    assert!(!s.notifications, "all off = false (the bell is off by default)");
    assert!(!s.notify_kind_on(true), "with the master off nothing is on, whatever a kind says");
    s.change_notify_kind(|s| s.notify_bell = true);
    assert!(s.notifications && s.notify_kind_on(s.notify_bell));
    assert!(!s.notify_kind_on(false));
}

#[test]
fn every_setting_is_either_for_projects_or_app_wide() {
    let mut classified: Vec<&str> = PROJECT_SETTINGS.iter().chain(APP_SETTINGS).copied().collect();
    classified.sort();
    let keys = keys();
    assert_eq!(classified, keys.iter().map(String::as_str).collect::<Vec<_>>(), "classify new settings");
    let app_wide = [
        "hotkey",
        "menu_bar",
        "auto_update",
        "anonymous_count",
        "update_channel",
        "notifications",
        "settings_translucent",
        "studio_translucent",
        // One switch for every window, so View → Show Status Bar says the same everywhere.
        "status_bar",
        "minimap",
        "sidebar_numbers",
        "dock_badge",
        "hotkey_action",
        "jump_waiting_key",
    ];
    for app_wide in app_wide {
        assert!(!PROJECT_SETTINGS.contains(&app_wide), "{app_wide}");
    }
    for key in keys.iter().filter(|k| k.starts_with("notify_") || k.starts_with("menu_bar")) {
        assert!(APP_SETTINGS.contains(&key.as_str()), "{key} is about the whole app");
    }
}

/// A project themes itself with `.wormhole/theme.json`, its one way to: the picks of the
/// user's own config.json are not a project's to change.
#[test]
fn a_project_themes_itself_with_its_theme_file_only() {
    for key in ["theme", "theme_mode", "theme_light"] {
        assert!(!PROJECT_SETTINGS.contains(&key), "{key}");
        assert!(APP_SETTINGS.contains(&key), "{key}");
    }
    assert!(PROJECT_SETTINGS.contains(&"primary_color"), "the primary stays a project's to set");
}

/// `control_write` lets scripts type into terminals and start programs, `control_read` lets them
/// read what terminals show: only the user's own config.json may turn either on, never a
/// repository's `.wormhole/config.json`.
#[test]
fn a_project_cannot_let_scripts_type_into_terminals() {
    assert!(APP_SETTINGS.contains(&"control_write") && !PROJECT_SETTINGS.contains(&"control_write"));
    assert!(APP_SETTINGS.contains(&"control_read") && !PROJECT_SETTINGS.contains(&"control_read"));
    assert!(
        APP_SETTINGS.contains(&"allow_screen_checksum")
            && !PROJECT_SETTINGS.contains(&"allow_screen_checksum")
    );
    assert!(APP_SETTINGS.contains(&"allow_window_ops") && !PROJECT_SETTINGS.contains(&"allow_window_ops"));
    assert!(
        APP_SETTINGS.contains(&"allow_title_report") && !PROJECT_SETTINGS.contains(&"allow_title_report")
    );
    let overrides = serde_json::json!({ "control_write": true, "control_read": true });
    let serde_json::Value::Object(map) = overrides else { unreachable!() };
    let (merged, applied) = Settings::default().with_overrides_reported(&map);
    // The merge itself is blind to the lists (the app applies only `PROJECT_SETTINGS`), so
    // what matters is the classification above; the overlay reads like any other key here.
    assert!(applied == ["control_read", "control_write"] && merged.control_write && merged.control_read);
}

/// A folder's colour is the person's own, for every window: a repository can't recolour the
/// folders in everyone's sidebar. It is the name that is stored, by an absolute path.
#[test]
fn folder_colours_are_the_apps_own_and_are_kept_by_name_and_absolute_path() {
    assert!(APP_SETTINGS.contains(&"folder_colors") && !PROJECT_SETTINGS.contains(&"folder_colors"));
    assert!(Settings::default().folder_colors.is_empty(), "no folder has a colour until asked");
    let read: Settings = jsonc::parse(
        r#"{ "folder_colors": { "/w/a": "teal", "/w/b/": "red", "~/c": "blue", "rel": "pink", "/w/d": "brown" } }"#,
    )
    .expect("valid jsonc");
    assert_eq!(read.folder_colors.len(), 5, "reading keeps what the file says");
    let kept = read.sanitized();
    assert_eq!(
        kept.folder_colors.keys().map(String::as_str).collect::<Vec<_>>(),
        ["/w/a", "/w/b", "/w/d"],
        "only absolute paths, a trailing slash dropped"
    );
    assert_eq!(kept.folder_color(Path::new("/w/a")), Some(FolderColor::Teal));
    assert_eq!(kept.folder_color(Path::new("/w/b")), Some(FolderColor::Red));
    assert_eq!(kept.folder_color(Path::new("/w/d")), None, "a name this build doesn't know colours nothing");
    assert!(kept.folder_colors.contains_key("/w/d"), "but stays for the build that does");
    assert_eq!(kept.folder_color(Path::new("/w/other")), None);
}

#[test]
fn a_picked_hex_colour_is_kept_for_the_folder_like_a_name() {
    let mut settings = Settings::default();
    let picked = ProjectColorChoice::parse("#2bb3a3").expect("a colour");
    assert!(settings.set_folder_color_choice(Path::new("/w/a"), Some(picked)));
    assert!(!settings.set_folder_color_choice(Path::new("/w/a"), Some(picked)), "nothing changed");
    assert_eq!(settings.folder_colors.get("/w/a").map(String::as_str), Some("#2bb3a3"));
    assert_eq!(settings.folder_color_choice(Path::new("/w/a")), Some(picked));
    assert_eq!(settings.folder_color(Path::new("/w/a")), None, "a hex colour is none of the eight");
}

#[test]
fn a_folder_is_coloured_and_uncoloured_by_its_path_however_it_is_spelt() {
    let mut settings = Settings::default();
    assert!(settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal)));
    assert!(!settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal)), "nothing changed");
    assert!(settings.set_folder_color(Path::new("/w/a/"), Some(FolderColor::Pink)), "one entry per folder");
    assert_eq!(settings.folder_colors.len(), 1);
    assert_eq!(settings.folder_color(Path::new("/w/a")), Some(FolderColor::Pink));
    assert_eq!(settings.folder_color(Path::new("/w/./a/")), Some(FolderColor::Pink));
    assert_eq!(settings.folder_color(Path::new("/w")), None, "a parent is another folder");
    assert!(!settings.set_folder_color(Path::new("/w/other"), None), "nothing to take away");
    assert!(settings.set_folder_color(Path::new("/w/a"), None));
    assert!(settings.folder_colors.is_empty());
}

#[test]
fn folder_colours_are_saved_in_the_file_as_names() {
    let mut settings = Settings::default();
    settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal));
    let text = serde_json::to_string(&settings).expect("serializes");
    assert!(text.contains(r#""folder_colors":{"/w/a":"teal"}"#), "{text}");
    let back: Settings = jsonc::parse(&text).expect("reads back");
    assert_eq!(back, settings);
}

#[test]
fn overrides_apply_and_bad_values_are_ignored() {
    let overrides = serde_json::json!({ "font_size": 18.0, "tab_width": "wide", "nope": 1 });
    let serde_json::Value::Object(map) = overrides else { unreachable!() };
    let merged = Settings::default().with_overrides(&map);
    assert_eq!(merged.font_size, 18.0);
    assert_eq!(merged.tab_width, Settings::default().tab_width);
}

#[test]
fn sanitizes_out_of_range_values() {
    let settings = Settings { font_size: 200.0, opacity: 0.0, tab_width: 0, ..Settings::default() };
    let settings = settings.sanitized();
    assert_eq!(settings.font_size, 36.0);
    assert_eq!(settings.opacity, 0.3);
    assert_eq!(settings.tab_width, 1);
}

#[test]
fn primary_color_is_a_valid_hex_or_nothing() {
    let with = |value: &str| Settings {
        primary_color: value.into(),
        theme_primary: Some(false),
        ..Settings::default()
    };
    assert_eq!(Settings::default().primary(), None);
    assert_eq!(with("#ff6600").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
    assert_eq!(with(" ff6600 ").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
    assert_eq!(with("#ff660080").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
    assert_eq!(with("orange").primary(), None);
    assert!(with("orange").primary_error().is_some_and(|e| e.starts_with("primary_color:")));
    assert!(with("").primary_error().is_none());
    assert!(with("#ff6600").primary_error().is_none());
}

#[test]
fn the_theme_primary_switch_keeps_the_color_chosen() {
    let mut s = Settings::default();
    assert!(s.uses_theme_primary());
    s.choose_primary("#ff6600".into());
    assert_eq!((s.theme_primary, s.primary().map(Color::to_hex).as_deref()), (Some(false), Some("#ff6600")));
    // Back to the theme's own: the color stays for when the switch goes off again.
    s.choose_primary(String::new());
    assert_eq!((s.primary(), s.primary_color.as_str()), (None, "#ff6600"));
    assert!(s.primary_error().is_none());
    s.theme_primary = Some(false);
    assert_eq!(s.primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
}

#[test]
fn a_config_from_before_the_switch_keeps_its_color() {
    let old: Settings = serde_json::from_str(r##"{ "primary_color": "#00aa88" }"##).expect("parses");
    assert_eq!(old.theme_primary, None);
    assert_eq!(old.primary().map(Color::to_hex).as_deref(), Some("#00aa88"));
    assert_eq!(old.clone().sanitized().theme_primary, Some(false), "and says so from now on");
    let plain: Settings = serde_json::from_str("{}").expect("parses");
    assert_eq!((plain.primary(), plain.sanitized().theme_primary), (None, Some(true)));
}

#[test]
fn projects_can_override_the_primary_color() {
    let serde_json::Value::Object(map) = serde_json::json!({ "primary_color": "#00aa88" }) else {
        unreachable!()
    };
    let merged = Settings::default().with_overrides(&map);
    assert_eq!(merged.primary().map(Color::to_hex).as_deref(), Some("#00aa88"));
    // Its own switch still wins.
    let serde_json::Value::Object(map) =
        serde_json::json!({ "primary_color": "#00aa88", "theme_primary": true })
    else {
        unreachable!()
    };
    assert_eq!(Settings::default().with_overrides(&map).primary(), None);
}

#[test]
fn overrides_report_the_keys_they_set() {
    let overrides = serde_json::json!({ "tab_width": 8, "font_size": "big", "nope": 1, "diff_wrap": true });
    let serde_json::Value::Object(map) = overrides else { unreachable!() };
    let (merged, applied) = Settings::default().with_overrides_reported(&map);
    assert_eq!(applied, ["diff_wrap", "tab_width"], "a mistyped or unknown key is not reported");
    assert_eq!(merged.tab_width, 8);
    assert!(merged.diff_wrap);
}

#[test]
fn unusable_and_duplicate_shell_profiles_are_dropped() {
    let profile =
        |name: &str, command: &str| ShellProfile { name: name.into(), command: command.into(), args: None };
    let settings = Settings {
        shells: vec![
            profile("fish", "/opt/homebrew/bin/fish"),
            profile("", "/bin/bash"),
            profile("nameless", " "),
            profile("fish", "/usr/local/bin/fish"),
            profile("nu", "nu"),
        ],
        ..Settings::default()
    }
    .sanitized();
    let kept: Vec<(&str, &str)> =
        settings.shells.iter().map(|p| (p.name.as_str(), p.command.as_str())).collect();
    assert_eq!(kept, [("fish", "/opt/homebrew/bin/fish"), ("nu", "nu")]);
}
