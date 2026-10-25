//! The fingerprint a Probe is approved under: SHA-256 over every file of its folder.
//!
//! The rule is project trust's (`storage::fingerprint`); this is the Probe's use of it: one input
//! per file, named by its path and taken in path order, under a tag of its own so that a Probe's
//! fingerprint can never equal a project's. An edit, an added file or a removed one changes it, and
//! so does a rename. The manifest is one of the files, so a changed permission changes it too.

use super::read::{ProbeFile, ProbeFolder};
use crate::storage::fingerprint::{Input, fingerprint_of};

/// Part of every Probe fingerprint. Changing it asks again about every Probe once.
pub const FINGERPRINT_VERSION: &str = "wormhole-probe-v1";

/// The fingerprint of `files`, whatever order they come in.
pub fn fingerprint(files: &[ProbeFile]) -> String {
    let mut sorted: Vec<&ProbeFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    fingerprint_of(
        FINGERPRINT_VERSION,
        sorted.into_iter().map(|file| (file.path.as_str(), Input::of_bytes(file.bytes()))),
    )
}

impl ProbeFolder {
    /// What an approval of this folder is bound to.
    pub fn fingerprint(&self) -> String {
        fingerprint(&self.files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[(&str, &str)]) -> Vec<ProbeFile> {
        list.iter().map(|(path, text)| ProbeFile::new(*path, text.as_bytes())).collect()
    }

    #[test]
    fn the_fingerprint_is_pinned() {
        // Computed outside this code, as `storage::fingerprint`'s pin is: the tag, then per file
        // the length-prefixed path, "present", the length and the SHA-256 of the bytes.
        assert_eq!(
            fingerprint(&files(&[("probe.toml", "[probe]\n"), ("scripts/run.sh", "echo hi\n")])),
            PINNED
        );
    }

    const PINNED: &str = "f8133cd3e1026f379ffc63094358fe0518510ac9a687e2d8c5a4b93c95e8c14a";

    #[test]
    fn the_order_the_files_come_in_does_not_matter() {
        let a = files(&[("a", "1"), ("b", "2"), ("c/d", "3")]);
        let mut b = a.clone();
        b.reverse();
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn an_edit_an_addition_a_removal_and_a_rename_each_change_it() {
        let base = fingerprint(&files(&[("probe.toml", "x"), ("run.sh", "y")]));
        for changed in [
            files(&[("probe.toml", "x"), ("run.sh", "z")]),
            files(&[("probe.toml", "x"), ("run.sh", "y"), ("more", "")]),
            files(&[("probe.toml", "x")]),
            files(&[("probe.toml", "x"), ("go.sh", "y")]),
            files(&[("probe.toml", "x "), ("run.sh", "y")]),
        ] {
            assert_ne!(fingerprint(&changed), base, "{changed:?}");
        }
    }

    #[test]
    fn a_probe_never_has_a_projects_fingerprint() {
        // The same bytes under project trust's tag give another hash.
        let input = [("probe.toml", Input::of_bytes(b"x"))];
        assert_ne!(fingerprint_of("andromeda-trust-v3", input), fingerprint(&files(&[("probe.toml", "x")])));
    }
}
