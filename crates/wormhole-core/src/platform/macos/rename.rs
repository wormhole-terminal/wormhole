use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd as _;
use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::{Path, PathBuf};

use crate::platform::unix::fs::open_dir_no_follow;

fn c_name(name: &OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has a NUL in its name", name.to_string_lossy()),
        )
    })
}

/// A folder opened by its descriptor.
pub(crate) struct Folder(File);

impl Folder {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        open_dir_no_follow(path).map(Self)
    }

    /// `F_GETPATH`: where the system says the descriptor's folder is.
    pub(crate) fn real_path(&self) -> io::Result<PathBuf> {
        let mut buffer = vec![0u8; libc::PATH_MAX as usize];
        // SAFETY: the descriptor is open for as long as `self`, and `F_GETPATH` writes a NUL-terminated
        // path of at most MAXPATHLEN bytes into the buffer, which has room for that.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::fcntl(self.0.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) };
        if rc == -1 {
            return Err(io::Error::last_os_error());
        }
        let len = buffer.iter().position(|&byte| byte == 0).unwrap_or(buffer.len());
        buffer.truncate(len);
        Ok(PathBuf::from(std::ffi::OsString::from_vec(buffer)))
    }

    /// `faccessat(AT_SYMLINK_NOFOLLOW)`: is there an entry of this name, a link counting as itself.
    pub(crate) fn has(&self, name: &OsStr) -> io::Result<bool> {
        let name = c_name(name)?;
        // SAFETY: the descriptor is open, `name` is NUL-terminated and outlives the call.
        #[allow(unsafe_code)]
        let rc = unsafe {
            libc::faccessat(self.0.as_raw_fd(), name.as_ptr(), libc::F_OK, libc::AT_SYMLINK_NOFOLLOW)
        };
        if rc == 0 {
            return Ok(true);
        }
        let err = io::Error::last_os_error();
        match err.kind() {
            io::ErrorKind::NotFound => Ok(false),
            _ => Err(err),
        }
    }

    /// `renameatx_np` with `RENAME_EXCL`, both names relative to this folder: the system refuses with
    /// `EEXIST` when `to` exists, and walks no path above the folder.
    pub(crate) fn rename_no_replace(&self, from: &OsStr, to: &OsStr) -> io::Result<()> {
        let (from, to) = (c_name(from)?, c_name(to)?);
        let fd = self.0.as_raw_fd();
        // SAFETY: `fd` is open, both names are NUL-terminated and outlive the call; the call only reads them.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::renameatx_np(fd, from.as_ptr(), fd, to.as_ptr(), libc::RENAME_EXCL) };
        if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
}
