//! The generated files are the committed ones, and the English catalog is what the rules of
//! `repo-docs/docs/languages.md` §3 group 1 say it must be.
//!
//! A deliberate change to the English files is followed by
//! `TEXT_BLESS=1 cargo test -p wormhole-text --test generated`, and the diff of `src/keys/` and `src/generated/` is
//! reviewed with it. Never bless to make a red test green without knowing why it was red.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::fs;

use support::{check, crate_dir, generate};

/// Every generated file, by path under the crate's `src/`.
fn expected() -> Vec<(String, String)> {
    let now = generate::generate();
    let mut files =
        vec![("generated/ids.rs".to_string(), now.ids), ("generated/files.rs".to_string(), now.files)];
    files.extend(now.keys.into_iter().map(|(name, text)| (format!("keys/{name}"), text)));
    files
}

fn on_disk(dir: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(crate_dir().join("src").join(dir))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| format!("{dir}/{}", entry.file_name().to_string_lossy()))
        .collect();
    names.sort();
    names
}

#[test]
fn the_generated_files_are_fresh() {
    let now = expected();
    let src = crate_dir().join("src");
    if std::env::var_os("TEXT_BLESS").is_some() {
        fs::create_dir_all(src.join("keys")).expect("makes src/keys");
        for (name, text) in &now {
            fs::write(src.join(name), text).unwrap_or_else(|e| panic!("writes {name}: {e}"));
        }
        // A module of a file that is gone does not stay.
        for stale in on_disk("keys").into_iter().filter(|name| !now.iter().any(|(known, _)| known == name)) {
            fs::remove_file(src.join(&stale)).unwrap_or_else(|e| panic!("removes {stale}: {e}"));
        }
        return;
    }
    for (name, text) in &now {
        let committed = fs::read_to_string(src.join(name)).unwrap_or_default();
        assert!(
            committed == *text,
            "the generated {name} is stale: run TEXT_BLESS=1 cargo test -p wormhole-text --test generated"
        );
    }
    let known: Vec<&str> = now.iter().map(|(name, _)| name.as_str()).collect();
    for name in on_disk("keys") {
        assert!(known.contains(&name.as_str()), "src/{name} is not generated any more: bless to remove it");
    }
}

#[test]
fn the_english_catalog_is_well_formed() {
    let problems = check::english_problems(&support::english());
    assert!(problems.is_empty(), "locales/en:\n{}", problems.join("\n"));
}
