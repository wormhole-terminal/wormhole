//! The host's socket and its directory: `0700` for the directory, `0600` for the socket, never
//! over a host that still listens, and bound again when the file went (macOS's temporary-file
//! cleaner deletes old sockets in `$TMPDIR`, where a long home folder puts them; section 5).

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sys;

/// How often the socket is touched and looked for.
pub const TOUCH_EVERY: Duration = Duration::from_secs(60 * 60);

/// Make sure `dir` is a directory of this user's that only this user can enter.
pub fn prepare_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the hosts directory is not a directory"));
    }
    if meta.uid() != sys::euid() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the hosts directory is someone else's"));
    }
    if meta.mode() & 0o077 != 0 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// A bound socket and what it was bound as.
pub struct Listener {
    pub listener: UnixListener,
    pub path: PathBuf,
    /// The socket file's inode: a different one at `path` is not ours any more.
    ino: u64,
}

impl Listener {
    /// Bind `path`, non-blocking. A socket file left by a host that is gone is replaced; one a
    /// host still listens on is not (`AddrInUse`), nor is anything that is not a socket.
    pub fn bind(path: &Path) -> io::Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(path) {
            if !meta.file_type().is_socket() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "something else is at the socket's path",
                ));
            }
            if UnixStream::connect(path).is_ok() {
                return Err(io::Error::new(io::ErrorKind::AddrInUse, "a host already listens there"));
            }
            fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        // The umask made it 0600 already; said again for a host started without it.
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let ino = fs::symlink_metadata(path)?.ino();
        Ok(Self { listener, path: path.to_owned(), ino })
    }

    /// Whether the file at our path is still our socket.
    pub fn is_ours(&self) -> bool {
        fs::symlink_metadata(&self.path).is_ok_and(|m| m.ino() == self.ino && m.file_type().is_socket())
    }

    /// The hourly look: touch the socket; when it is gone, bind it again. Whether it was bound
    /// again (the caller re-registers the new listener).
    pub fn refresh(&mut self) -> io::Result<bool> {
        if self.is_ours() {
            sys::touch(&self.path)?;
            return Ok(false);
        }
        tracing::info!("the socket file went, binding again");
        *self = Self::bind(&self.path)?;
        Ok(true)
    }

    /// Remove the socket file, if it is still ours.
    pub fn remove(&self) {
        if self.is_ours() {
            let _ = fs::remove_file(&self.path);
        }
    }
}
