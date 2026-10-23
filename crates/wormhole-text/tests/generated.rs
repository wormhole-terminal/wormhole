//! The generated files are the committed ones, and the English catalog is what the rules of
//! `repo-docs/docs/languages.md` §3 group 1 say it must be.
//!
//! A deliberate change to the English files is followed by
//! `TEXT_BLESS=1 cargo test -p wormhole-text --test generated`, and the diff of `src/keys.rs` and `src/generated/` is
//! reviewed with it. Never bless to make a red test green without knowing why it was red.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::fs;

use support::{check, crate_dir, generate};

/// Where each generated file lives: the data in `src/generated/`, the keys beside `lib.rs`.
fn path(name: &str) -> std::path::PathBuf {
    let src = crate_dir().join("src");
    if name == "keys.rs" { src.join(name) } else { src.join("generated").join(name) }
}

fn committed(name: &str) -> String {
    fs::read_to_string(path(name)).unwrap_or_default()
}

#[test]
fn the_generated_files_are_fresh() {
    let now = generate::generate();
    if std::env::var_os("TEXT_BLESS").is_some() {
        for (name, text) in [("ids.rs", &now.ids), ("keys.rs", &now.keys), ("files.rs", &now.files)] {
            fs::write(path(name), text).unwrap_or_else(|e| panic!("writes {name}: {e}"));
        }
        return;
    }
    for (name, text) in [("ids.rs", &now.ids), ("keys.rs", &now.keys), ("files.rs", &now.files)] {
        assert!(
            *text == committed(name),
            "the generated {name} is stale: run TEXT_BLESS=1 cargo test -p wormhole-text --test generated"
        );
    }
}

#[test]
fn the_english_catalog_is_well_formed() {
    let problems = check::english_problems(&support::english());
    assert!(problems.is_empty(), "locales/en:\n{}", problems.join("\n"));
}
