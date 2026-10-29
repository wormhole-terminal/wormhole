//! `repo-docs/docs/languages.md` §3 group 2: no message nobody asks for. Every constant and function the
//! generator wrote into `src/keys/` appears as an identifier in some crate's sources (`crates/*/src`),
//! `keys/` itself aside. No lookup by a string exists in the API, so a scan is complete: a table that
//! maps an enum to keys (`match section { … => KEY }`) names the constants and counts.
//!
//! Test code does not count as a use: a message only a test asks for is still one nobody sees. A file of
//! tests (`tests.rs`, `*_tests.rs`, a `tests/` folder under `src`) and an inline `#[cfg(test)] mod … {`
//! are left out, as `crates/wormhole-rules/tests/support/mod.rs` does for its scans (copied: a different crate).
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

/// A file of tests only, which `#[cfg(test)] mod x;` pulls in.
fn is_test_file(path: &Path) -> bool {
    let in_tests_dir = path.components().any(|part| part.as_os_str() == "tests");
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    in_tests_dir || name == "tests.rs" || name.ends_with("_tests.rs")
}

/// The part of a file that is not test code: up to a `#[cfg(test)]`-style attribute (one that
/// names `test`) in front of an inline module, which closes the file.
fn non_test_part(text: &str) -> &str {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(") && trimmed.contains("test") {
            let rest = &text[offset + line.len()..];
            let item = rest.lines().map(str::trim_start).find(|l| !l.starts_with("#["));
            if item.is_some_and(|l| (l.starts_with("mod ") || l.starts_with("pub mod ")) && l.ends_with('{'))
            {
                return &text[..offset];
            }
        }
        offset += line.len();
    }
    text
}

/// The text of every source under `crates/*/src` except the generated `keys/` and test code, comments cut.
fn used_text() -> String {
    let keys = keys_dir();
    let mut files = Vec::new();
    for krate in fs::read_dir(crates_dir()).expect("crates/").flatten() {
        sources(&krate.path().join("src"), &mut files);
    }
    files.sort();
    let mut text = String::new();
    let relevant = files.into_iter().filter(|file| !file.starts_with(&keys) && !is_test_file(file));
    for file in relevant {
        let source = fs::read_to_string(&file).unwrap_or_default();
        for line in non_test_part(&source).lines() {
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
