use std::fs::{DirBuilder, FileType, Metadata, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{
    DirBuilderExt as _, FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::Path;

pub(crate) fn mode(meta: &Metadata) -> u32 {
    meta.mode()
}

pub(crate) fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    std::fs::set_permissions(path, Permissions::from_mode(mode))
}

pub(crate) fn inode(meta: &Metadata) -> u64 {
    meta.ino()
}

pub(crate) fn is_socket(kind: &FileType) -> bool {
    kind.is_socket()
}

pub(crate) fn private_dir(builder: &mut DirBuilder) -> &mut DirBuilder {
    builder.mode(0o700)
}

pub(crate) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}

pub(crate) fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// A plain `fsync(2)` of `file`: the data is handed to the drive, which is enough for a file that is written
/// whole and renamed over its name. (Rust's `File::sync_all` is `F_FULLFSYNC` on macOS, which also flushes the
/// drive's own cache and takes far longer; a checkpoint written at a restart has about a second.)
#[allow(unsafe_code)] // libc FFI: one call on a descriptor the borrow keeps open.
pub(crate) fn fsync(file: &std::fs::File) -> io::Result<()> {
    use std::os::fd::AsRawFd as _;
    // SAFETY: fsync takes the descriptor of an open file and nothing else.
    if unsafe { libc::fsync(file.as_raw_fd()) } < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}
