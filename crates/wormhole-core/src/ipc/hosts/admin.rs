//! Ending and forgetting session hosts from outside the app: what `Wormhole sessions end | forget`,
//! "Quit and End All Terminals" and devctl's `host-kill` do. None of it needs the app to run;
//! everything starts from the files in `<data dir>/hosts/` ([`scan`](super::scan)).
//!
//! **End.** The polite way is the protocol's: connect to the host's socket, say `Hello{role: Admin}`
//! (the one role every host version answers), check the `Welcome` names the host the meta file does,
//! and send `Kill`; the host hangs the shell up, removes its socket and meta and exits. A socket in
//! a shared folder (the long-path fallback) could be anyone's, so the peer's uid must be ours before
//! a byte is sent. When that fails (the socket is gone, the host is stuck, an older build), SIGTERM
//! to the meta's pid has the same effect, but only for a process that really is a `wormhole-host`:
//! a pid in a file is no proof, and pids are reused.
//!
//! **Forget.** Removes the files of a host that is not alive, and refuses one that is (or might be).

use std::io;
use std::os::unix::fs::FileTypeExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use wormhole_wire::io::{FrameReader, FrameWriter};
use wormhole_wire::{AppVersion, CHANNEL_LOCAL, Features, Hello, Message, Role, version};

use super::scan::{StaleReason, scan};
use super::{HostMeta, meta_path, read_meta};

/// How an ended host was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndHow {
    /// `Kill` over its socket, and the host hung up.
    Asked,
    /// SIGTERM to its pid (the socket could not be used).
    Signalled,
}

impl EndHow {
    pub fn word(self) -> &'static str {
        match self {
            Self::Asked => "ended",
            Self::Signalled => "ended (signalled)",
        }
    }
}

/// End the host `meta` describes: ask it over its socket, and signal its process when that does
/// not work. Waits at most `wait` for the host to answer and hang up.
pub fn end_host(meta: &HostMeta, wait: Duration) -> Result<EndHow, String> {
    let asked = ask_kill(meta, wait);
    let Err(why) = asked else { return Ok(EndHow::Asked) };
    tracing::info!(id = %meta.id, %why, "the host did not take Kill, signalling it");
    if !is_host_process(meta.pid) {
        return Err(format!("{why}; pid {} is not a wormhole-host, so it was left alone", meta.pid));
    }
    signal_term(meta.pid).map_err(|err| format!("{why}; SIGTERM to {}: {err}", meta.pid))?;
    Ok(EndHow::Signalled)
}

/// End every host in `hosts` at once (a few at a time; each has `wait`). The answers come back in
/// the order of `hosts`.
pub fn end_all(hosts: &[HostMeta], wait: Duration) -> Vec<(String, Result<EndHow, String>)> {
    /// Hosts are asked this many at a time: threads, not an async runtime, and a user has few.
    const AT_ONCE: usize = 16;
    let mut out = Vec::with_capacity(hosts.len());
    for chunk in hosts.chunks(AT_ONCE) {
        std::thread::scope(|scope| {
            let running: Vec<_> = chunk
                .iter()
                .map(|meta| (meta.id.clone(), scope.spawn(move || end_host(meta, wait))))
                .collect();
            for (id, handle) in running {
                let result = handle.join().unwrap_or_else(|_| Err("the worker stopped".into()));
                out.push((id, result));
            }
        });
    }
    out
}

/// End every live host in `hosts_dir`, as Quit and End All Terminals and `sessions end --all` do.
/// A folder that can't be read has none.
pub fn end_all_live(hosts_dir: &Path, wait: Duration) -> Vec<(String, Result<EndHow, String>)> {
    match scan(hosts_dir) {
        Ok(scanned) => end_all(&scanned.live, wait),
        Err(err) => {
            tracing::warn!(%err, "couldn't look for session hosts to end");
            Vec::new()
        }
    }
}

/// `Hello{Admin}`, check `Welcome`, `Kill`, wait for the hang-up.
fn ask_kill(meta: &HostMeta, wait: Duration) -> Result<(), String> {
    let deadline = Instant::now() + wait;
    let stream = UnixStream::connect(&meta.socket).map_err(|e| format!("connecting: {e}"))?;
    let left = || deadline.saturating_duration_since(Instant::now()).max(Duration::from_millis(1));
    stream.set_write_timeout(Some(left())).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(left())).map_err(|e| e.to_string())?;
    // The socket may sit in a folder others can write to; only our own host gets a word from us.
    if !crate::storage::fsutil::peer_is_this_user(&stream) {
        return Err("the socket's owner is not this user".into());
    }
    let mut writer = FrameWriter::new(&stream);
    let hello = Hello {
        proto_min: 1,
        proto_max: version::PROTO_MAJOR,
        proto_minor: version::PROTO_MINOR,
        role: Role::Admin,
        force: false,
        features: Features::NONE,
        client_version: app_version(),
        client_name: "wormhole sessions",
        ticket: None,
    };
    writer.send(CHANNEL_LOCAL, 0, &Message::Hello(hello)).map_err(|e| format!("hello: {e}"))?;
    let mut reader = FrameReader::new(&stream);
    match reader.read_frame().map_err(|e| format!("welcome: {e}"))? {
        Some(frame) => match frame.message {
            Message::Welcome(welcome) if welcome.session_id.as_slice() == meta.id.as_bytes() => {}
            Message::Welcome(_) => return Err("the socket belongs to another host".into()),
            Message::Incompatible(_) => return Err("the host speaks no protocol this build does".into()),
            _ => return Err("the host answered something unexpected".into()),
        },
        None => return Err("the host hung up".into()),
    }
    writer.send(CHANNEL_LOCAL, 0, &Message::Kill).map_err(|e| format!("kill: {e}"))?;
    // The host answers nothing to `Kill`: it ends, and its side of the connection closes.
    loop {
        let _ = stream.set_read_timeout(Some(left()));
        match reader.read_frame() {
            Ok(None) | Err(wormhole_wire::io::RecvError::UnexpectedEof) => return Ok(()),
            Ok(Some(_)) => {}
            Err(wormhole_wire::io::RecvError::Io(e))
                if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
            {
                return Err("the host did not hang up".into());
            }
            // A reset is a hang-up too.
            Err(_) => return Ok(()),
        }
    }
}

fn app_version() -> AppVersion {
    let mut parts = env!("CARGO_PKG_VERSION").split('.').map(|p| p.parse::<u16>().unwrap_or(0));
    AppVersion {
        major: parts.next().unwrap_or(0),
        minor: parts.next().unwrap_or(0),
        patch: parts.next().unwrap_or(0),
    }
}

#[allow(unsafe_code)]
fn signal_term(pid: u32) -> io::Result<()> {
    let pid = i32::try_from(pid).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid"))?;
    if pid <= 1 {
        // 0 and 1 would signal a process group or init; never from a file's say-so.
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a host's pid"));
    }
    // SAFETY: kill with a positive pid signals that one process; the pid was checked to be a host.
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Whether process `pid` runs the `wormhole-host` executable. Only its name is checked: a pid in a
/// file proves nothing, and a signal must not go to whatever reused the number. (To *talk* to a
/// host the meta and `Welcome` identify it; this guards the signal fallback.)
pub fn is_host_process(pid: u32) -> bool {
    process_path(pid)
        .and_then(|path| path.file_name().map(|n| n == crate::ipc::host_binary::HOST_BINARY_NAME))
        == Some(true)
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // libproc FFI; the buffer's size is passed along with it.
fn process_path(pid: u32) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt as _;
    let pid = i32::try_from(pid).ok().filter(|p| *p > 0)?;
    // PROC_PIDPATHINFO_MAXSIZE is 4 * MAXPATHLEN.
    let mut buf = vec![0u8; 4 * libc::PATH_MAX as usize];
    // SAFETY: `buf` is writable for the length given; proc_pidpath writes at most that many bytes.
    let len = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    let len = usize::try_from(len).ok().filter(|len| *len > 0)?;
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(&buf[..len.min(buf.len())])))
}

#[cfg(not(target_os = "macos"))]
fn process_path(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

/// What `forget` removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgetOutcome {
    pub removed: Vec<PathBuf>,
}

/// Why `forget` left a host's files alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgetError {
    /// Not a host id.
    BadId,
    /// No file of any host with that id.
    Unknown,
    /// The host is alive (or may be): end it instead.
    Live(String),
    Io(String),
}

impl std::fmt::Display for ForgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadId => f.write_str("not a session id (12 letters and digits)"),
            Self::Unknown => f.write_str("no such session"),
            Self::Live(why) => write!(f, "{why}; end it instead"),
            Self::Io(why) => f.write_str(why),
        }
    }
}

/// Remove the files of the stale host `id` in `hosts_dir`: its meta, and the socket the meta names
/// (or `<id>.sock`). A host that is alive keeps its files: a pid that runs, or a socket something
/// answers on, is a reason to refuse.
pub fn forget(hosts_dir: &Path, id: &str) -> Result<ForgetOutcome, ForgetError> {
    if !super::is_valid_host_id(id) {
        return Err(ForgetError::BadId);
    }
    let scanned = scan(hosts_dir).map_err(|e| ForgetError::Io(e.to_string()))?;
    if scanned.live.iter().any(|m| m.id == id) {
        return Err(ForgetError::Live(format!("session {id} is running")));
    }
    let Some(stale) = scanned.stale.into_iter().find(|s| s.id == id) else {
        return Err(ForgetError::Unknown);
    };
    let home_socket = hosts_dir.join(format!("{id}.sock"));
    match (&stale.reason, &stale.meta) {
        // The process runs, only its socket is missing (it binds it again within the hour).
        (StaleReason::SocketMissing, Some(meta)) => {
            return Err(ForgetError::Live(format!("session {id} runs on as pid {}", meta.pid)));
        }
        // Nothing says it is dead, and something may still answer.
        (StaleReason::Unreadable(_) | StaleReason::NoMeta, _) if answers(&home_socket) => {
            return Err(ForgetError::Live(format!("session {id} still answers on its socket")));
        }
        _ => {}
    }
    let mut targets = vec![meta_path(hosts_dir, id), home_socket];
    if let Some(meta) = &stale.meta {
        targets.push(meta.socket.clone());
    }
    let mut removed = Vec::new();
    for path in targets {
        if !removable(&path, id) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(ForgetError::Io(format!("{}: {err}", path.display()))),
        }
    }
    Ok(ForgetOutcome { removed })
}

/// Whether `path` is a file `forget` may remove for host `id`: its meta (any plain file or link,
/// removed as itself, never followed) or a socket named like the host's. The meta is hostile input:
/// the path it names is removed only when it is a socket with a host's file name.
fn removable(path: &Path, id: &str) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return false };
    if name == format!("{id}.meta") {
        return true;
    }
    let socket_name = name == format!("{id}.sock")
        || name.strip_prefix("wormhole-host-").is_some_and(|rest| rest.ends_with(&format!("-{id}.sock")));
    socket_name && meta.file_type().is_socket()
}

/// Whether something accepts connections on `path`.
fn answers(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// The meta file of host `id` in `hosts_dir`, checked.
pub fn find(hosts_dir: &Path, id: &str) -> Result<HostMeta, String> {
    if !super::is_valid_host_id(id) {
        return Err(format!("{id:?} is not a session id (12 letters and digits)"));
    }
    read_meta(&meta_path(hosts_dir, id)).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => format!("no session {id}"),
        _ => e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;

    fn write_meta(dir: &Path, id: &str, pid: u32, socket: &Path) {
        let text = format!(
            "format=1\nid={id}\npid={pid}\nstarted_at=1\nhost_version=0.3.0\nproto_min=1\nproto_max=1\n\
             socket={}\ndata_dir=/d\n",
            socket.display()
        );
        std::fs::write(dir.join(format!("{id}.meta")), text).expect("meta");
    }

    fn short_dir() -> tempfile::TempDir {
        // Short paths: a temp dir can be too long for sun_path.
        tempfile::Builder::new().prefix("ha").tempdir_in("/tmp").expect("temp")
    }

    /// A pid that certainly ran and is gone.
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("/usr/bin/true").spawn().expect("spawns");
        let pid = child.id();
        child.wait().expect("waits");
        pid
    }

    #[test]
    fn this_process_is_not_a_host_and_a_gone_one_is_not_either() {
        assert!(!is_host_process(std::process::id()), "the test binary is not wormhole-host");
        assert!(!is_host_process(dead_pid()));
        assert!(!is_host_process(0));
        assert!(!is_host_process(u32::MAX));
    }

    #[test]
    fn a_stale_host_is_forgotten_with_its_meta_and_socket() {
        let dir = short_dir();
        let d = dir.path();
        let sock = d.join("aaaaaaaaaaaa.sock");
        drop(UnixListener::bind(&sock).expect("binds")); // the file stays after the listener is gone
        write_meta(d, "aaaaaaaaaaaa", dead_pid(), &sock);
        std::fs::write(d.join("other.txt"), "x").expect("write");
        let outcome = forget(d, "aaaaaaaaaaaa").expect("forgets");
        assert_eq!(outcome.removed, [d.join("aaaaaaaaaaaa.meta"), sock.clone()]);
        assert!(!sock.exists() && !d.join("aaaaaaaaaaaa.meta").exists());
        assert!(d.join("other.txt").exists(), "only the host's files");
        assert_eq!(forget(d, "aaaaaaaaaaaa"), Err(ForgetError::Unknown));
    }

    #[test]
    fn a_live_host_is_not_forgotten() {
        let dir = short_dir();
        let d = dir.path();
        let sock = d.join("bbbbbbbbbbbb.sock");
        let _listener = UnixListener::bind(&sock).expect("binds");
        // This test process stands in for the host: its pid runs.
        write_meta(d, "bbbbbbbbbbbb", std::process::id(), &sock);
        assert!(matches!(forget(d, "bbbbbbbbbbbb"), Err(ForgetError::Live(_))));
        assert!(sock.exists() && d.join("bbbbbbbbbbbb.meta").exists());
    }

    #[test]
    fn a_host_whose_socket_is_missing_but_whose_pid_runs_is_kept() {
        let dir = short_dir();
        let d = dir.path();
        write_meta(d, "cccccccccccc", std::process::id(), &d.join("cccccccccccc.sock"));
        assert!(matches!(forget(d, "cccccccccccc"), Err(ForgetError::Live(_))));
        assert!(d.join("cccccccccccc.meta").exists());
    }

    #[test]
    fn an_unreadable_meta_with_a_listening_socket_is_kept_and_without_one_removed() {
        let dir = short_dir();
        let d = dir.path();
        std::fs::write(d.join("dddddddddddd.meta"), "format=9\n").expect("write");
        let listener = UnixListener::bind(d.join("dddddddddddd.sock")).expect("binds");
        assert!(matches!(forget(d, "dddddddddddd"), Err(ForgetError::Live(_))));
        drop(listener);
        let outcome = forget(d, "dddddddddddd").expect("forgets");
        assert_eq!(outcome.removed.len(), 2, "{outcome:?}");
    }

    #[test]
    fn a_socket_without_a_meta_is_forgotten() {
        let dir = short_dir();
        let d = dir.path();
        drop(UnixListener::bind(d.join("eeeeeeeeeeee.sock")).expect("binds"));
        assert_eq!(forget(d, "eeeeeeeeeeee").expect("forgets").removed.len(), 1);
    }

    #[test]
    fn forget_removes_only_sockets_a_meta_names_with_a_hosts_file_name() {
        let dir = short_dir();
        let d = dir.path();
        let elsewhere = short_dir();
        let precious = elsewhere.path().join("notes.txt");
        std::fs::write(&precious, "keep").expect("write");
        // A meta pointing at an ordinary file: removed meta, untouched file.
        write_meta(d, "ffffffffffff", dead_pid(), &precious);
        let outcome = forget(d, "ffffffffffff").expect("forgets");
        assert_eq!(outcome.removed, [d.join("ffffffffffff.meta")]);
        assert!(precious.exists());
        // A real socket with another name is not a host's either.
        let odd = elsewhere.path().join("my.sock");
        drop(UnixListener::bind(&odd).expect("binds"));
        write_meta(d, "gggggggggggg", dead_pid(), &odd);
        forget(d, "gggggggggggg").expect("forgets");
        assert!(odd.exists());
    }

    #[test]
    fn forget_checks_the_id_and_knows_what_is_not_there() {
        let dir = short_dir();
        assert_eq!(forget(dir.path(), "../etc"), Err(ForgetError::BadId));
        assert_eq!(forget(dir.path(), "hhhhhhhhhhhh"), Err(ForgetError::Unknown));
        assert_eq!(forget(&dir.path().join("none"), "hhhhhhhhhhhh"), Err(ForgetError::Unknown));
    }

    #[test]
    fn find_reads_the_meta_of_one_host() {
        let dir = short_dir();
        let d = dir.path();
        write_meta(d, "iiiiiiiiiiii", 7, &d.join("iiiiiiiiiiii.sock"));
        assert_eq!(find(d, "iiiiiiiiiiii").expect("reads").pid, 7);
        assert!(find(d, "jjjjjjjjjjjj").expect_err("none").contains("no session"));
        assert!(find(d, "x").expect_err("bad").contains("not a session id"));
    }

    /// A stand-in host: says `Welcome` to `Hello`, and ends on `Kill` by closing the connection.
    fn serve_one(
        listener: UnixListener,
        id: &'static str,
        answer_kill: bool,
    ) -> std::thread::JoinHandle<bool> {
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else { return false };
            let mut reader = FrameReader::new(&stream);
            let mut writer = FrameWriter::new(&stream);
            let hello_role = match reader.read_frame() {
                Ok(Some(frame)) => match frame.message {
                    Message::Hello(hello) => hello.role,
                    _ => return false,
                },
                _ => return false,
            };
            if hello_role != Role::Admin {
                return false;
            }
            let mut name = [0u8; 12];
            name.copy_from_slice(id.as_bytes());
            let welcome = wormhole_wire::Welcome {
                proto: 1,
                proto_minor: 0,
                state: wormhole_wire::HostState::Running,
                host_version: AppVersion { major: 1, minor: 0, patch: 0 },
                pid: 0,
                session_id: name,
                features: Features::NONE,
                started_at_ms: 0,
                first_block_id: 0,
                size: wormhole_wire::Size { cols: 80, rows: 24, cell_w: 8, cell_h: 16 },
                counters: wormhole_wire::Counters {
                    epoch: 0,
                    history_lines: 0,
                    trimmed_total: 0,
                    screen_rows: 24,
                    alt_screen: false,
                    revision: 0,
                },
                exit: None,
                token: None,
            };
            if writer.send(CHANNEL_LOCAL, 0, &Message::Welcome(welcome)).is_err() {
                return false;
            }
            let killed = matches!(reader.read_frame(), Ok(Some(f)) if matches!(f.message, Message::Kill));
            if killed && !answer_kill {
                std::thread::sleep(Duration::from_millis(600)); // never hangs up in time
            }
            killed
        })
    }

    fn meta_for(id: &str, socket: &Path) -> HostMeta {
        HostMeta {
            id: id.into(),
            pid: dead_pid(),
            child_pid: None,
            started_at: 1,
            host_version: "1.0.0".into(),
            proto_min: 1,
            proto_max: 1,
            socket: socket.to_path_buf(),
            data_dir: PathBuf::from("/d"),
        }
    }

    #[test]
    fn end_sends_kill_to_a_host_that_names_itself_and_waits_for_the_hang_up() {
        let dir = short_dir();
        let sock = dir.path().join("kkkkkkkkkkkk.sock");
        let server = serve_one(UnixListener::bind(&sock).expect("binds"), "kkkkkkkkkkkk", true);
        let how = end_host(&meta_for("kkkkkkkkkkkk", &sock), Duration::from_secs(5)).expect("ends");
        assert_eq!(how, EndHow::Asked);
        assert!(server.join().expect("joins"), "the host got Kill");
    }

    #[test]
    fn end_does_not_trust_a_socket_that_names_another_host() {
        let dir = short_dir();
        let sock = dir.path().join("llllllllllll.sock");
        let server = serve_one(UnixListener::bind(&sock).expect("binds"), "mmmmmmmmmmmm", true);
        // The pid is gone, so there is no signal fallback either.
        let err = end_host(&meta_for("llllllllllll", &sock), Duration::from_secs(5)).expect_err("refused");
        assert!(err.contains("another host"), "{err}");
        assert!(!server.join().expect("joins"), "no Kill was sent");
    }

    #[test]
    fn end_gives_up_on_a_host_that_never_hangs_up_and_on_a_missing_socket() {
        let dir = short_dir();
        let sock = dir.path().join("nnnnnnnnnnnn.sock");
        let server = serve_one(UnixListener::bind(&sock).expect("binds"), "nnnnnnnnnnnn", false);
        let err =
            end_host(&meta_for("nnnnnnnnnnnn", &sock), Duration::from_millis(250)).expect_err("times out");
        assert!(err.contains("did not hang up"), "{err}");
        server.join().expect("joins");
        let err = end_host(&meta_for("oooooooooooo", &dir.path().join("none.sock")), Duration::from_secs(1))
            .expect_err("no socket");
        assert!(err.contains("connecting") && err.contains("not a wormhole-host"), "{err}");
    }

    #[test]
    fn end_all_answers_for_each_host_in_order() {
        let dir = short_dir();
        let d = dir.path();
        let (a, b) = (d.join("pppppppppppp.sock"), d.join("qqqqqqqqqqqq.sock"));
        let first = serve_one(UnixListener::bind(&a).expect("binds"), "pppppppppppp", true);
        let metas = [meta_for("pppppppppppp", &a), meta_for("qqqqqqqqqqqq", &b)];
        let answers = end_all(&metas, Duration::from_secs(5));
        assert_eq!(answers.len(), 2);
        assert_eq!(answers[0].0, "pppppppppppp");
        assert_eq!(answers[0].1, Ok(EndHow::Asked));
        assert_eq!(answers[1].0, "qqqqqqqqqqqq");
        assert!(answers[1].1.is_err(), "no socket");
        first.join().expect("joins");
    }
}
