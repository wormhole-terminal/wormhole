use std::io;
use std::path::Path;

/// Nothing builds this (macOS is the only target): a look, then a rename, which can overwrite a file
/// that appears in between. The macOS side is a single system call.
pub(crate) fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
}
