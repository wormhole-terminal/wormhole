//! Frames of past releases, frozen: `tests/vectors/release-<tag>.txt` is the `golden.txt` its tag
//! had, copied once and never blessed. A host started under an old release talks to an app
//! updated today (and the other way round), so every frame that release wrote must still decode
//! with this build, to the message it was. `golden.rs` pins what HEAD writes; this pins what the
//! past wrote.
//!
//! The ledger says which protocol minor introduced each sample. The test fails when a sample is
//! not in the ledger (a new message must say when it came), when the ledger names one that no
//! longer exists, and when one is newer than [`PROTO_MINOR`] (the minor must be bumped with the
//! message). A release file holds exactly the samples of its minor and before.
//!
//! To add a release: copy its `golden.txt` here as `release-<tag>.txt` with the frozen header, and
//! add the tag and the protocol minor it spoke to [`RELEASES`].

mod support;

use std::path::PathBuf;

use support::*;
use wormhole_wire::version::PROTO_MINOR;
use wormhole_wire::*;

/// The protocol minor that introduced each sample message. Everything not listed here came with
/// minor 1, the first release.
const LEDGER: &[(&str, u16)] = &[("clear_history", 2)];

/// The released tags kept here, and the protocol minor each spoke.
const RELEASES: &[(&str, u16)] = &[("v1.2.0", 1), ("v1.5.0", 2)];

fn introduced(name: &str) -> u16 {
    LEDGER.iter().find(|(n, _)| *n == name).map_or(1, |(_, minor)| *minor)
}

fn vectors(tag: &str) -> Vec<(String, String)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/vectors/release-{tag}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (name, hexed) = l.split_once(" = ").expect("name = hex");
            (name.to_string(), hexed.to_string())
        })
        .collect()
}

#[test]
fn every_released_frame_still_decodes_to_the_message_it_was() {
    for (tag, _) in RELEASES {
        for (name, hexed) in vectors(tag) {
            let (_, want) = samples::all()
                .into_iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{tag}: {name} is not a sample any more"));
            let mut dec = Decoder::new();
            dec.push(&unhex(&hexed));
            let frame = dec.next_frame().unwrap_or_else(|e| panic!("{tag} {name}: {e}")).expect(&name);
            assert_eq!(frame.message, want, "{tag} {name}");
            // And the encoding of that message is the bytes the release wrote.
            assert_eq!(hex(&encode_one(CHANNEL_LOCAL, &want)), hexed, "{tag} {name}: the encoding changed");
        }
    }
}

#[test]
fn every_sample_is_in_the_ledger_and_none_is_newer_than_this_build() {
    let names: Vec<&str> = samples::all().into_iter().map(|(n, _)| n).collect();
    for (name, minor) in LEDGER {
        assert!(names.contains(name), "the ledger names {name}, which is not a sample");
        assert!(*minor > 1, "{name}: minor 1 is the default, do not list it");
    }
    for name in &names {
        let minor = introduced(name);
        assert!(minor <= PROTO_MINOR, "{name} came with minor {minor}, past PROTO_MINOR {PROTO_MINOR}");
    }
    // A sample added after the newest release must be in the ledger with its minor: when the newest
    // release file lacks it, it has to be newer than that release.
    let (newest, newest_minor) = RELEASES.last().expect("a release");
    let in_release: Vec<String> = vectors(newest).into_iter().map(|(n, _)| n).collect();
    for name in &names {
        if !in_release.iter().any(|n| n == name) {
            assert!(
                introduced(name) > *newest_minor,
                "{name} is not in {newest}'s vectors, so the ledger must say a newer minor introduced it"
            );
        }
    }
}

#[test]
fn a_release_file_holds_exactly_the_samples_of_its_minor_and_before() {
    for (tag, minor) in RELEASES {
        let mut have: Vec<String> = vectors(tag).into_iter().map(|(n, _)| n).collect();
        have.sort();
        let mut want: Vec<String> = samples::all()
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| introduced(n) <= *minor)
            .map(str::to_string)
            .collect();
        want.sort();
        assert_eq!(have, want, "{tag} (protocol 1.{minor})");
    }
}
