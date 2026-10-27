//! The seal on an approval: HMAC-SHA256 under the approval's own key (`key`), so an approval that was not
//! written by Wormhole, or was changed after, reads as not approved.
//!
//! Without it, `probe-approvals.json` is a fingerprint (public: SHA-256 of files anyone can read) and the
//! grants, and a program with file access could write one and have a Probe turned on without the Touch
//! ID that `probes.md` 6.1 promises it cannot press. With it, writing an entry takes the key, and the key
//! is the Keychain's.
//!
//! What is sealed is every field that says what is allowed, in one unambiguous encoding: each field
//! after the domain tag is length-prefixed (a `u64`, little-endian, then the bytes), the way
//! `storage::fingerprint` frames its inputs, so no two different approvals encode alike.
//!
//! ```text
//! tag      "wormhole-probe-approval-v2"
//! version  the store's format version (u32, little-endian)
//! id       the Probe's id
//! account  the account its key is kept under
//! print    the folder's fingerprint
//! release  the manifest's version, as shown on the page
//! declared count (u64), then each permission id, sorted
//! granted  count (u64), then each permission id, sorted
//! when     approved_at (u64, little-endian)
//! ```
//!
//! The tag keeps a seal from standing for anything else a key might one day sign. Sorting makes the seal
//! independent of the order the lists were written in. The check is `ring::hmac::verify`, constant time.

use ring::hmac;

use super::store::Approval;
use crate::storage::secret_key::Key;

/// Names what the seal is for.
const TAG: &[u8] = b"wormhole-probe-approval-v2";

/// An approval's key, ready to seal with and check against.
pub(super) struct SealKey(hmac::Key);

impl SealKey {
    pub(super) fn new(key: &Key) -> Self {
        Self(hmac::Key::new(hmac::HMAC_SHA256, key.as_ref()))
    }

    /// The seal of `approval` for Probe `id`, its key under `account`, in a store of format `version`, as hex.
    pub(super) fn seal(&self, version: u32, id: &str, account: &str, approval: &Approval) -> String {
        let tag = hmac::sign(&self.0, &encode(version, id, account, approval));
        tag.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Whether `seal` is this key's seal of exactly this approval.
    pub(super) fn verify(
        &self,
        version: u32,
        id: &str,
        account: &str,
        approval: &Approval,
        seal: &str,
    ) -> bool {
        let Some(tag) = unhex(seal) else { return false };
        hmac::verify(&self.0, &encode(version, id, account, approval), &tag).is_ok()
    }
}

fn encode(version: u32, id: &str, account: &str, approval: &Approval) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    field(&mut out, TAG);
    field(&mut out, &version.to_le_bytes());
    field(&mut out, id.as_bytes());
    field(&mut out, account.as_bytes());
    field(&mut out, approval.fingerprint.as_bytes());
    field(&mut out, approval.version.as_bytes());
    list(&mut out, &approval.declared);
    list(&mut out, &approval.granted);
    field(&mut out, &approval.approved_at.to_le_bytes());
    out
}

fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// A sorted list: its count, then each item as a field.
fn list(out: &mut Vec<u8>, items: &[String]) {
    let mut sorted: Vec<&str> = items.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    out.extend_from_slice(&(sorted.len() as u64).to_le_bytes());
    for item in sorted {
        field(out, item.as_bytes());
    }
}

/// The bytes of a hex string, or `None` when it is anything else.
fn unhex(text: &str) -> Option<Vec<u8>> {
    let digits: Vec<u32> = text.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    Some(digits.chunks(2).map(|pair| (pair[0] * 16 + pair[1]) as u8).collect())
}
