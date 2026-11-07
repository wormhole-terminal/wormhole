use crate::aux_backends::common::test_support::scratch;
use crate::aux_backends::{import_apply, import_scan};

fn ghostty_home(root: &std::path::Path) -> String {
    let home = root.join("home");
    std::fs::create_dir_all(home.join(".config/ghostty")).expect("folder");
    std::fs::write(home.join(".config/ghostty/config"), "font-size = 15\nfont-family = Menlo\n").expect("config");
    home.display().to_string()
}

#[test]
fn a_terminal_with_settings_is_found_and_its_ticked_rows_are_applied() {
    let (root, data) = scratch();
    let home = ghostty_home(root.path());
    let found = import_scan(Some(home.clone())).expect("a scan");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].source.as_str(), found[0].name.as_str()), ("ghostty", "Ghostty"));
    let keys: Vec<_> = found[0].rows.iter().map(|r| r.setting.as_str()).collect();
    assert!(keys.contains(&"font_size") && keys.contains(&"font_family"), "{keys:?}");

    // Only what is ticked is taken.
    let taken = import_apply("ghostty".into(), Some(home), Some(vec!["font_size".into()]), Some(data.display().to_string()))
        .expect("applied");
    assert_eq!(taken, 1);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(data.join("config.json")).expect("config.json")).expect("JSON");
    assert_eq!(json["font_size"], 15.0);
    assert_ne!(json["font_family"], "Menlo", "unticked rows stay as they were");
}

#[test]
fn nothing_found_and_unknown_sources_are_plain_answers() {
    let (root, data) = scratch();
    let empty = root.path().join("empty-home");
    std::fs::create_dir_all(&empty).expect("folder");
    assert!(import_scan(Some(empty.display().to_string())).expect("a scan").is_empty());
    let err = import_apply("nope".into(), Some(empty.display().to_string()), None, Some(data.display().to_string()));
    assert!(err.is_err());
    let err = import_apply("ghostty".into(), Some(empty.display().to_string()), None, Some(data.display().to_string()));
    assert!(err.is_err(), "it has no settings there");
}
