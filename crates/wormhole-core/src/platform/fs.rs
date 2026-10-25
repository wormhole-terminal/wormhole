//! File modes, kinds and identity, so the rest of core names no `std::os::unix`:
//! [`wormhole_term::sys::fs`] (shared with the terminal layer and the session host) and what only
//! core needs.

use std::fs::{Metadata, OpenOptions, Permissions};
use std::path::Path;

use super::unix::fs as os;

pub use wormhole_term::sys::fs::*;

/// Permissions with exactly these mode bits.
pub fn permissions(mode: u32) -> Permissions {
    os::permissions(mode)
}

/// Open without waiting: a FIFO has no writer yet, and opening it must not block a read that
/// refuses anything but a regular file.
pub fn no_block(options: &mut OpenOptions) -> &mut OpenOptions {
    os::no_block(options)
}

/// [`no_block`], and refuse a link at the last component: a link swapped in after the path was
/// checked fails instead of being followed.
pub fn no_block_no_follow(options: &mut OpenOptions) -> &mut OpenOptions {
    os::no_block_no_follow(options)
}

/// Set the mode of the folder at `path` through a descriptor opened without following a link, so a name
/// swapped for a link between the check and the change is refused rather than followed.
pub fn set_dir_mode_no_follow(path: &Path, mode: u32) -> std::io::Result<()> {
    os::open_dir_no_follow(path)?.set_permissions(permissions(mode))
}

/// A path made of exactly these bytes, which need not be text (file names on Unix are bytes).
pub fn path_from_bytes(bytes: &[u8]) -> &Path {
    os::path_from_bytes(bytes)
}

/// Whether two metadata are of the same file: the same device and inode.
pub fn same_file(a: &Metadata, b: &Metadata) -> bool {
    os::same_file(a, b)
}

/// Whether anyone may execute the file (any execute bit).
pub fn is_executable(meta: &Metadata) -> bool {
    os::is_executable(meta)
}

/// A file's modification time as seconds and nanoseconds since the epoch.
pub fn mtime(meta: &Metadata) -> (i64, i64) {
    os::mtime(meta)
}

/// What a path looks like from outside, to notice a change without reading it: identity (device,
/// inode), type and permissions, size and both change times. `ctime` is in it because tools that
/// restore `mtime` (`cp -p`, `rsync -t`, `touch -d`) can't set it, and the inode because
/// `git checkout` and atomic saves replace the file.
///
/// A port without those fields would fall back to what the platform has (size, modification and
/// creation times, and whether it is a folder), as an earlier version of this type already did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    dev: u64,
    ino: u64,
    mode: u32,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}

impl Stamp {
    pub fn of(meta: &Metadata) -> Self {
        Self {
            dev: os::device(meta),
            ino: inode(meta),
            mode: mode(meta),
            len: meta.len(),
            mtime: os::mtime(meta),
            ctime: os::ctime(meta),
        }
    }
}
