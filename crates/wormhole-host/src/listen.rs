//! The host's socket and its directory: `0700` for the directory, `0600` for the socket, never
//! over a host that still listens, and bound again when the file went (macOS's temporary-file
//! cleaner deletes old sockets in `$TMPDIR`, where a long home folder puts them; section 5).

use std::fs::{self, DirBuilder};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use wormhole_term::sys::fs::{inode, is_socket, mode, private_dir, set_mode};
use wormhole_term::sys::socket::{self, Stream};
use wormhole_term::sys::user::UserId;

use crate::sys;

/// How often the socket is touched and looked for.
pub(crate) const TOUCH_EVERY: Duration = Duration::from_secs(60 * 60);

/// Make sure `dir` is a directory of this user's that only this user can enter.
pub(crate) fn prepare_dir(dir: &Path) -> io::Result<()> {
    private_dir(DirBuilder::new().recursive(true)).create(dir)?;
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the hosts directory is not a directory"));
    }
    if UserId::owner_of(&meta) != UserId::current() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the hosts directory is someone else's"));
    }
    if mode(&meta) & 0o077 != 0 {
        set_mode(dir, 0o700)?;
    }
    Ok(())
}

/// A bound socket and what it was bound as.
pub(crate) struct Listener {
    pub listener: socket::Listener,
    pub path: PathBuf,
    /// The socket file's inode: a different one at `path` is not ours any more.
    ino: u64,
}

impl Listener {
    /// Bind `path`, non-blocking. A socket file left by a host that is gone is replaced; one a
    /// host still listens on is not (`AddrInUse`), nor is anything that is not a socket.
    pub(crate) fn bind(path: &Path) -> io::Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(path) {
            if !is_socket(&meta.file_type()) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "something else is at the socket's path",
                ));
            }
            if Stream::connect(path).is_ok() {
                return Err(io::Error::new(io::ErrorKind::AddrInUse, "a host already listens there"));
            }
            fs::remove_file(path)?;
        }
        let listener = socket::Listener::bind(path)?;
        // The umask made it 0600 already; said again for a host started without it.
        set_mode(path, 0o600)?;
        listener.set_nonblocking(true)?;
        let ino = inode(&fs::symlink_metadata(path)?);
        Ok(Self { listener, path: path.to_owned(), ino })
    }

    /// Whether the file at our path is still our socket.
    pub(crate) fn is_ours(&self) -> bool {
        fs::symlink_metadata(&self.path).is_ok_and(|m| inode(&m) == self.ino && is_socket(&m.file_type()))
    }

    /// The hourly look: touch the socket; when it is gone, bind it again. Whether it was bound
    /// again (the caller re-registers the new listener).
    pub(crate) fn refresh(&mut self) -> io::Result<bool> {
        if self.is_ours() {
            sys::fs::touch(&self.path)?;
            return Ok(false);
        }
        tracing::info!("the socket file went, binding again");
        *self = Self::bind(&self.path)?;
        Ok(true)
    }

    /// Remove the socket file, if it is still ours.
    pub(crate) fn remove(&self) {
        if self.is_ours() {
            let _ = fs::remove_file(&self.path);
        }
    }
}
