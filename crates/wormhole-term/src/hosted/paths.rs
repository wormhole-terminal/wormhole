//! Where a session host listens. The host is the only one that decides it, by this rule, and writes
//! what it chose into its meta file (`socket=`); everyone else reads the meta, never the rule.
//!
//! The socket belongs in `<hosts dir>/<id>.sock`, next to the meta, in the `0700` directory. A
//! Unix socket's path must fit `sun_path` (104 bytes with its NUL on macOS), which a long home
//! folder can break; the socket then goes to the per-user `$TMPDIR` as
//! `wormhole-host-<uid>-<id>.sock`, and to `/tmp` when even that is too long. A socket outside
//! the hosts directory is in a directory others may write to, so a client checks the peer it
//! reaches there (its uid) before it believes it is the host the meta names.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// `sun_path`'s size on macOS: the most bytes a socket's path may have, its NUL included.
pub const SUN_PATH_MAX: usize = 104;

/// Where the host with `id` listens, given its hosts directory (see the module docs).
pub fn socket_path(hosts_dir: &Path, id: &str) -> PathBuf {
    let tmpdir = std::env::var_os("TMPDIR");
    socket_path_with(hosts_dir, id, tmpdir.as_deref(), effective_uid())
}

/// `<hosts dir>/<id>.meta`.
pub fn meta_path(hosts_dir: &Path, id: &str) -> PathBuf {
    super::meta::path_in(hosts_dir, id)
}

/// Whether `path` fits `sun_path` with its NUL.
pub fn fits(path: &Path) -> bool {
    path.as_os_str().len() < SUN_PATH_MAX
}

/// [`socket_path`] with the temporary folder and the uid given.
fn socket_path_with(hosts_dir: &Path, id: &str, tmpdir: Option<&OsStr>, uid: u32) -> PathBuf {
    let preferred = hosts_dir.join(format!("{id}.sock"));
    if fits(&preferred) {
        return preferred;
    }
    let name = format!("wormhole-host-{uid}-{id}.sock");
    if let Some(tmp) = tmpdir.map(Path::new).filter(|tmp| tmp.is_absolute()) {
        let path = tmp.join(&name);
        if fits(&path) {
            return path;
        }
    }
    Path::new("/tmp").join(name)
}

#[allow(unsafe_code)] // libc FFI; no arguments.
fn effective_uid() -> u32 {
    // SAFETY: geteuid takes nothing and cannot fail.
    unsafe { libc::geteuid() }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "abcdefghij12";

    #[test]
    fn a_short_hosts_dir_holds_the_socket() {
        let dir = Path::new("/Users/me/Library/Application Support/Wormhole/hosts");
        let path = socket_path_with(dir, ID, Some(OsStr::new("/var/folders/x/T/")), 501);
        assert_eq!(path, dir.join("abcdefghij12.sock"));
    }

    #[test]
    fn a_long_one_falls_back_to_tmpdir_and_then_to_tmp() {
        let long = PathBuf::from(format!("/Users/{}/hosts", "x".repeat(90)));
        let tmp = OsStr::new("/var/folders/ab/cdef/T/");
        assert_eq!(
            socket_path_with(&long, ID, Some(tmp), 501),
            Path::new("/var/folders/ab/cdef/T/wormhole-host-501-abcdefghij12.sock")
        );
        let long_tmp = format!("/var/{}", "t".repeat(100));
        for tmp in [None, Some(OsStr::new("relative/tmp")), Some(OsStr::new(&long_tmp))] {
            assert_eq!(
                socket_path_with(&long, ID, tmp, 501),
                Path::new("/tmp/wormhole-host-501-abcdefghij12.sock"),
                "{tmp:?}"
            );
        }
    }

    #[test]
    fn the_limit_counts_the_nul() {
        let exactly = PathBuf::from(format!("/{}", "a".repeat(SUN_PATH_MAX - 2)));
        assert!(fits(&exactly), "103 bytes and the NUL fit");
        let over = PathBuf::from(format!("/{}", "a".repeat(SUN_PATH_MAX - 1)));
        assert!(!fits(&over));
    }
}
