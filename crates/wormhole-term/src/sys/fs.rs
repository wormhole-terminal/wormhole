//! File modes and kinds, so callers don't name `std::os::unix`.

use std::fs::{DirBuilder, FileType, Metadata, OpenOptions};
use std::io;
use std::path::Path;

use super::unix::fs as os;

/// The permission bits of a file (with the file type's bits, as the system reports them: mask
/// with `0o777`).
pub fn mode(meta: &Metadata) -> u32 {
    os::mode(meta)
}

/// Set the permission bits of `path` (the file type's bits are ignored).
pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    os::set_mode(path, mode)
}

/// The file's identity on its volume, which a replacement at the same path does not keep.
pub fn inode(meta: &Metadata) -> u64 {
    os::inode(meta)
}

/// Whether the file is a socket.
pub fn is_socket(kind: &FileType) -> bool {
    os::is_socket(kind)
}

/// Make new folders owner-only (`0700`) from the moment they exist.
pub fn private_dir(builder: &mut DirBuilder) -> &mut DirBuilder {
    os::private_dir(builder)
}

/// Make a new file owner-only (`0600`) from the moment it exists.
pub fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    os::private_file(options)
}

/// Make `link` a symbolic link to `target`.
pub fn symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    os::symlink(target.as_ref(), link.as_ref())
}
