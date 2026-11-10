use std::collections::HashSet;

use wormhole_core::keys::keymap::menus::{self, MenuState};
use wormhole_core::keys::keymap::{self};

use super::keys::*;
use super::menus::*;
use super::records::*;
use super::*;

fn spec(glyphs: &str) -> KeySpecRecord {
    keys::key_spec_from_glyphs(glyphs.into()).expect("a spec")
}

#[test]
fn every_action_has_a_unique_text_id_that_reads_back() {
    let ids: Vec<String> = keymap::entries().iter().map(|entry| id_string(entry.id)).collect();
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), ids.len(), "ids are unique");
    for entry in keymap::entries() {
        assert_eq!(id_from_string(&id_string(entry.id)), Some(entry.id), "{:?}", entry.id);
    }
    assert!(ids.contains(&"newTerminal".to_string()) && ids.contains(&"terminal.3".to_string()));
    assert!(ids.contains(&"group.9".to_string()));
    assert_eq!(id_from_string("nonsense"), None);
}

#[test]
fn entries_carry_what_the_table_says() {
    let entries = keymap_entries();
    assert_eq!(entries.len(), keymap::entries().len());
    let find = |id: &str| entries.iter().find(|e| e.id == id).expect(id);
    let new = find("newTerminal");
    assert_eq!(new.keys.iter().map(|k| key_spec_glyphs(k.clone())).collect::<Vec<_>>(), ["⌘T", "⇧⌘N"]);
    let zoom = find("zoomPane");
    assert_eq!(zoom.title_off.as_deref(), Some("Zoom Pane"));
    assert_eq!(find("help").stage, KeyStageRecord::System);
    assert_eq!(find("print").stage, KeyStageRecord::Reserved);
    assert_eq!(find("focusPaneLeft").scope, KeyScopeRecord::Terminal);
    assert_eq!(find("focusPaneLeft").name.as_deref(), Some("Select Pane on the Left"));
    assert_eq!(find("newTerminal").name, None, "a row without a name of its own says none");
    assert!(find("inspector").debug_only && find("quitAndEndAll").persistent_only);
    assert_eq!(find("insertFolderPath").flag.as_deref(), Some("folder_picker"));
}

#[test]
fn the_menus_are_cores_menus_for_the_same_state() {
    let state = MenuStateRecord {
        off: vec!["toggleStatusBar".into(), "fullScreen".into(), "zoomPane".into()],
        checked: vec!["showChanges".into()],
        terminals: vec!["sidebar polish".into(), "diff perf".into(), "zsh".into()],
        current_terminal: Some(1),
        profiles: vec!["Default".into(), "Work".into()],
        current_profile: Some(0),
        window_profile: Some(0),
        debug: true,
        ..MenuStateRecord::default()
    };
    let core = menus::menus(&state.to_core());
    let records = keymap_menus(state.clone());
    assert_eq!(records.len(), core.len());
    assert_eq!(records.iter().map(|m| m.title.as_str()).collect::<Vec<_>>().last(), Some(&"Developer"));
    assert_eq!(keymap_menus_dump(state), menus::dump(&core));
    // A submenu keeps its items and a placeholder is disabled.
    let shell = records.iter().find(|m| m.title == "Shell").expect("Shell");
    let recent = shell.items.iter().find(|i| i.title == "Open Recent").expect("Open Recent");
    assert_eq!(recent.kind, MenuItemKindRecord::Submenu);
    assert_eq!(recent.items[0].kind, MenuItemKindRecord::Placeholder);
    assert!(!recent.items[0].enabled);
}

#[test]
fn a_selected_terminal_is_checked_and_a_split_closes_a_pane() {
    let state = MenuStateRecord {
        terminals: vec!["a".into(), "b".into()],
        current_terminal: Some(0),
        split: true,
        ..MenuStateRecord::default()
    };
    let bar = keymap_menus(state);
    let window = bar.iter().find(|m| m.title == "Window").expect("Window");
    let numbered: Vec<_> = window
        .items
        .iter()
        .filter(
            |i| matches!(&i.command, Some(MenuCommandRecord::Action { id }) if id.starts_with("terminal.")),
        )
        .collect();
    assert_eq!(numbered.len(), 2);
    assert!(numbered[0].checked && !numbered[1].checked);
    assert_eq!(key_spec_glyphs(numbered[1].key.clone().expect("key")), "⌘2");
    let shell = bar.iter().find(|m| m.title == "Shell").expect("Shell");
    assert!(shell.items.iter().any(|i| i.title == "Close Pane"));
}

#[test]
fn an_unknown_action_in_the_state_is_dropped_not_a_crash() {
    let state =
        MenuStateRecord { hidden: vec!["noSuchAction".into(), "newNote".into()], ..Default::default() };
    assert_eq!(state.to_core().hidden.len(), 1);
}

#[test]
fn hidden_actions_follow_the_feature_and_flag_tags() {
    let off = keymap_hidden_actions(vec!["notes".into()], vec!["folder_picker".into()]);
    assert!(off.contains(&"insertFolderPath".to_string()));
    assert!(off.contains(&"showNotes".to_string()), "{off:?}");
    assert!(keymap_hidden_actions(vec![], vec![]).is_empty());
}

#[test]
fn glyphs_and_the_config_grammar_round_trip_for_every_key_of_the_table() {
    for entry in keymap_entries() {
        for key in entry.keys {
            let glyphs = key_spec_glyphs(key.clone());
            let again = keys::key_spec_from_glyphs(glyphs.clone()).unwrap_or_else(|| panic!("{glyphs}"));
            assert!(key_spec_same_key(again, key.clone()), "{glyphs}");
            assert_eq!(key_spec_parse(key_spec_text(key.clone())).expect("grammar"), key, "{glyphs}");
        }
    }
    assert!(key_spec_parse("cmd-".into()).is_err());
    assert!(matches!(key_spec_parse("hyper-x".into()), Err(KeySpecParseError::UnknownModifier { .. })));
    assert!(key_spec_same_key(spec("⌘}"), spec("⇧⌘]")), "a US keyboard's shifted spelling");
}

#[test]
fn who_has_a_key() {
    assert!(matches!(key_uses(spec("⌘T")), Some(KeyUseRecord::Action { id, .. }) if id == "newTerminal"));
    assert!(matches!(key_uses(spec("⌘Space")), Some(KeyUseRecord::System { what }) if what == "Spotlight"));
    assert!(matches!(key_uses(spec("⌥A")), Some(KeyUseRecord::Shell { .. })));
    assert_eq!(key_uses(spec("⌃⌥⇧⌘Z")), None);
    assert_eq!(keymap_system_keys().len(), 19);
}

#[test]
fn recording_a_shortcut_and_its_conflicts() {
    assert_eq!(record_shortcut(0, "escape".into()), RecordedRecord::Cancel);
    assert_eq!(record_shortcut(0, "backspace".into()), RecordedRecord::Clear);
    assert_eq!(record_shortcut(0, "a".into()), RecordedRecord::NeedsModifier);
    assert_eq!(record_shortcut(8 | 4, "r".into()), RecordedRecord::Shortcut { spec: spec("⇧⌘R") });
    assert_eq!(shortcut_conflict(spec("⌘T"), None).as_deref(), Some("⌘T is New Terminal"));
    assert_eq!(shortcut_conflict(spec("⌘T"), Some("newTerminal".into())), None, "its own key is free");
    assert_eq!(shortcut_conflict(spec("⌘Space"), None).as_deref(), Some("macOS uses ⌘Space for Spotlight"));
    assert_eq!(shortcut_conflict(spec("⌥Space"), None).as_deref(), Some("The shell uses ⌥Space"));
}

#[test]
fn keys_are_routed_by_scope() {
    assert_eq!(key_action_for(spec("⌘T"), KeyScopeRecord::Terminal).as_deref(), Some("newTerminal"));
    assert_eq!(key_terminal_action(spec("⌘C")).as_deref(), Some("copy"));
    assert!(key_terminal_yields(spec("⌘T")));
    assert!(!key_terminal_yields(spec("⌘C")));
    assert_eq!(key_studio_action(spec("⌘S")).as_deref(), Some("saveTheme"));
    assert!(key_list_action(spec("⌥⌘↑")).is_some());
}

#[test]
fn the_dump_of_the_bar_matches_the_state_it_was_given() {
    // Core's dump is the contract the Swift side compares line by line.
    let dump = menus::dump(&menus::menus(&MenuState::default()));
    assert!(dump.starts_with("Wormhole\n  About Wormhole"));
}

#[test]
fn the_palette_lists_what_the_window_can_do_and_the_sheet_reads_the_table() {
    use super::lists::{keymap_palette_actions, keymap_sheet, keymap_sheet_columns};
    let all = keymap_palette_actions(MenuStateRecord::default());
    assert!(all.actions.len() > 60, "{}", all.actions.len());
    let new = all.actions.iter().find(|a| a.id == "newTerminal").expect("New terminal");
    assert_eq!((new.label.as_str(), new.section.as_str()), ("New terminal", "Terminal"));
    assert_eq!(all.suggested.first().map(String::as_str), Some("newTerminal"));
    let greyed = keymap_palette_actions(MenuStateRecord {
        disabled: vec!["showChanges".into()],
        hidden: vec!["showNotes".into()],
        ..Default::default()
    });
    assert!(!greyed.actions.iter().any(|a| a.id == "showChanges" || a.id == "showNotes"));

    let sheet = keymap_sheet(vec![], String::new());
    assert_eq!(sheet.sections.len(), 6);
    assert_eq!(sheet.shown, sheet.total);
    let found = keymap_sheet(vec![], "⌘W".into());
    assert!(found.sections.iter().flat_map(|s| &s.rows).any(|r| r.label.starts_with("Close terminal")));
    let marked = keymap_sheet(vec![], "split".into());
    assert!(marked.sections.iter().flat_map(|s| &s.rows).any(|r| r.label_hit.is_some()));
    let hidden = keymap_sheet(vec!["newNote".into()], "new note".into());
    assert_eq!(hidden.shown, 0);
    assert_eq!(
        keymap_sheet_columns(vec![3, 3, 3], 3),
        [(0, 1), (1, 2), (2, 3)].map(|(start, end)| crate::folder_picker::ByteRangeRecord { start, end })
    );
}
