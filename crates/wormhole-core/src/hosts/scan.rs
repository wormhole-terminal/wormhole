//! Which session hosts are alive, from the files they leave in `<data dir>/hosts/`.
//!
//! A host is live when its meta file reads, its pid runs and its socket exists. A pid alone proves
//! nothing (pids are reused) and neither does a file, so a caller that is about to talk to a host
//! still checks it is the right one with the protocol's `Welcome`; this only narrows the list and
//! tells the stale files apart. Nothing here deletes anything.

use std::io;
use std::os::unix::fs::FileTypeExt as _;
use std::path::{Path, PathBuf};

use crate::fsutil::{self, LinkPolicy};

use super::is_valid_host_id;

/// A meta file is a dozen short lines; anything bigger is not one.
const META_CAP: u64 = 16 * 1024;

/// What a host says about itself in `<id>.meta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostMeta {
    pub id: String,
    /// The host process.
    pub pid: u32,
    /// The terminal's shell, once the host knows it.
    pub child_pid: Option<u32>,
    /// Unix seconds.
    pub started_at: u64,
    pub host_version: String,
    pub proto_min: u32,
    pub proto_max: u32,
    /// Where the host listens (an absolute path; see [`super::socket_path`]).
    pub socket: PathBuf,
    pub data_dir: PathBuf,
}

/// Read and check one meta file.
///
/// TODO(integration): the format's reader and writer live in `wormhole_term::hosted::meta`
/// (shared by the host, core and the app). This is a local parser of the same `key=value` format
/// (contract `p1-contract.md`) so the two could be written in parallel; at integration the body
/// becomes a call to that module and the `HostMeta` type its re-export.
///
/// The file is hostile input as far as its size and content go (any process of the user can write
/// to the folder): it is read capped and without following links, unknown keys and lines without
/// `=` are skipped, and the id inside must be valid and equal the file name's.
pub fn read_meta(path: &Path) -> io::Result<HostMeta> {
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {what}", path.display()));
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".meta"))
        .filter(|s| is_valid_host_id(s))
        .ok_or_else(|| bad("not named <id>.meta"))?;
    // `Within` the folder: a link planted as the file is refused, not followed.
    let root = path.parent().ok_or_else(|| bad("no folder"))?;
    let text = fsutil::read_to_string_capped(path, META_CAP, LinkPolicy::Within(root))?;

    let (mut format, mut id, mut pid, mut child_pid, mut started_at) = (None, None, None, None, None);
    let (mut host_version, mut proto_min, mut proto_max, mut socket, mut data_dir) =
        (String::new(), 0, 0, None, None);
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        match key {
            "format" => format = value.parse::<u32>().ok(),
            "id" => id = Some(value),
            "pid" => pid = value.parse::<u32>().ok(),
            "child_pid" => child_pid = value.parse::<u32>().ok().filter(|p| *p > 0),
            "started_at" => started_at = value.parse::<u64>().ok(),
            "host_version" => host_version = value.to_string(),
            "proto_min" => proto_min = value.parse().map_err(|_| bad("proto_min"))?,
            "proto_max" => proto_max = value.parse().map_err(|_| bad("proto_max"))?,
            "socket" => socket = Some(PathBuf::from(value)),
            "data_dir" => data_dir = Some(PathBuf::from(value)),
            _ => {}
        }
    }
    if format != Some(1) {
        return Err(bad("unknown format"));
    }
    if id != Some(stem) {
        return Err(bad("id does not match the file name"));
    }
    let pid = pid.filter(|p| *p > 0 && i32::try_from(*p).is_ok()).ok_or_else(|| bad("pid"))?;
    let socket = socket.filter(|s| s.is_absolute()).ok_or_else(|| bad("socket"))?;
    Ok(HostMeta {
        id: stem.to_string(),
        pid,
        child_pid,
        started_at: started_at.ok_or_else(|| bad("started_at"))?,
        host_version,
        proto_min,
        proto_max,
        socket,
        data_dir: data_dir.unwrap_or_default(),
    })
}

/// Why a host's files are left over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    /// The meta file does not read (or is not a meta file); the text says why.
    Unreadable(String),
    /// Its pid no longer runs: the host crashed or was killed.
    PidGone,
    /// The host runs but there is no socket where it says (it removes and re-binds it hourly, so a
    /// rescan a moment later may find it).
    SocketMissing,
    /// A socket with no meta file beside it.
    NoMeta,
}

/// Files of a host that is not alive, reported so the caller can show or remove them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleHost {
    pub id: String,
    pub reason: StaleReason,
    /// What the meta file said, when it could be read.
    pub meta: Option<HostMeta>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Hosts that are alive, oldest first.
    pub live: Vec<HostMeta>,
    /// Hosts with files but no life, by id.
    pub stale: Vec<StaleHost>,
}

/// Look in `hosts_dir`. A folder that is not there is an empty scan.
pub fn scan(hosts_dir: &Path) -> io::Result<Scan> {
    scan_with(hosts_dir, pid_runs)
}

/// [`scan`] with the check for a running pid as an argument, for the tests.
pub fn scan_with(hosts_dir: &Path, pid_runs: impl Fn(u32) -> bool) -> io::Result<Scan> {
    let entries = match std::fs::read_dir(hosts_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Scan::default()),
        Err(err) => return Err(err),
    };
    let mut metas = Vec::new();
    let mut socket_ids = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if let Some(id) = name.strip_suffix(".meta").filter(|id| is_valid_host_id(id)) {
            metas.push(id.to_string());
        } else if let Some(id) = name.strip_suffix(".sock").filter(|id| is_valid_host_id(id)) {
            socket_ids.push(id.to_string());
        }
    }
    metas.sort();
    let mut scan = Scan::default();
    for id in &metas {
        match read_meta(&super::meta_path(hosts_dir, id)) {
            Err(err) => scan.stale.push(StaleHost {
                id: id.clone(),
                reason: StaleReason::Unreadable(err.to_string()),
                meta: None,
            }),
            Ok(meta) if !pid_runs(meta.pid) => {
                scan.stale.push(StaleHost { id: id.clone(), reason: StaleReason::PidGone, meta: Some(meta) })
            }
            Ok(meta) if !is_socket(&meta.socket) => scan.stale.push(StaleHost {
                id: id.clone(),
                reason: StaleReason::SocketMissing,
                meta: Some(meta),
            }),
            Ok(meta) => scan.live.push(meta),
        }
    }
    // A socket in the folder whose meta is gone (the meta is written first and removed last, so
    // this is a host that was killed in between, or files someone removed by hand).
    for id in socket_ids {
        if metas.binary_search(&id).is_err() {
            scan.stale.push(StaleHost { id, reason: StaleReason::NoMeta, meta: None });
        }
    }
    scan.live.sort_by(|a, b| (a.started_at, &a.id).cmp(&(b.started_at, &b.id)));
    scan.stale.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scan)
}

/// Whether a process with this pid exists: signal 0 checks without sending anything. `EPERM` means
/// it exists but is not ours, which still counts (the pid is taken).
#[allow(unsafe_code)]
fn pid_runs(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else { return false };
    if pid <= 0 {
        return false;
    }
    // SAFETY: kill with signal 0 only checks that the process exists and may be signalled.
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Whether `path` is a socket itself (a link, or a file of another kind, is not).
fn is_socket(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_socket())
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;

    fn meta_text(id: &str, pid: u32, socket: &Path, started_at: u64) -> String {
        format!(
            "format=1\nid={id}\npid={pid}\nchild_pid=4242\nstarted_at={started_at}\nhost_version=0.3.0\n\
             proto_min=1\nproto_max=2\nsocket={}\ndata_dir=/d\nfuture_key=whatever\n",
            socket.display()
        )
    }

    /// A host's files: its meta, and a socket when `listening`. Returns the listener to keep it open.
    fn host(dir: &Path, id: &str, pid: u32, started_at: u64, listening: bool) -> Option<UnixListener> {
        let socket = dir.join(format!("{id}.sock"));
        std::fs::write(dir.join(format!("{id}.meta")), meta_text(id, pid, &socket, started_at))
            .expect("meta");
        listening.then(|| UnixListener::bind(&socket).expect("binds"))
    }

    fn runs(pids: &'static [u32]) -> impl Fn(u32) -> bool {
        move |pid| pids.contains(&pid)
    }

    #[test]
    fn a_meta_file_reads_and_unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("abcdefghij12.meta");
        std::fs::write(&path, meta_text("abcdefghij12", 77, Path::new("/s/x.sock"), 1_700_000_000))
            .expect("write");
        let meta = read_meta(&path).expect("reads");
        assert_eq!(meta.id, "abcdefghij12");
        assert_eq!((meta.pid, meta.child_pid, meta.started_at), (77, Some(4242), 1_700_000_000));
        assert_eq!((meta.proto_min, meta.proto_max), (1, 2));
        assert_eq!(meta.host_version, "0.3.0");
        assert_eq!(meta.socket, Path::new("/s/x.sock"));
        assert_eq!(meta.data_dir, Path::new("/d"));
    }

    #[test]
    fn a_meta_file_that_is_wrong_is_refused() {
        let dir = tempfile::tempdir().expect("temp");
        let good = meta_text("abcdefghij12", 77, Path::new("/s/x.sock"), 5);
        let cases = [
            ("format=2", good.replace("format=1", "format=2")),
            ("other id", good.replace("id=abcdefghij12", "id=zzzzzzzzzzzz")),
            ("no pid", good.replace("pid=77\n", "")),
            ("pid 0", good.replace("pid=77", "pid=0")),
            ("pid too big", good.replace("pid=77", "pid=4294967295")),
            ("relative socket", good.replace("socket=/s/x.sock", "socket=x.sock")),
            ("no socket", good.replace("socket=/s/x.sock\n", "")),
            ("no start", good.replace("started_at=5\n", "")),
            ("binary", String::from_utf8_lossy(&[0xff, 0xfe, b'\n']).into_owned()),
            ("empty", String::new()),
        ];
        for (name, text) in cases {
            let path = dir.path().join("abcdefghij12.meta");
            std::fs::write(&path, text).expect("write");
            assert!(read_meta(&path).is_err(), "{name}");
        }
        let misnamed = dir.path().join("short.meta");
        std::fs::write(&misnamed, &good).expect("write");
        assert!(read_meta(&misnamed).is_err(), "the name must be an id");
        let huge = dir.path().join("abcdefghij12.meta");
        std::fs::write(&huge, format!("{good}{}", "x".repeat(20_000))).expect("write");
        assert!(read_meta(&huge).is_err(), "capped");
    }

    #[test]
    fn a_meta_file_that_is_a_link_is_not_followed() {
        let dir = tempfile::tempdir().expect("temp");
        let elsewhere = tempfile::tempdir().expect("temp");
        let real = elsewhere.path().join("abcdefghij12.meta");
        std::fs::write(&real, meta_text("abcdefghij12", 77, Path::new("/s/x.sock"), 5)).expect("write");
        let link = dir.path().join("abcdefghij12.meta");
        std::os::unix::fs::symlink(&real, &link).expect("link");
        assert!(read_meta(&link).is_err());
    }

    #[test]
    fn a_folder_that_is_not_there_has_no_hosts() {
        let dir = tempfile::tempdir().expect("temp");
        assert_eq!(scan(&dir.path().join("none")).expect("scans"), Scan::default());
    }

    #[test]
    fn live_hosts_are_listed_oldest_first_and_the_rest_reported_not_deleted() {
        // Short socket paths: a temp dir can be too long for sun_path on some systems.
        let dir = tempfile::Builder::new().prefix("hs").tempdir_in("/tmp").expect("temp");
        let d = dir.path();
        let _alive_new = host(d, "aaaaaaaaaaaa", 100, 2000, true);
        let _alive_old = host(d, "bbbbbbbbbbbb", 200, 1000, true);
        let _dead = host(d, "cccccccccccc", 300, 1500, true); // its pid is gone
        let _no_socket = host(d, "dddddddddddd", 100, 1600, false);
        std::fs::write(d.join("eeeeeeeeeeee.meta"), "garbage").expect("write");
        let _stray = UnixListener::bind(d.join("ffffffffffff.sock")).expect("binds");
        // Files that are not a host's are ignored.
        std::fs::write(d.join("notes.txt"), "x").expect("write");
        std::fs::write(d.join("SHORT.meta"), "x").expect("write");

        let scan = scan_with(d, runs(&[100, 200])).expect("scans");
        let live: Vec<&str> = scan.live.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(live, ["bbbbbbbbbbbb", "aaaaaaaaaaaa"]);
        let stale: Vec<(&str, &StaleReason)> =
            scan.stale.iter().map(|s| (s.id.as_str(), &s.reason)).collect();
        assert_eq!(stale.len(), 4, "{stale:?}");
        assert_eq!(stale[0], ("cccccccccccc", &StaleReason::PidGone));
        assert_eq!(stale[1], ("dddddddddddd", &StaleReason::SocketMissing));
        assert!(matches!(stale[2], ("eeeeeeeeeeee", StaleReason::Unreadable(_))));
        assert_eq!(stale[3], ("ffffffffffff", &StaleReason::NoMeta));
        assert_eq!(scan.stale[0].meta.as_ref().map(|m| m.pid), Some(300), "what it said is kept");
        // Nothing was removed.
        for name in ["cccccccccccc.meta", "cccccccccccc.sock", "eeeeeeeeeeee.meta", "ffffffffffff.sock"] {
            assert!(d.join(name).exists(), "{name}");
        }
    }

    #[test]
    fn a_socket_that_is_a_plain_file_does_not_count() {
        let dir = tempfile::tempdir().expect("temp");
        let socket = dir.path().join("abcdefghij12.sock");
        std::fs::write(&socket, "").expect("write");
        std::fs::write(dir.path().join("abcdefghij12.meta"), meta_text("abcdefghij12", 100, &socket, 1))
            .expect("meta");
        let scan = scan_with(dir.path(), runs(&[100])).expect("scans");
        assert!(scan.live.is_empty());
        assert_eq!(scan.stale[0].reason, StaleReason::SocketMissing);
    }

    #[test]
    fn the_real_pid_check_knows_this_process_and_a_gone_one() {
        assert!(pid_runs(std::process::id()));
        let mut child = std::process::Command::new("/usr/bin/true").spawn().expect("spawns");
        let pid = child.id();
        child.wait().expect("waits");
        assert!(!pid_runs(pid), "a reaped child is gone");
        assert!(!pid_runs(0));
        assert!(!pid_runs(u32::MAX));
    }
}
