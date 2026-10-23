//! `repo-docs/docs/languages.md` §3 group 2: no message nobody asks for. Every constant and function the
//! generator wrote into `src/keys/` appears as an identifier in some crate's sources (`crates/*/src`),
//! `keys/` itself aside. No lookup by a string exists in the API, so a scan is complete: a table that
//! maps an enum to keys (`match section { … => KEY }`) names the constants and counts.
//!
//! A message that is no longer used is deleted from `locales/en/` (and blessed away), or a translator is
//! made to translate a sentence the interface never shows.

#![allow(clippy::disallowed_methods, reason = "a test reads the workspace's sources")]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crates/").to_path_buf()
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn keys_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/keys")
}

/// The text of every source under `crates/*/src` except the generated `keys/`, comments cut.
fn used_text() -> String {
    let keys = keys_dir();
    let mut files = Vec::new();
    for krate in fs::read_dir(crates_dir()).expect("crates/").flatten() {
        sources(&krate.path().join("src"), &mut files);
    }
    files.sort();
    let mut text = String::new();
    for file in files.into_iter().filter(|file| !file.starts_with(&keys)) {
        let source = fs::read_to_string(&file).unwrap_or_default();
        for line in source.lines() {
            text.push_str(line.split("//").next().unwrap_or(""));
            text.push('\n');
        }
    }
    text
}

/// The names the generated `keys/` define: its constants (`pub const NAME: Key`) and functions (`pub fn name(`),
/// not the helpers that are only there for tests.
fn defined(keys: &str) -> Vec<String> {
    keys.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let name = line.strip_prefix("pub const ").or_else(|| line.strip_prefix("pub fn "))?;
            let name: String = name.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            (name != "ALL" && name != "samples").then_some(name)
        })
        .collect()
}

/// The identifiers of `text`: runs of letters, digits and underscores.
fn identifiers(text: &str) -> BTreeSet<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|word| !word.is_empty()).collect()
}

#[test]
fn every_message_is_asked_for_by_some_source() {
    let mut generated = Vec::new();
    sources(&keys_dir(), &mut generated);
    let keys: String = generated.iter().map(|file| fs::read_to_string(file).unwrap_or_default()).collect();
    let used = used_text();
    let words = identifiers(&used);
    let unused: Vec<String> =
        defined(&keys).into_iter().filter(|name| !words.contains(name.as_str())).collect();
    assert!(
        unused.is_empty(),
        "messages no source asks for (delete them from locales/en/ and bless, or use them):\n{}",
        unused.join("\n")
    );
}

#[test]
fn the_scan_reads_names_and_skips_comments() {
    let keys = "pub const A_KEY: Key = Key::new(0);\n/// doc\npub fn a_sentence(x: &str) -> String {\npub const ALL: [Key; 1] = [A_KEY];\npub fn samples() {\n";
    assert_eq!(defined(keys), ["A_KEY", "a_sentence"]);
    let words = identifiers("let x = text(A_KEY); // a_sentence in a comment\n");
    assert!(words.contains("A_KEY") && words.contains("text"));
}
