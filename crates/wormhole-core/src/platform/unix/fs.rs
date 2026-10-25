use std::ffi::OsStr;
use std::fs::{Metadata, OpenOptions, Permissions};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;

pub(crate) fn permissions(mode: u32) -> Permissions {
    Permissions::from_mode(mode)
}

/// `O_NONBLOCK`: opening a FIFO doesn't wait for a writer.
pub(crate) fn no_block(options: &mut OpenOptions) -> &mut OpenOptions {
    options.custom_flags(libc::O_NONBLOCK)
}

/// `O_NONBLOCK | O_NOFOLLOW`: also refuse a link at the last component. (The close-on-exec flag the
/// standard library always adds stays.)
pub(crate) fn no_block_no_follow(options: &mut OpenOptions) -> &mut OpenOptions {
    options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
}

/// Open a folder for its descriptor, refusing a link at the last component (`O_NOFOLLOW | O_DIRECTORY`):
/// what is changed through the descriptor is the folder that was opened, not what a name is swapped for.
pub(crate) fn open_dir_no_follow(path: &Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY).open(path)
}

pub(crate) fn path_from_bytes(bytes: &[u8]) -> &Path {
    Path::new(OsStr::from_bytes(bytes))
}

pub(crate) fn is_executable(meta: &Metadata) -> bool {
    meta.permissions().mode() & 0o111 != 0
}

pub(crate) fn same_file(a: &Metadata, b: &Metadata) -> bool {
    a.dev() == b.dev() && a.ino() == b.ino()
}

pub(crate) fn device(meta: &Metadata) -> u64 {
    meta.dev()
}

/// Modification time as seconds and nanoseconds since the epoch.
pub(crate) fn mtime(meta: &Metadata) -> (i64, i64) {
    (meta.mtime(), meta.mtime_nsec())
}

/// Status-change time as seconds and nanoseconds since the epoch.
pub(crate) fn ctime(meta: &Metadata) -> (i64, i64) {
    (meta.ctime(), meta.ctime_nsec())
}
