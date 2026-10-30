use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

/// Nothing builds this (macOS is the only target): the folder is a path, looked at and then renamed in two
/// steps, so a file that appears in between can be overwritten. The macOS side is one system call on a
/// descriptor.
pub(crate) struct Folder(PathBuf);

impl Folder {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        if !std::fs::symlink_metadata(path)?.is_dir() {
            return Err(io::Error::from(io::ErrorKind::NotADirectory));
        }
        Ok(Self(path.to_path_buf()))
    }

    pub(crate) fn real_path(&self) -> io::Result<PathBuf> {
        std::fs::canonicalize(&self.0)
    }

    pub(crate) fn has(&self, name: &OsStr) -> io::Result<bool> {
        Ok(std::fs::symlink_metadata(self.0.join(name)).is_ok())
    }

    pub(crate) fn rename_no_replace(&self, from: &OsStr, to: &OsStr) -> io::Result<()> {
        if self.has(to)? {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        std::fs::rename(self.0.join(from), self.0.join(to))
    }

    pub(crate) fn move_no_replace_into(&self, from: &OsStr, dest: &Self, to: &OsStr) -> io::Result<()> {
        if dest.has(to)? {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        std::fs::rename(self.0.join(from), dest.0.join(to))
    }
}
