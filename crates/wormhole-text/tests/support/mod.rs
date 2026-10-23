//! What the catalog tests share: reading `.ftl` files into a model, the generator, the rules.
//! Included by each test binary with `mod support;`, which uses only part of it.
#![allow(dead_code)]
#![allow(unreachable_pub, reason = "not yet cleaned (R-13)")]

pub mod catalog;
pub mod check;
pub mod generate;

use std::fs;
use std::path::{Path, PathBuf};

/// The crate's folder; tests run with it as the working directory, but a path from the manifest
/// does not depend on that.
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn locales_dir() -> PathBuf {
    crate_dir().join("locales")
}

/// `(file name, source)` of every `.ftl` in `dir`, sorted by name.
pub fn read_dir(dir: &Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ftl"))
        .map(|entry| {
            let source = fs::read_to_string(entry.path()).expect("an .ftl file is UTF-8 text");
            (entry.file_name().to_string_lossy().into_owned(), source)
        })
        .collect();
    files.sort();
    files
}

/// Every language directory under `locales/`: English first, the others by tag.
pub fn languages() -> Vec<String> {
    let mut tags: Vec<String> = fs::read_dir(locales_dir())
        .expect("locales/ exists")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    tags.sort_by(|a, b| (a != "en", a).cmp(&(b != "en", b)));
    tags
}

pub fn english() -> catalog::Catalog {
    catalog::Catalog::parse(&read_dir(&locales_dir().join("en")))
}
