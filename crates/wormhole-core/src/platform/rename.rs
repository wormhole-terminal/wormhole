//! Renaming a file or folder without replacing anything: the one call the Files tab's Rename needs from
//! the system, because `std::fs::rename` silently overwrites a target that exists.

use std::io;
use std::path::Path;

#[cfg(target_os = "linux")]
use super::linux::rename as os;
#[cfg(target_os = "macos")]
use super::macos::rename as os;

/// Moves `from` to `to` (the entry itself: a link is renamed, never followed) unless something is at `to`,
/// which is an [`io::ErrorKind::AlreadyExists`] error. One step for the system, so a file that appears
/// at `to` after a look is not overwritten either.
///
/// It also fails with `AlreadyExists` for a case-only rename (`readme.md` to `README.md`) on a volume
/// that ignores case, where `to` is `from` itself: the caller tells that from a real clash.
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    os::rename_no_replace(from, to)
}

/// Moves `from` to `to`, replacing what is at `to`. For the case-only rename that
/// [`rename_no_replace`] refuses, where `to` is the same entry.
pub fn rename_same_entry(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)
}
