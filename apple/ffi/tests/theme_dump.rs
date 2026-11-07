//! `themes.json`, the Swift UI's copy of the bundled themes, is what `wormhole-core` resolves today.
//! On a mismatch run `apple/scripts/dump-themes.sh`.

#[test]
fn committed_themes_json_is_what_core_resolves() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../swift/Sources/DesignSystem/Resources/themes.json");
    let committed = std::fs::read_to_string(path).expect("themes.json is committed");
    assert!(
        committed == wormhole_ffi::theme_dump::json(),
        "themes.json is stale: run apple/scripts/dump-themes.sh"
    );
}
