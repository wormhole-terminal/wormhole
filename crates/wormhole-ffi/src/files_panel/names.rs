//! File names across the bridge. A name on disk is bytes; Swift's paths are text. A name that is valid UTF-8 crosses as
//! itself. One that is not crosses as a key: its valid parts as they are and each other byte as a NUL followed by the
//! byte's two hex digits. No name on disk holds a NUL, so a key is never another file's name, and read back it is the
//! very bytes that were listed. Swift treats a key as opaque (it joins keys with `/`, compares them and hands them
//! back); what it draws is the entry's label.
//!
//! Hostile input: a repository chooses its names. A key that does not read back (a NUL not followed by two hex
//! digits) names nothing, and the call answers as for a path that is gone.

use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

/// What marks a byte that is not text in a key.
const MARK: u8 = 0;

/// `name` (a name or a whole path) as Swift carries it.
pub(crate) fn key(name: &OsStr) -> String {
    let mut out = String::with_capacity(name.len());
    for chunk in name.as_bytes().utf8_chunks() {
        out.push_str(chunk.valid());
        for byte in chunk.invalid() {
            out.push(char::from(MARK));
            // Writing to a `String` cannot fail.
            let _ = write!(out, "{byte:02x}");
        }
    }
    out
}

/// The path a key (or plain text) from Swift names, or `None` for a key that is not one.
pub(crate) fn path(key: &str) -> Option<PathBuf> {
    let bytes = key.as_bytes();
    if !bytes.contains(&MARK) {
        return Some(PathBuf::from(key));
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&first, after)) = rest.split_first() {
        if first != MARK {
            out.push(first);
            rest = after;
            continue;
        }
        let (hex, after) = after.split_at_checked(2)?;
        let hex = std::str::from_utf8(hex).ok().filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))?;
        let byte = u8::from_str_radix(hex, 16).ok()?;
        // Only a byte that is not text is written this way; a NUL or a slash smuggled in is refused.
        if byte.is_ascii() {
            return None;
        }
        out.push(byte);
        rest = after;
    }
    Some(PathBuf::from(OsString::from_vec(out)))
}

/// A whole path as Swift carries it.
pub(crate) fn path_key(path: &Path) -> String {
    key(path.as_os_str())
}

/// The folder a key (or plain text) from Swift names: a terminal's folder, a repository's or a project's root, as
/// `path_key` gave it. A key that is not one names a folder that is nowhere: no look at it, or above it, finds anything.
pub(crate) fn folder(key: &str) -> PathBuf {
    path(key).unwrap_or_else(|| PathBuf::from(NOWHERE))
}

/// A path no file can have (a NUL in it fails every system call) whose only folder above is the root.
const NOWHERE: &str = "/\0";

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    use super::{NOWHERE, folder, path_key};

    #[test]
    fn a_folder_that_is_not_text_reads_back_as_its_bytes_and_a_broken_key_names_nowhere() {
        let bytes = b"/tmp/caf\xe9/sub".to_vec();
        let key = path_key(&PathBuf::from(OsString::from_vec(bytes.clone())));
        assert!(key.contains('\0') && key.ends_with("/sub"), "{key:?}");
        assert_eq!(folder(&key).as_os_str().as_bytes(), bytes.as_slice());
        assert_eq!(folder("/tmp/plain"), PathBuf::from("/tmp/plain"));
        // Hostile: a NUL that is not a byte over 0x7f in two hex digits (a slash smuggled in, a cut key).
        for broken in ["/tmp/caf\0e", "/tmp/caf\0zz", "/tmp/\02f/etc"] {
            let nowhere = folder(broken);
            assert_eq!(nowhere, PathBuf::from(NOWHERE), "{broken:?}");
            assert!(!nowhere.exists() && wormhole_core::git::find_root(&nowhere).is_none());
        }
    }
}
