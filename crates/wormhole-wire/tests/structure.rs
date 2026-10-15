//! The crate's structure rules, checked on every test run (as `wormhole-vt`'s are): files stay
//! small, it depends on nothing, and no message type is left out of the samples that pin the
//! format.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use wormhole_wire::{max_payload, samples};

/// A tripwire, not the target: files aim for ~600 lines with one responsibility each.
const MAX_LINES: usize = 1000;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn files_stay_under_the_length_limit() {
    let mut files = Vec::new();
    for dir in ["src", "tests", "examples", "benches", "fuzz/fuzz_targets", "fuzz/src"] {
        rust_files(&crate_dir().join(dir), &mut files);
    }
    let long: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let lines = fs::read_to_string(path).ok()?.lines().count();
            (lines > MAX_LINES).then(|| format!("{} ({lines} lines)", path.display()))
        })
        .collect();
    assert!(long.is_empty(), "split by responsibility (over {MAX_LINES} lines): {long:#?}");
}

#[test]
fn the_crate_has_no_dependencies_and_forbids_unsafe_code() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).expect("manifest");
    let deps: Vec<&str> = manifest
        .lines()
        .skip_while(|l| l.trim() != "[dependencies]")
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect();
    assert!(deps.is_empty(), "the host links this crate; keep it dependency-free: {deps:?}");
    let lib = fs::read_to_string(crate_dir().join("src/lib.rs")).expect("lib.rs");
    assert!(lib.contains("#![forbid(unsafe_code)]"));
}

#[test]
fn every_message_type_id_has_a_limit_and_a_sample() {
    let header = fs::read_to_string(crate_dir().join("src/header.rs")).expect("header.rs");
    let ids: Vec<u8> = header
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub const ")?.split_once(": u8 = 0x"))
        .filter_map(|(_, rest)| u8::from_str_radix(rest.trim_end_matches(';'), 16).ok())
        .collect();
    assert!(ids.len() > 30, "found the ids in header.rs ({})", ids.len());
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "type ids are never reused");

    let sampled: BTreeSet<u8> = samples::all().into_iter().map(|(_, m)| m.ty()).collect();
    for id in &unique {
        assert!(max_payload(*id).is_some(), "type {id:#04x} has no size limit");
        assert!(sampled.contains(id), "type {id:#04x} has no sample (and so no golden vector)");
    }
    assert_eq!(sampled, unique);
}
