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

/// A crate that ships no license file gets the standard text only when it declares exactly MPL-2.0 or Apache-2.0
/// (a text with nothing to fill in), and its entry says so. The MIT, BSD and ISC crates need their holder's own
/// line, so they stay without a text.
#[test]
fn a_standard_text_is_only_for_a_crate_that_declares_exactly_that_license() {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/licenses.json")).expect("the bundled list parses");
    let texts = file["texts"].as_array().expect("texts");
    let standard = ["MPL-2.0", "Apache-2.0"];
    let mut marked = Vec::new();
    for entry in file["crates"].as_array().expect("crates") {
        let name = entry["name"].as_str().expect("name");
        let license = entry["license"].as_str().unwrap_or_default();
        let given = entry["texts"].as_array().expect("texts of an entry");
        if entry["standard_text"] == true {
            assert!(standard.contains(&license), "{name} declares {license}");
            assert_eq!(given.len(), 1, "{name}: the standard text, and nothing else");
            let text = texts[given[0].as_u64().expect("an index") as usize].as_str().expect("a text");
            assert!(
                text.contains("Mozilla Public License Version 2.0") || text.contains("Apache License"),
                "{name}"
            );
            marked.push(name);
        } else if given.is_empty() {
            assert!(!standard.contains(&license), "{name} could carry the standard text");
        }
    }
    assert!(marked.contains(&"uniffi"), "the UniFFI family declares MPL-2.0 and ships no file: {marked:?}");
}
