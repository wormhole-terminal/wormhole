//! The fingerprint a Probe is approved under: SHA-256 over every file of its folder.
//!
//! The rule is project trust's (`storage::fingerprint`); this is the Probe's use of it: one input
//! per file, named by its path and taken in path order, under a tag of its own so that a Probe's
//! fingerprint can never equal a project's. An edit, an added file or a removed one changes it, and
//! so does a rename. The manifest is one of the files, so a changed permission changes it too. So does
//! the execute bit: a script that can start is not the same script as one that cannot, and the approved
//! copy keeps the bit, so it is what was approved.

use super::read::{ProbeFile, ProbeFolder};
use crate::storage::fingerprint::{Input, fingerprint_of};

/// Part of every Probe fingerprint. Changing it asks again about every Probe once.
pub const FINGERPRINT_VERSION: &str = "wormhole-probe-v2";

/// The fingerprint of `files`, whatever order they come in.
pub fn fingerprint(files: &[ProbeFile]) -> String {
    let mut sorted: Vec<&ProbeFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    // The bit is its own input, named by the path and a NUL, which no file name holds, so it can never
    // be mistaken for a file. Only files that have it add one, so most Probes hash as plain files.
    let mut inputs: Vec<(String, Input)> = Vec::with_capacity(sorted.len());
    for file in sorted {
        inputs.push((file.path.clone(), Input::of_bytes(file.bytes())));
        if file.is_executable() {
            inputs.push((format!("{}\0exec", file.path), Input::of_bytes(b"exec")));
        }
    }
    fingerprint_of(FINGERPRINT_VERSION, inputs.iter().map(|(name, input)| (name.as_str(), *input)))
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

    const PINNED: &str = "a65e6534ac8471f28af56f042a9a1f2ec98536a01e0ef56556fcce8a8428f266";

    #[test]
    fn the_execute_bit_is_pinned_too() {
        let list = [
            ProbeFile::new("probe.toml", b"[probe]\n".to_vec()),
            ProbeFile::new("scripts/run.sh", b"echo hi\n".to_vec()).with_executable(true),
        ];
        assert_eq!(fingerprint(&list), "448498cc2a42a42bd340a0545ea2d5880337a89e8123098992663e269f0fa102");
    }

    #[test]
    fn a_script_that_became_executable_is_another_script() {
        let plain = files(&[("run.sh", "echo")]);
        let executable = vec![ProbeFile::new("run.sh", b"echo".to_vec()).with_executable(true)];
        assert_ne!(fingerprint(&plain), fingerprint(&executable));
    }

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
