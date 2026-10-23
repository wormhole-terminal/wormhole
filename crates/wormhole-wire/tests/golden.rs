//! The wire format, pinned. `tests/vectors/golden.txt` holds the exact bytes of one frame per
//! sample message (channel 1, no flags), as `name = hex`. A host started yesterday talks to an app
//! updated today, so an encoding that changes is a break: this test fails on any difference, and
//! says which message.
//!
//! A deliberate change to the format (a new minor's addition leaves old vectors alone and adds
//! lines) is blessed with `WIRE_BLESS=1 cargo test -p wormhole-wire --test golden`, and the diff of
//! `golden.txt` is then reviewed like the protocol change it is. Never bless to make a red test
//! green without knowing why it was red.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use support::*;
use wormhole_wire::*;

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/golden.txt")
}

fn current() -> Vec<(&'static str, Vec<u8>)> {
    samples::all().into_iter().map(|(name, msg)| (name, encode_one(CHANNEL_LOCAL, &msg))).collect()
}

fn render(vectors: &[(&str, Vec<u8>)]) -> String {
    let mut out = String::from(
        "# Golden frames of wormhole-wire, one per line: name = hex (channel 1, no flags).\n\
         # Pinned by tests/golden.rs; see its header before changing a line.\n",
    );
    for (name, bytes) in vectors {
        let _ = writeln!(out, "{name} = {}", hex(bytes));
    }
    out
}

#[test]
fn the_encoding_matches_the_committed_vectors() {
    let now = current();
    if std::env::var_os("WIRE_BLESS").is_some() {
        std::fs::write(path(), render(&now)).expect("writes the vectors");
        return;
    }
    let text = std::fs::read_to_string(path()).expect("tests/vectors/golden.txt exists");
    let committed: BTreeMap<&str, &str> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split_once(" = ").expect("name = hex"))
        .collect();
    for (name, bytes) in &now {
        let want =
            committed.get(name).unwrap_or_else(|| panic!("{name}: no committed vector (WIRE_BLESS=1)"));
        assert_eq!(&hex(bytes), want, "{name}: the encoding changed");
    }
    assert_eq!(committed.len(), now.len(), "a committed vector has no sample any more");
}

#[test]
fn the_committed_vectors_decode_to_the_samples() {
    // The other direction: bytes from a past build must still read as the same message.
    let text = std::fs::read_to_string(path()).expect("tests/vectors/golden.txt exists");
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let (name, hexed) = line.split_once(" = ").expect("name = hex");
        let (_, want) = samples::all().into_iter().find(|(n, _)| *n == name).expect("sample exists");
        let bytes = unhex(hexed);
        let mut dec = Decoder::new();
        dec.push(&bytes);
        let frame = dec.next_frame().expect(name).expect(name);
        assert_eq!(frame.message, want, "{name}");
    }
}

#[test]
fn sample_names_are_unique() {
    let mut names: Vec<_> = samples::all().into_iter().map(|(n, _)| n).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before);
}
