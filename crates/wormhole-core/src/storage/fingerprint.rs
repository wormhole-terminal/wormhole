//! One hash over a folder's files, the way an approval is bound to what was approved.
//!
//! A project's `.wormhole/` folder (`project::trust`) and a Probe's folder (`probe::fingerprint`)
//! are trusted by the same rule: the user approves exact contents, and any change, addition or
//! removal of an input turns the approval into "changed". This is the rule and nothing more: which
//! inputs there are, in which order, and under which `version` tag is each caller's. A tag per kind
//! of folder keeps a project's fingerprint from ever equalling a Probe's.

use sha2::{Digest, Sha256};

/// One input of a fingerprint, as read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Absent,
    /// Refused or failed to read: nothing in it can run, and the loaders report why.
    Unreadable,
    Present {
        len: u64,
        digest: [u8; 32],
    },
}

impl Input {
    /// The input for a file whose whole content is `bytes`.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self::Present { len: bytes.len() as u64, digest: Sha256::digest(bytes).into() }
    }
}

/// SHA-256 (hex) over named inputs, in the order given, under `version`. Names are length-prefixed
/// and each input is marked absent, unreadable or present (with its length and digest), so adding,
/// deleting, moving or editing any input changes the result. The bytes are raw (even white space or
/// comments count): what the user approved is what is on disk.
///
/// The encoding is stored in every approval ever given: changing it asks everyone again.
pub fn fingerprint_of<'a>(version: &str, inputs: impl IntoIterator<Item = (&'a str, Input)>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(version.as_bytes());
    hasher.update([0]);
    for (name, input) in inputs {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        match input {
            Input::Absent => hasher.update(b"absent\0"),
            Input::Unreadable => hasher.update(b"unreadable\0"),
            Input::Present { len, digest } => {
                hasher.update(b"present\0");
                hasher.update(len.to_le_bytes());
                hasher.update(digest);
            }
        }
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> Vec<(&'static str, Input)> {
        vec![
            ("a.txt", Input::of_bytes(b"alpha\n")),
            ("b/c.txt", Input::of_bytes(b"")),
            ("gone", Input::Absent),
            ("bad", Input::Unreadable),
        ]
    }

    /// Computed outside this code (a few lines of Python following the encoding above), so the
    /// encoding that every stored approval depends on cannot drift unnoticed.
    #[test]
    fn the_encoding_is_pinned() {
        assert_eq!(
            fingerprint_of("pin-v1", inputs()),
            "65b52806d293fa5f5380360e4e5c31e365636fdffdd2f2f1ae3b68ec564d0ad1"
        );
    }

    #[test]
    fn any_change_of_an_input_the_order_or_the_tag_changes_it() {
        let base = fingerprint_of("v1", inputs());
        assert_ne!(base, fingerprint_of("v2", inputs()), "the tag");
        let mut reversed = inputs();
        reversed.reverse();
        assert_ne!(base, fingerprint_of("v1", reversed), "the order");
        let mut edited = inputs();
        edited[0].1 = Input::of_bytes(b"alpha");
        assert_ne!(base, fingerprint_of("v1", edited), "a byte");
        let mut renamed = inputs();
        renamed[0].0 = "a.txt2";
        assert_ne!(base, fingerprint_of("v1", renamed), "a name");
        let mut missing = inputs();
        missing[2].1 = Input::Unreadable;
        assert_ne!(base, fingerprint_of("v1", missing), "absent against unreadable");
    }
}
