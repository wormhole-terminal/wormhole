//! Copying a folder tree as copy-on-write clones: a clone shares the original's data blocks until either
//! side changes a file, so it is far faster than reading and writing every byte, and only its metadata
//! (the directory entries and inodes) is new disk space at first. Only macOS (APFS) has this; elsewhere
//! the call says it is unsupported, and the caller goes without.

use std::io;
use std::path::Path;

#[cfg(target_os = "linux")]
use super::linux::clone as os;
#[cfg(target_os = "macos")]
use super::macos::clone as os;

/// Clone the folder `from` to `to`, which must not exist yet (its parent must). Links inside are
/// cloned as links, never followed, and no mount point inside `from` is crossed. It fails with
/// [`io::ErrorKind::Unsupported`], copying nothing, unless `from`, the parent of `to` and `same_volume_as`
/// (a folder the caller will move the result into) are all on the same APFS volume: a full copy instead of
/// a clone would be slow and take the disk space the caller counted on sparing, and a move to another
/// volume would be a copy as well. On failure part-way, a partial `to` may be left for the caller to remove.
pub fn clone_dir(from: &Path, to: &Path, same_volume_as: &Path) -> io::Result<()> {
    os::clone_dir(from, to, same_volume_as)
}
