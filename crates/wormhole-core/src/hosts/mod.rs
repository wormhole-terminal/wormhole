//! Session hosts as the app sees them from the outside: their ids, the folder their files live
//! in, where their sockets are, and which of them are alive (`scan`). The host process itself is
//! the `wormhole-host` crate; the design is `repo-docs/docs/session-host.md`.
//!
//! Everything is files under `<data dir>/hosts/` (mode `0700`): `<id>.meta` (what a host says about
//! itself, written by the host) and `<id>.sock` (its socket). Nothing here talks to a host or
//! deletes a file: [`scan`] reports what is stale and the caller decides.

mod scan;

use std::io;
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

pub use scan::{HostMeta, Scan, StaleHost, StaleReason, read_meta, scan, scan_with};

/// How many characters a host id has.
pub const ID_LEN: usize = 12;

/// The alphabet of an id: lowercase letters and digits, so it is safe in a file name, a socket
/// path and an environment variable.
const ALPHABET: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";

/// `sun_path` holds 104 bytes on macOS, the terminating NUL included.
const SUN_PATH_MAX: usize = 104;

/// A new host id: [`ID_LEN`] characters of `[a-z0-9]` from the system's random source (about 62
/// bits). Random rather than a counter because a row of the database names a host by it, and a
/// recovered or copied database must never hand one host to the wrong terminal.
pub fn new_host_id() -> io::Result<String> {
    let mut id = String::with_capacity(ID_LEN);
    while id.len() < ID_LEN {
        let mut bytes = [0u8; 32];
        fill_random(&mut bytes)?;
        // 252 = 7 * 36: bytes at or above it are skipped so every letter is equally likely.
        id.extend(bytes.iter().filter(|b| **b < 252).map(|b| ALPHABET[usize::from(*b) % 36] as char));
    }
    id.truncate(ID_LEN);
    Ok(id)
}

#[allow(unsafe_code)]
fn fill_random(buf: &mut [u8]) -> io::Result<()> {
    debug_assert!(buf.len() <= 256, "getentropy gives at most 256 bytes");
    // SAFETY: `buf` is valid for writes of `buf.len()` bytes, which is at most the 256 getentropy
    // accepts.
    let result = unsafe { libc::getentropy(buf.as_mut_ptr().cast(), buf.len()) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Whether `id` is a host id: exactly [`ID_LEN`] characters of `[a-z0-9]`. Anything that becomes
/// part of a path or a command line is checked with this first.
pub fn is_valid_host_id(id: &str) -> bool {
    id.len() == ID_LEN && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// `<data dir>/hosts`: where the hosts' files are. Does not touch the disk; see
/// [`ensure_hosts_dir`].
pub fn hosts_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("hosts")
}

/// [`hosts_dir`], created with mode `0700` when it is missing. One that exists must be a real folder
/// (not a link) of this user, and gets its mode set back to `0700` when something loosened it: the
/// sockets in it are the only thing that keeps other users off a terminal.
pub fn ensure_hosts_dir(data_dir: &Path) -> io::Result<PathBuf> {
    let dir = hosts_dir(data_dir);
    std::fs::create_dir_all(data_dir)?;
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => return Err(err),
    }
    // Not `metadata`: a link planted at `hosts` must not be followed.
    let meta = std::fs::symlink_metadata(&dir)?;
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a folder", dir.display()),
        ));
    }
    if meta.uid() != current_uid() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} belongs to another user", dir.display()),
        ));
    }
    if meta.permissions().mode() & 0o777 != 0o700 {
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(dir)
}

fn current_uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    #[allow(unsafe_code)]
    unsafe {
        libc::getuid()
    }
}

/// `<hosts dir>/<id>.meta`.
pub fn meta_path(hosts_dir: &Path, id: &str) -> PathBuf {
    hosts_dir.join(format!("{id}.meta"))
}

/// Where host `id` listens: `<hosts dir>/<id>.sock`, or, when that path does not fit `sun_path`
/// (a long data dir), `$TMPDIR/wormhole-host-<uid>-<id>.sock`. The host writes the path it chose
/// into its meta file, so readers use [`HostMeta::socket`] and this is for whoever starts the host.
pub fn socket_path(hosts_dir: &Path, id: &str) -> PathBuf {
    socket_path_in(hosts_dir, &std::env::temp_dir(), current_uid(), id)
}

/// [`socket_path`] with the temp folder and uid as arguments, for the tests.
fn socket_path_in(hosts_dir: &Path, temp: &Path, uid: u32, id: &str) -> PathBuf {
    use std::os::unix::ffi::OsStrExt as _;
    let home = hosts_dir.join(format!("{id}.sock"));
    if home.as_os_str().as_bytes().len() < SUN_PATH_MAX {
        return home;
    }
    let name = format!("wormhole-host-{uid}-{id}.sock");
    let in_temp = temp.join(&name);
    // Same last resort as the app's own socket (`ipc::socket_path`).
    if in_temp.as_os_str().as_bytes().len() < SUN_PATH_MAX { in_temp } else { Path::new("/tmp").join(name) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_twelve_lowercase_letters_and_digits_and_differ() {
        let ids: HashSet<String> = (0..200).map(|_| new_host_id().expect("random")).collect();
        assert_eq!(ids.len(), 200, "no repeats");
        for id in &ids {
            assert!(is_valid_host_id(id), "{id}");
        }
        // Over 200 ids every character of the alphabet shows up (a broken modulo would not).
        let seen: HashSet<char> = ids.iter().flat_map(|id| id.chars()).collect();
        assert_eq!(seen.len(), 36);
    }

    #[test]
    fn the_validator_accepts_only_exact_ids() {
        assert!(is_valid_host_id("abcdefghij12"));
        assert!(is_valid_host_id("000000000000"));
        for bad in [
            "",
            "abcdefghij1",
            "abcdefghij123",
            "ABCDEFGHIJ12",
            "abcdefghij1-",
            "abcdefghij1.",
            "abcdefghij1/",
            "../abcdefghi",
            "abcdefghij1\n",
            "abcdefghij1é",
            "abcdefghij 1",
        ] {
            assert!(!is_valid_host_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_hosts_dir_is_made_private_and_a_loosened_one_is_tightened() {
        let data = tempfile::tempdir().expect("temp");
        let dir = ensure_hosts_dir(&data.path().join("Wormhole")).expect("creates, parents too");
        assert_eq!(dir, data.path().join("Wormhole").join("hosts"));
        let mode = |p: &Path| std::fs::metadata(p).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("loosen");
        ensure_hosts_dir(&data.path().join("Wormhole")).expect("again");
        assert_eq!(mode(&dir), 0o700);
    }

    #[test]
    fn a_link_in_place_of_the_hosts_dir_is_refused() {
        let data = tempfile::tempdir().expect("temp");
        let elsewhere = tempfile::tempdir().expect("temp");
        std::os::unix::fs::symlink(elsewhere.path(), data.path().join("hosts")).expect("link");
        let err = ensure_hosts_dir(data.path()).expect_err("refused");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn the_socket_sits_in_the_hosts_dir_unless_the_path_is_too_long() {
        let id = "abcdefghij12";
        let short = Path::new("/Users/me/Library/Application Support/Wormhole/hosts");
        assert_eq!(
            socket_path_in(short, Path::new("/var/folders/ab/T"), 501, id),
            short.join("abcdefghij12.sock")
        );
        // 103 bytes fit (the NUL makes 104), 104 do not.
        let name_len = format!("/{id}.sock").len();
        let fits = PathBuf::from(format!("/{}", "d".repeat(SUN_PATH_MAX - 1 - name_len - 1)));
        assert_eq!(fits.join(format!("{id}.sock")).as_os_str().len(), SUN_PATH_MAX - 1);
        assert_eq!(socket_path_in(&fits, Path::new("/t"), 501, id), fits.join("abcdefghij12.sock"));
        let long = PathBuf::from(format!("/{}", "d".repeat(SUN_PATH_MAX - name_len)));
        assert_eq!(
            socket_path_in(&long, Path::new("/var/folders/ab/T"), 501, id),
            Path::new("/var/folders/ab/T/wormhole-host-501-abcdefghij12.sock")
        );
        // A temp folder that is long too falls back to /tmp.
        let long_temp = PathBuf::from(format!("/{}", "t".repeat(120)));
        assert_eq!(
            socket_path_in(&long, &long_temp, 501, id),
            Path::new("/tmp/wormhole-host-501-abcdefghij12.sock")
        );
    }

    #[test]
    fn meta_paths_sit_beside_the_socket() {
        assert_eq!(meta_path(Path::new("/h"), "abcdefghij12"), Path::new("/h/abcdefghij12.meta"));
    }
}
