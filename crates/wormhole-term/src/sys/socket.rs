//! The sockets a session host and the control commands are reached on, and who is at the other end.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use super::user::UserId;

pub use super::unix::socket::{Listener, Stream};

#[cfg(target_os = "linux")]
use super::linux::socket as os;
#[cfg(target_os = "macos")]
use super::macos::socket as os;

/// The most bytes a socket's path may have here. `sun_path` holds 104 on macOS, its NUL included; one
/// number below that for every socket, so the hook socket's name (recomputed by every process, never
/// recorded) and a host's (recorded in its meta file) are decided by the same rule.
pub const PATH_LIMIT: usize = 100;

/// Whether `path` fits a socket address.
pub fn fits(path: &Path) -> bool {
    path.as_os_str().len() < PATH_LIMIT
}

/// Where a socket goes: `preferred` when it fits, else `fallback_name` in the per-user temporary folder
/// (`$TMPDIR`, when it is absolute), else in `/tmp`. A socket outside the folder it was meant for is in a
/// directory others may write to, so whoever connects there checks the peer ([`peer_is_same_user`]).
pub fn socket_in(preferred: &Path, fallback_name: &str) -> PathBuf {
    socket_in_with(preferred, fallback_name, std::env::var_os("TMPDIR").as_deref())
}

/// [`socket_in`] with the temporary folder given.
pub fn socket_in_with(preferred: &Path, fallback_name: &str, tmpdir: Option<&OsStr>) -> PathBuf {
    if fits(preferred) {
        return preferred.to_path_buf();
    }
    if let Some(tmp) = tmpdir.map(Path::new).filter(|tmp| tmp.is_absolute()) {
        let path = tmp.join(fallback_name);
        if fits(&path) {
            return path;
        }
    }
    Path::new("/tmp").join(fallback_name)
}

/// The user at the other end of `stream`, as the system recorded it when it connected.
pub fn peer_user(stream: &Stream) -> io::Result<UserId> {
    os::peer_uid(stream).map(UserId::from_raw)
}

/// Whether the peer of `stream` is the user this process acts as. A peer whose id can't be read is
/// not: a server answers only what it can attribute.
pub fn peer_is_same_user(stream: &Stream) -> bool {
    peer_user(stream).is_ok_and(|peer| peer == UserId::current())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &str = "wormhole-host-501-abcdefghij12.sock";

    #[test]
    fn a_path_that_fits_is_kept() {
        let preferred = Path::new("/Users/me/Library/Application Support/Wormhole/hosts/abcdefghij12.sock");
        assert_eq!(socket_in_with(preferred, NAME, Some(OsStr::new("/var/folders/x/T/"))), preferred);
    }

    #[test]
    fn one_that_does_not_goes_to_tmpdir_and_then_to_tmp() {
        let long = PathBuf::from(format!("/Users/{}/hosts/a.sock", "x".repeat(90)));
        assert_eq!(
            socket_in_with(&long, NAME, Some(OsStr::new("/var/folders/ab/cdef/T/"))),
            Path::new("/var/folders/ab/cdef/T/").join(NAME)
        );
        let long_tmp = format!("/var/{}", "t".repeat(100));
        for tmp in [None, Some(OsStr::new("relative/tmp")), Some(OsStr::new(&long_tmp))] {
            assert_eq!(socket_in_with(&long, NAME, tmp), Path::new("/tmp").join(NAME), "{tmp:?}");
        }
    }

    #[test]
    fn the_limit_counts_the_nul_and_leaves_room() {
        let exactly = PathBuf::from(format!("/{}", "a".repeat(PATH_LIMIT - 2)));
        assert!(fits(&exactly), "{} bytes fit", PATH_LIMIT - 1);
        let over = PathBuf::from(format!("/{}", "a".repeat(PATH_LIMIT - 1)));
        assert!(!fits(&over));
    }
}
