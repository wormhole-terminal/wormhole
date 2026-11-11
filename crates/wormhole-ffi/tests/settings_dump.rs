//! `settings.json`, the Swift UI's copy of the settings schema, is what `wormhole-core` holds today.
//! On a mismatch run `macos/scripts/dump-settings.sh`.

#[test]
fn committed_settings_json_is_what_core_holds() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../macos/swift/Sources/SettingsModel/Resources/settings.json");
    let committed = std::fs::read_to_string(path).expect("settings.json is committed");
    assert!(
        committed == wormhole_ffi::settings_dump::json(),
        "settings.json is stale: run macos/scripts/dump-settings.sh"
    );
}

#[test]
fn every_row_has_a_control() {
    let json = wormhole_ffi::settings_dump::json();
    assert!(
        !json.contains("\"unsupported\""),
        "a settings row has no control: add it to settings_dump/controls.rs"
    );
}
