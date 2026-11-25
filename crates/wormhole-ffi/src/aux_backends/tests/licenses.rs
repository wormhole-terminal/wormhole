use std::collections::HashSet;

use crate::aux_backends::licenses::{Kind, List};
use crate::aux_backends::{LicensesRecord, licenses};

#[test]
fn the_bundled_list_is_complete_and_shares_its_texts_by_index() {
    let list = licenses();
    assert!(list.packages.len() > 100, "the crates Wormhole is built from");
    for package in &list.packages {
        for &index in &package.texts {
            assert!((index as usize) < list.texts.len(), "{} points at a missing text", package.name);
        }
    }
    let shared = list.packages.iter().flat_map(|p| &p.texts).count();
    assert!(list.texts.len() < shared, "identical texts are stored once");
    assert!(list.packages.iter().any(|p| p.is_font), "the bundled font is listed too");
}

/// This library's list, not core's: the Swift app has UniFFI and no GPUI, and bundles icons the 1.x list has no
/// kind for.
#[test]
fn the_list_is_this_librarys_own() {
    let list = licenses();
    let has = |name: &str| list.packages.iter().any(|p| p.name == name);
    assert!(has("uniffi"), "the bridge's own dependency tree is listed");
    assert!(!has("gpui"), "the 1.x app's is not");
    assert!(has("Catppuccin VSCode Icons"), "the file icon theme is listed");
    assert!(has("JetBrains Mono"), "the bundled font is listed");
    assert!(!has("wormhole-core"), "this repository's own crates are not third-party software");
}

#[test]
fn a_dangling_text_index_is_an_error() {
    let broken = r#"{"texts": ["MIT text"], "crates": [
        {"name": "alpha", "version": "1.0.0", "license": "MIT", "repository": null, "texts": [0, 7]}]}"#;
    let err = List::parse(broken).expect_err("index 7 does not exist");
    assert!(err.to_string().contains("alpha 1.0.0"), "{err}");
    let record: LicensesRecord = List::parse(&broken.replace("[0, 7]", "[0]")).expect("parses").into();
    assert_eq!(record.packages[0].texts, [0]);
}

/// A dependency upgrade without `scripts/gen-licenses.py --ffi` leaves an entry for a version the library is no
/// longer built from. (The other direction, a crate missing from the list, needs `cargo tree` for each target and is
/// what `gen-licenses.py --ffi --check` tells.)
#[test]
fn the_list_matches_the_lockfile() {
    let lock: toml::Table = include_str!("../../../Cargo.lock").parse().expect("Cargo.lock parses");
    let locked: HashSet<(&str, &str)> = lock["package"]
        .as_array()
        .expect("packages")
        .iter()
        .filter_map(|p| Some((p.get("name")?.as_str()?, p.get("version")?.as_str()?)))
        .collect();
    let list = List::parse(include_str!("../../../assets/licenses.json")).expect("the bundled list parses");
    let stale: Vec<String> = list
        .crates
        .iter()
        .filter(|entry| entry.kind == Kind::Crate)
        .filter(|entry| !locked.contains(&(entry.name.as_str(), entry.version.as_str())))
        .map(|entry| format!("{} {}", entry.name, entry.version))
        .collect();
    assert!(stale.is_empty(), "run scripts/gen-licenses.py --ffi; not in Cargo.lock any more: {stale:?}");
}
