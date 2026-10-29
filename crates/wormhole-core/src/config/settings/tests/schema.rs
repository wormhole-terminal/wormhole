//! Defaults, the names the file uses and what older files still read as.

use super::super::{
    APP_SETTINGS, AgentAccess, ChangesScope, DockBadge, HotkeyAction, InFront, MenuBarIcon, NotifySound,
    OptionAsMeta, PROJECT_SETTINGS, PlainChanges, Settings, SharedChanges, SizeUnits, ThemeMode, TitleBorder,
    keys,
};
use crate::storage::jsonc;
use crate::theme::fonts;
use crate::view::layout::title_parts::TitlePart;
use crate::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoices, StatusMotion};
use crate::view::sidebar::{Density, SidebarNumbers, TerminalsIn};

#[test]
fn glass_needs_the_setting_the_mac_and_a_system_that_allows_it() {
    let off = Settings::default();
    assert!(!off.liquid_glass, "off by default: a beta is chosen");
    assert!(!off.glass_on(true, false));
    let on: Settings = jsonc::parse(r#"{ "liquid_glass": true }"#).expect("valid jsonc");
    assert!(on.glass_on(true, false));
    assert!(!on.glass_on(false, false), "before macOS 26");
    assert!(!on.glass_on(true, true), "Reduce Transparency or Increase Contrast");
}

#[test]
fn the_final_design_keys_have_their_defaults() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    // The primary's switch is worked out as the file is read (an older config has none).
    assert_eq!(unset.clone().sanitized(), Settings::default());
    assert!(!unset.status_bar, "the status bar is hidden until asked for");
    assert!(!unset.minimap, "so is the minimap");
    assert!(unset.bell_sound && unset.bell_visual);
    assert_eq!(unset.sidebar_numbers, SidebarNumbers::Terminal);
    assert!(unset.sidebar_cmd_hints);
    assert_eq!(unset.terminals_in, TerminalsIn::Sidebar);
    assert!(!unset.sidebar_notes, "the rows are drawn without the notes and tasks count");
    assert!(unset.sidebar_changes);
    assert_eq!(unset.theme_mode, ThemeMode::System);
    assert_eq!(unset.theme_light, "Wormhole Light");
    assert!(unset.notify_needs_you && unset.notify_agent_done && unset.notify_command_done);
    assert!(unset.notify_command_failed, "a failing command is told unless switched off");
    assert_eq!(unset.notify_command_after, 10);
    assert!(!unset.notify_bell && unset.notify_only_unseen && unset.notify_group_bursts);
    assert_eq!(unset.notify_in_front, InFront::None, "in front, the window itself says nothing");
    assert_eq!(unset.notify_sound, NotifySound::NeedsYou);
    assert_eq!(unset.dock_badge, DockBadge::NeedsYou);
    assert!(unset.dock_bounce && !unset.menu_bar_animate);
    assert_eq!(unset.menu_bar_icon, MenuBarIcon::StatusCount);
    assert_eq!(unset.hotkey_action, HotkeyAction::QuickPanel);
    assert_eq!(unset.jump_waiting_key, "alt-cmd-j");
    assert_eq!(unset.size_units, SizeUnits::Decimal, "sizes read like Finder's unless asked");
    assert_eq!(unset.title_border, TitleBorder::Scrolled, "the line shows once output is under the bar");
    assert!(unset.open_in.is_empty(), "Open in… picks an editor until one is chosen");
    assert_eq!(
        unset.agent_access,
        AgentAccess::Ask,
        "programs ask the first time, unless the user said otherwise"
    );
    assert!(!unset.control_write, "the command line can't type into terminals until asked");
    assert!(!unset.allow_screen_checksum, "programs can't read the screen back until asked");
    assert!(!unset.allow_window_ops, "programs can't move the window until asked");
    assert!(!unset.allow_title_report, "programs can't read the window title back until asked");
    assert!(unset.worktree_base.is_empty(), "worktrees go beside the repository until asked");
}

#[test]
fn group_colours_fill_the_card_evenly_until_fading_is_turned_on() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(!unset.fade_group_colors);
    let on: Settings = jsonc::parse(r#"{ "fade_group_colors": true }"#).expect("valid jsonc");
    assert!(on.fade_group_colors);
    assert!(keys().contains(&"fade_group_colors".to_string()));
    assert!(PROJECT_SETTINGS.contains(&"fade_group_colors") && !APP_SETTINGS.contains(&"fade_group_colors"));
}

#[test]
fn a_groups_folder_shows_under_its_name_until_turned_off() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(unset.show_group_folders);
    let off: Settings = jsonc::parse(r#"{ "show_group_folders": false }"#).expect("valid jsonc");
    assert!(!off.show_group_folders);
    assert!(keys().contains(&"show_group_folders".to_string()));
    assert!(
        PROJECT_SETTINGS.contains(&"show_group_folders") && !APP_SETTINGS.contains(&"show_group_folders")
    );
}

#[test]
fn icon_animations_play_until_turned_off_and_are_the_persons_own() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(unset.icon_animations);
    let off: Settings = jsonc::parse(r#"{ "icon_animations": false }"#).expect("valid jsonc");
    assert!(!off.icon_animations);
    assert!(APP_SETTINGS.contains(&"icon_animations") && !PROJECT_SETTINGS.contains(&"icon_animations"));
}

#[test]
fn column_dividers_are_shown_until_turned_off_and_the_persons_own() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(unset.column_dividers);
    let off: Settings = jsonc::parse(r#"{ "column_dividers": false }"#).expect("valid jsonc");
    assert!(!off.column_dividers);
    assert!(keys().contains(&"column_dividers".to_string()));
    assert!(APP_SETTINGS.contains(&"column_dividers") && !PROJECT_SETTINGS.contains(&"column_dividers"));
}

#[test]
fn the_title_border_reads_by_name_and_shows_by_rule() {
    let settings: Settings =
        jsonc::parse(r#"{ "title_border": "never", "open_in": "dev.zed.Zed" }"#).expect("valid jsonc");
    assert_eq!(settings.title_border, TitleBorder::Never);
    assert_eq!(settings.open_in, "dev.zed.Zed");
    for (border, under, shown) in [
        (TitleBorder::Always, false, true),
        (TitleBorder::Always, true, true),
        (TitleBorder::Scrolled, false, false),
        (TitleBorder::Scrolled, true, true),
        (TitleBorder::Never, true, false),
    ] {
        assert_eq!(border.shows(under), shown, "{border:?} with content under: {under}");
    }
    // The window's chrome and the programs a person opens: not a repository's to set.
    for key in ["title_border", "open_in"] {
        assert!(APP_SETTINGS.contains(&key) && !PROJECT_SETTINGS.contains(&key), "{key}");
    }
}

/// The title row shows everything until the person arranges it, and it is never a repository's to
/// arrange (the pull request runs `gh`).
#[test]
fn the_title_row_shows_every_part_by_default_and_is_the_persons_own() {
    let read = |text: &str| jsonc::parse::<Settings>(text).expect("valid jsonc").sanitized();
    assert!(read("{}").title_row().is_default());
    let mine = read(r#"{ "title_row": ["terminal", "space", "splits"] }"#);
    assert_eq!(mine.title_row().parts(), [TitlePart::Terminal, TitlePart::Space, TitlePart::Splits]);
    assert!(!mine.title_shows(TitlePart::PullRequest));
    assert!(APP_SETTINGS.contains(&"title_row") && !PROJECT_SETTINGS.contains(&"title_row"));
}

/// Files from before the row could be arranged have four switches for its extras: the parts they
/// turned off stay hidden, the old keys are never written again, and a list the file has wins.
#[test]
fn the_old_title_row_switches_become_the_list() {
    let read = |text: &str| jsonc::parse::<Settings>(text).expect("valid jsonc").sanitized();
    let old = read(r#"{ "title_ports": false, "title_pull_request": false }"#);
    assert_eq!(old.title_row().hidden().collect::<Vec<_>>(), [TitlePart::Ports, TitlePart::PullRequest]);
    assert!(old.title_shows(TitlePart::AgentGauge) && old.title_shows(TitlePart::BranchStatus));
    let json = serde_json::to_value(&old).expect("serializes");
    for gone in ["title_branch_status", "title_agent_gauge", "title_ports", "title_pull_request"] {
        assert!(json.get(gone).is_none(), "{gone} is not written back");
    }
    assert_eq!(json["title_row"].as_array().map(Vec::len), Some(TitlePart::ALL.len() - 2));
    let both = read(r#"{ "title_ports": false, "title_row": ["ports", "space"] }"#);
    assert!(both.title_shows(TitlePart::Ports), "the list wins over the old switch");
    // Changing the row of an old file writes the list, which then holds.
    let mut changed: Settings = jsonc::parse(r#"{ "title_agent_gauge": false }"#).expect("valid jsonc");
    changed.show_title_part(TitlePart::AgentGauge, true);
    assert!(changed.title_row.as_ref().is_some_and(|row| row.is_default()));
}

/// Files from before the three densities have one switch for one-line rows: it becomes Compact,
/// is never written again, and a density the file does say wins over it.
#[test]
fn the_old_compact_switch_becomes_a_density() {
    let read = |text: &str| jsonc::parse::<Settings>(text).expect("valid jsonc").sanitized();
    assert_eq!(read(r#"{ "compact_sidebar": true }"#).density, Density::Compact);
    assert_eq!(read(r#"{ "compact_sidebar": false }"#).density, Density::Regular);
    assert_eq!(read(r#"{ "compact_sidebar": true, "density": "dense" }"#).density, Density::Dense);
    assert_eq!(read("{}").density, Density::Regular);
    assert!(read("{}").show_groups_as_cards, "cards are on until asked");
    let json = serde_json::to_value(read(r#"{ "compact_sidebar": true }"#)).expect("serializes");
    assert!(json.get("compact_sidebar").is_none(), "the old key is not written back");
    assert_eq!(json["density"], "compact");
    assert!(PROJECT_SETTINGS.contains(&"density") && PROJECT_SETTINGS.contains(&"show_groups_as_cards"));
}

/// A config.json from before the final sidebar still reads: its grouping, order, tab layout and
/// pinned folders mean nothing now, and the rest applies as before.
#[test]
fn a_config_with_the_old_sidebar_keys_still_reads() {
    let text = r#"{
        "sidebar_group": "folder", "sidebar_order": "classic", "tab_layout": "stacks",
        "pinned_folders": ["/w/a"], "sidebar_numbers": "list", "font_size": 15.0
    }"#;
    let settings: Settings = jsonc::parse(text).expect("old keys are ignored");
    assert_eq!(settings.font_size, 15.0);
    assert_eq!(settings.sidebar_numbers, SidebarNumbers::List);
    let json = serde_json::to_value(&settings).expect("serializes");
    for gone in ["sidebar_group", "sidebar_order", "tab_layout", "pinned_folders"] {
        assert!(json.get(gone).is_none(), "{gone} is no setting any more");
    }
}

#[test]
fn the_final_design_keys_round_trip_by_their_documented_names() {
    let text = r#"{
        "status_bar": true, "bell_sound": false, "bell_visual": false,
        "sidebar_cmd_hints": false, "sidebar_numbers": "list",
        "theme_mode": "light", "theme_light": "GitHub Light",
        "notify_command_after": 30, "notify_in_front": "banner", "notify_sound": "all",
        "dock_badge": "needs_you_done", "menu_bar_icon": "status_count",
        "hotkey_action": "toggle_app", "jump_waiting_key": "ctrl-cmd-j"
    }"#;
    let settings: Settings = jsonc::parse(text).expect("valid jsonc");
    assert!(settings.status_bar && !settings.bell_sound && !settings.bell_visual);
    assert_eq!(settings.sidebar_numbers, SidebarNumbers::List);
    assert_eq!(settings.theme_mode, ThemeMode::Light);
    assert_eq!(settings.notify_in_front, InFront::Banner);
    assert_eq!(settings.notify_sound, NotifySound::All);
    assert_eq!(settings.dock_badge, DockBadge::NeedsYouDone);
    assert_eq!(settings.hotkey_action, HotkeyAction::ToggleApp);
    let json = serde_json::to_value(&settings).expect("serializes");
    for (key, value) in [
        ("sidebar_numbers", "list"),
        ("theme_mode", "light"),
        ("notify_in_front", "banner"),
        ("notify_sound", "all"),
        ("dock_badge", "needs_you_done"),
        ("menu_bar_icon", "status_count"),
        ("hotkey_action", "toggle_app"),
    ] {
        assert_eq!(json[key], value, "{key}");
    }
    assert_eq!(serde_json::from_value::<Settings>(json).expect("reads back"), settings);
}

/// Older builds offered an in-window toast while Wormhole is in front, and a config.json
/// written then may still say so. It must read (an unknown value would mark the whole file
/// unreadable and stop saving) and mean nothing in front, which is what replaced it.
#[test]
fn an_older_configs_toast_reads_as_nothing_in_front() {
    let text = r#"{ "notify_in_front": "toast", "font_size": 15.0 }"#;
    let settings: Settings = jsonc::parse(text).expect("the old value still reads");
    assert_eq!(settings.notify_in_front, InFront::None);
    assert_eq!(settings.font_size, 15.0, "the rest of the file applies as before");
    let json = serde_json::to_value(&settings).expect("serializes");
    assert_eq!(json["notify_in_front"], "none", "saved by its current name");
}

#[test]
fn the_terminal_font_size_stays_within_the_one_range() {
    let with = |size| Settings { font_size: size, code_font_size: size, ..Settings::default() }.sanitized();
    let (min, max) = (*fonts::TERMINAL_FONT_SIZES.start(), *fonts::TERMINAL_FONT_SIZES.end());
    assert_eq!((with(1.0).font_size, with(99.0).font_size), (min, max));
    assert_eq!((with(1.0).code_font_size, with(99.0).code_font_size), (min, max));
    assert_eq!(with(14.0).font_size, 14.0);
    assert_eq!(fonts::FontSlot::Terminal.size_range(), (min, max));
    assert_eq!(fonts::FontSlot::Code.size_range(), (min, max));
}

#[test]
fn a_number_that_is_not_one_becomes_the_default_not_a_nan_that_clamp_keeps() {
    let defaults = Settings::default();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let kept = Settings {
            font_size: bad,
            ui_font_size: bad,
            code_font_size: bad,
            line_height: bad,
            opacity: bad,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(kept.font_size, defaults.font_size, "{bad}");
        assert_eq!(kept.ui_font_size, defaults.ui_font_size, "{bad}");
        assert_eq!(kept.code_font_size, defaults.code_font_size, "{bad}");
        assert_eq!(kept.line_height, defaults.line_height, "{bad}");
        assert_eq!(kept.opacity, defaults.opacity, "{bad}");
    }
}

#[test]
fn a_command_notification_waits_between_5_seconds_and_5_minutes() {
    let with = |after| Settings { notify_command_after: after, ..Settings::default() }.sanitized();
    assert_eq!(with(0).notify_command_after, 5);
    assert_eq!(with(60).notify_command_after, 60);
    assert_eq!(with(3600).notify_command_after, 300);
}

#[test]
fn partial_json_uses_defaults() {
    let settings: Settings = jsonc::parse("{ // comment\n \"font_size\": 16.0, }").expect("valid jsonc");
    assert_eq!(settings.font_size, 16.0);
    assert_eq!(settings.theme, Settings::default().theme);
}

#[test]
fn a_chosen_terminal_font_survives_the_new_default() {
    let chosen: Settings = jsonc::parse(r#"{ "font_family": "Menlo" }"#).expect("valid jsonc");
    assert_eq!(chosen.sanitized().font_family, "Menlo");
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert_eq!(unset.font_family, fonts::bundled::FAMILY);
    let blank = Settings { font_family: " ".into(), ..Settings::default() };
    assert_eq!(blank.sanitized().font_family, fonts::bundled::FAMILY);
}

#[test]
fn ligatures_default_on_and_round_trip() {
    assert!(Settings::default().font_ligatures);
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(unset.font_ligatures);
    let off: Settings = jsonc::parse(r#"{ "font_ligatures": false }"#).expect("valid jsonc");
    assert!(!off.font_ligatures);
    let json =
        serde_json::to_value(Settings { font_ligatures: false, ..Settings::default() }).expect("serializes");
    assert_eq!(json["font_ligatures"], false);
}

#[test]
fn the_status_dot_follows_the_setting_and_keys_of_older_files_still_load() {
    // Keys of older files that mean nothing now: the agent's logo is always on its tile.
    let settings: Settings = jsonc::parse(
        r#"{ "sidebar_status_position": "right", "sidebar_branch": false, "sidebar_agent_icon": false,
            "sidebar_changes": false }"#,
    )
    .expect("valid jsonc");
    assert!(!settings.sidebar_changes, "the rest of the file is read");
    let json = serde_json::to_value(&settings).expect("serializes");
    for gone in ["sidebar_status_position", "sidebar_branch", "sidebar_agent_icon"] {
        assert!(json.get(gone).is_none(), "{gone} is not written back");
    }
}

#[test]
fn option_as_meta_reads_its_sides_and_the_old_switch() {
    let read = |json: &str| -> OptionAsMeta {
        let settings: Settings = jsonc::parse(&format!("{{ \"option_as_meta\": {json} }}")).expect("reads");
        settings.option_as_meta
    };
    assert_eq!(Settings::default().option_as_meta, OptionAsMeta::Off);
    assert_eq!(read("\"left\""), OptionAsMeta::Left);
    assert_eq!(read("\"right\""), OptionAsMeta::Right);
    assert_eq!(read("\"both\""), OptionAsMeta::Both);
    assert_eq!(read("\"off\""), OptionAsMeta::Off);
    // Every config.json written before the sides existed has the boolean.
    assert_eq!(read("true"), OptionAsMeta::Both);
    assert_eq!(read("false"), OptionAsMeta::Off);
    assert!(jsonc::parse::<Settings>(r#"{ "option_as_meta": "middle" }"#).is_err());
    let json = serde_json::to_value(Settings { option_as_meta: OptionAsMeta::Left, ..Settings::default() })
        .expect("serializes");
    assert_eq!(json["option_as_meta"], "left");
}

#[test]
fn option_as_meta_follows_the_side_that_is_held() {
    use OptionAsMeta::{Both, Left, Off, Right};
    // (left held, right held) → Meta?
    for (setting, left, right, both, unknown) in [
        (Off, false, false, false, false),
        (Left, true, false, true, true),
        (Right, false, true, true, true),
        (Both, true, true, true, true),
    ] {
        assert_eq!(setting.is_meta(true, false), left, "{setting:?} left");
        assert_eq!(setting.is_meta(false, true), right, "{setting:?} right");
        assert_eq!(setting.is_meta(true, true), both, "{setting:?} both held");
        assert_eq!(setting.is_meta(false, false), unknown, "{setting:?} side unknown");
    }
}

#[test]
fn shell_profiles_read_with_or_without_arguments() {
    let settings: Settings = jsonc::parse(
        r#"{
            "default_shell": "fish",
            "shells": [
                { "name": "fish", "command": "/opt/homebrew/bin/fish" },
                { "name": "Claude", "command": "claude", "args": [] },
                { "name": "Dev", "command": "npm", "args": ["run", "dev"] },
            ],
        }"#,
    )
    .expect("valid jsonc");
    assert_eq!(settings.default_shell, "fish");
    let args: Vec<Option<&[String]>> = settings.shells.iter().map(|p| p.args.as_deref()).collect();
    assert_eq!(args[0], None, "no args: a login shell");
    assert_eq!(args[1], Some(&[][..]), "[]: the program bare");
    assert_eq!(args[2].map(|a| a.len()), Some(2));
    // Missing args stay missing when written back, so the login-shell meaning is kept.
    let json = serde_json::to_value(&settings).expect("serializes");
    assert_eq!(json["shells"][0], serde_json::json!({ "name": "fish", "command": "/opt/homebrew/bin/fish" }));
    assert_eq!(json["shells"][1]["args"], serde_json::json!([]));
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert!(unset.shells.is_empty() && unset.default_shell.is_empty(), "the login shell by default");
    // An unknown default is kept as written: the resolver falls back and says so.
    let odd: Settings = jsonc::parse(r#"{ "default_shell": "nope" }"#).expect("valid jsonc");
    assert_eq!(odd.sanitized().default_shell, "nope");
}

#[test]
fn the_motion_object_reads_per_choice_and_is_the_persons_own() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert_eq!(unset.motion.insert, InsertMotion::FromSide);
    assert_eq!(unset.motion.fold, FoldMotion::Tuck);
    assert_eq!(unset.motion.status, StatusMotion::Classic);
    assert_eq!(unset.motion.column, ColumnMotion::Classic);
    let partly: Settings =
        jsonc::parse(r#"{ "motion": { "status": "badge-pop", "column": "content-after" } }"#)
            .expect("valid jsonc");
    assert_eq!(partly.motion.status, StatusMotion::BadgePop);
    assert_eq!(partly.motion.column, ColumnMotion::ContentAfter);
    assert_eq!(partly.motion.insert, InsertMotion::FromSide, "a key left out is its default");
    assert!(keys().contains(&"motion".to_string()));
    assert!(APP_SETTINGS.contains(&"motion") && !PROJECT_SETTINGS.contains(&"motion"));
    // A fresh file writes the object empty: nothing is chosen yet, so no default is frozen into it.
    let written = serde_json::to_value(&unset).expect("serializes");
    assert_eq!(written["motion"], serde_json::json!({}));
    let chosen = Settings { motion: partly.motion, ..Settings::default() };
    let written = serde_json::to_value(&chosen).expect("serializes");
    assert_eq!(written["motion"], serde_json::json!({ "status": "badge-pop", "column": "content-after" }));
}

#[test]
fn the_motion_in_force_is_the_persons_while_the_feature_is_on_and_classic_otherwise() {
    let chosen: Settings = jsonc::parse(r#"{ "motion": { "insert": "reveal" } }"#).expect("valid jsonc");
    assert_eq!(chosen.motion_in_force(), chosen.motion, "a development build has the feature on");
    let off: Settings =
        jsonc::parse(r#"{ "motion": { "insert": "reveal" }, "feature_flags": { "motion": false } }"#)
            .expect("valid jsonc");
    assert_eq!(off.motion_in_force(), MotionChoices::CLASSIC, "the choices wait for the feature");
    assert_eq!(off.motion.insert, InsertMotion::Reveal, "and are kept in the file");
    assert!(chosen.motion_feature_on(), "the fixed motions play with the feature");
    assert!(!off.motion_feature_on(), "and wait for it");
}

/// Which features the person does without is theirs: a repository's overlay never switches a tool.
#[test]
fn the_feature_switches_are_the_persons_own() {
    assert!(APP_SETTINGS.contains(&"features") && !PROJECT_SETTINGS.contains(&"features"));
    let json = serde_json::to_value(Settings::default()).expect("serializes");
    assert_eq!(json["features"], serde_json::json!({}));
}

#[test]
fn the_row_count_choices_default_to_the_terminals_own_files_and_read_by_their_words() {
    let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
    assert_eq!(unset.sidebar_changes_scope, ChangesScope::Terminal);
    assert_eq!(unset.sidebar_changes_plain, PlainChanges::Finishing);
    assert_eq!(unset.sidebar_changes_shared, SharedChanges::Each);
    assert!(unset.sidebar_changes_remember);

    let chosen: Settings = jsonc::parse(
        r#"{ "sidebar_changes_scope": "repository", "sidebar_changes_plain": "any",
            "sidebar_changes_shared": "none", "sidebar_changes_remember": false }"#,
    )
    .expect("valid jsonc");
    assert_eq!(chosen.sidebar_changes_scope, ChangesScope::Repository);
    assert_eq!(chosen.sidebar_changes_plain, PlainChanges::Any);
    assert_eq!(chosen.sidebar_changes_shared, SharedChanges::None);
    assert!(!chosen.sidebar_changes_remember);
    let json = serde_json::to_value(&chosen).expect("serializes");
    for (key, value) in [
        ("sidebar_changes_scope", "repository"),
        ("sidebar_changes_plain", "any"),
        ("sidebar_changes_shared", "none"),
    ] {
        assert_eq!(json[key], value, "{key}");
    }
    assert_eq!(serde_json::from_value::<Settings>(json).expect("reads back"), chosen);
}

/// A word this build doesn't know (a newer build's choice, a typo) reads as the default, and the
/// rest of the file is read: it is not a reason to call the whole file unreadable.
#[test]
fn a_row_count_choice_this_build_doesnt_know_reads_as_the_default() {
    let settings: Settings = jsonc::parse(
        r#"{ "sidebar_changes_scope": "branch", "sidebar_changes_plain": "sometimes",
            "sidebar_changes_shared": "most", "font_size": 15.0 }"#,
    )
    .expect("unknown words read");
    assert_eq!(settings.sidebar_changes_scope, ChangesScope::Terminal);
    assert_eq!(settings.sidebar_changes_plain, PlainChanges::Finishing);
    assert_eq!(settings.sidebar_changes_shared, SharedChanges::Each);
    assert_eq!(settings.font_size, 15.0, "the rest of the file is read");
}

/// The mode is spelled as the file spells it, and a word it does not know falls back to asking (a
/// misspelt `"open"` must never read as letting everybody in, nor as locking everybody out).
#[test]
fn the_agent_access_mode_reads_by_its_words() {
    for (word, mode) in [("off", AgentAccess::Off), ("ask", AgentAccess::Ask), ("open", AgentAccess::Open)] {
        let settings: Settings =
            serde_json::from_value(serde_json::json!({ "agent_access": word })).expect("reads");
        assert_eq!(settings.agent_access, mode, "{word}");
        assert_eq!(serde_json::to_value(mode).expect("writes"), serde_json::json!(word));
    }
}
