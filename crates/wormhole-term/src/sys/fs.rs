//! File modes and kinds, so callers don't name `std::os::unix`.

use std::fs::{Metadata, OpenOptions};
use std::io;
use std::path::Path;

use super::unix::fs as os;

/// The permission bits of a file (with the file type's bits, as the system reports them: mask
/// with `0o777`).
pub fn mode(meta: &Metadata) -> u32 {
    os::mode(meta)
}

/// Make a new file owner-only (`0600`) from the moment it exists.
pub fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    os::private_file(options)
}

/// Make `link` a symbolic link to `target`.
pub fn symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    os::symlink(target.as_ref(), link.as_ref())
}
