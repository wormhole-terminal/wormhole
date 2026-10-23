//! Which session hosts are alive, from the files they leave in `<data dir>/hosts/`.
//!
//! A host is live when its meta file reads, its pid runs and its socket exists. A pid alone proves
//! nothing (pids are reused) and neither does a file, so a caller that is about to talk to a host
//! still checks it is the right one with the protocol's `Welcome`; this only narrows the list and
//! tells the stale files apart. Nothing here deletes anything.

use std::io;
use std::path::Path;

use wormhole_term::hosted::meta::Meta;

use crate::platform::{fs, process};
use crate::storage::files::{self, LinkPolicy};

use super::is_valid_host_id;

/// A meta file is a dozen short lines; anything bigger is not one.
const META_CAP: u64 = wormhole_term::hosted::meta::READ_MAX;

/// What a host says about itself in `<id>.meta`: the one type the host, the app and this crate
/// share (`wormhole_term::hosted::meta`).
pub type HostMeta = Meta;

/// Read and check one meta file.
///
/// The file is hostile input as far as its size and content go (any process of the user can write
/// to the folder): it is read capped and without following links, then parsed by
/// [`Meta::parse`] (the format's one reader), and on top of that the id inside must equal the
/// file name's, the pid must be one `kill` can address and the socket path absolute.
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
    let text = files::read_to_string_capped(path, META_CAP, LinkPolicy::Within(root))?;
    let mut meta = Meta::parse(&text).map_err(|err| bad(&err.to_string()))?;
    // 0 is "not known yet" to every reader, as it is on the wire.
    meta.child_pid = meta.child_pid.filter(|pid| *pid > 0);
    if meta.id != stem {
        return Err(bad("id does not match the file name"));
    }
    if meta.pid == 0 || i32::try_from(meta.pid).is_err() {
        return Err(bad("pid"));
    }
    if !meta.socket.is_absolute() {
        return Err(bad("socket"));
    }
    Ok(meta)
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
    scan_with(hosts_dir, process::exists)
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

/// Whether `path` is a socket itself (a link, or a file of another kind, is not).
fn is_socket(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| fs::is_socket(&m.file_type()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::socket::Listener;

    fn meta_text(id: &str, pid: u32, socket: &Path, started_at: u64) -> String {
        format!(
            "format=1\nid={id}\npid={pid}\nchild_pid=4242\nstarted_at={started_at}\nhost_version=0.3.0\n\
             proto_min=1\nproto_max=2\nsocket={}\ndata_dir=/d\nfuture_key=whatever\n",
            socket.display()
        )
    }

    /// A host's files: its meta, and a socket when `listening`. Returns the listener to keep it open.
    fn host(dir: &Path, id: &str, pid: u32, started_at: u64, listening: bool) -> Option<Listener> {
        let socket = dir.join(format!("{id}.sock"));
        std::fs::write(dir.join(format!("{id}.meta")), meta_text(id, pid, &socket, started_at))
            .expect("meta");
        listening.then(|| Listener::bind(&socket).expect("binds"))
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
        assert_eq!((meta.proto_min, meta.proto_max), (1_u16, 2_u16));
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
    fn a_child_pid_of_zero_is_not_known() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("abcdefghij12.meta");
        let text =
            meta_text("abcdefghij12", 77, Path::new("/s/x.sock"), 5).replace("child_pid=4242", "child_pid=0");
        std::fs::write(&path, text).expect("write");
        assert_eq!(read_meta(&path).expect("reads").child_pid, None);
    }

    #[test]
    fn a_meta_file_that_is_a_link_is_not_followed() {
        let dir = tempfile::tempdir().expect("temp");
        let elsewhere = tempfile::tempdir().expect("temp");
        let real = elsewhere.path().join("abcdefghij12.meta");
        std::fs::write(&real, meta_text("abcdefghij12", 77, Path::new("/s/x.sock"), 5)).expect("write");
        let link = dir.path().join("abcdefghij12.meta");
        crate::platform::fs::symlink(&real, &link).expect("link");
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
        let _stray = Listener::bind(d.join("ffffffffffff.sock")).expect("binds");
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
        assert!(process::exists(std::process::id()));
        // Not ours to signal (EPERM), and still taken.
        assert!(process::exists(1));
        let mut child = std::process::Command::new("/usr/bin/true").spawn().expect("spawns");
        let pid = child.id();
        child.wait().expect("waits");
        assert!(!process::exists(pid), "a reaped child is gone");
        assert!(!process::exists(0));
        assert!(!process::exists(u32::MAX));
    }
}
