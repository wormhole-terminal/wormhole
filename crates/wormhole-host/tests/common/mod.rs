//! The real host binary in a temp dir, and a client for it that does what the app will: connect
//! through the meta file, say `Hello`, acknowledge what it reads, keep a replica.
//!
//! Every host runs a small program (`/bin/cat`, `/bin/sh -c …`, `yes`), never a login shell, in
//! a scratch data dir with an empty environment besides what `Spawn` gives it. Each test starts
//! one host (one PTY), so the PTY count stays far from the system's limit.

#![allow(dead_code)] // each test file uses what it needs

use std::fs;
use std::io::{self, Read as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use wormhole_vt::{Line, Point, Terminal};
use wormhole_wire::io::{FrameReader, FrameWriter, RecvError};
use wormhole_wire::{
    self as wire, AppVersion, Attach, CHANNEL_LOCAL, DetachReason, EventKind, Features, Message, Role,
    StreamMode, StreamOrder,
};

pub const SIZE: wire::Size = wire::Size { cols: 80, rows: 24, cell_w: 128, cell_h: 256 };
/// How long any wait in these tests may take before it fails.
pub const PATIENCE: Duration = Duration::from_secs(10);

static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A 12-character id unique to this test process and call.
pub fn new_id() -> String {
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let seed = format!("{:06x}{:06x}", std::process::id() & 0xff_ffff, n & 0xff_ffff);
    seed.chars().map(|c| c.to_ascii_lowercase()).collect()
}

/// A host process and its files.
pub struct Host {
    pub data: tempfile::TempDir,
    pub hosts_dir: PathBuf,
    pub id: String,
    pub child: Child,
    /// The readiness pipe's read end.
    ready: Option<fs::File>,
}

impl Host {
    /// Start the host binary as the app does: its own session, an empty environment, the
    /// readiness pipe on descriptor 3.
    pub fn start() -> Self {
        let data = tempfile::tempdir().expect("a temp dir");
        let hosts_dir = data.path().join("hosts");
        Self::start_in(data, hosts_dir, new_id())
    }

    pub fn start_in(data: tempfile::TempDir, hosts_dir: PathBuf, id: String) -> Self {
        let (read, write) = pipe();
        let mut command = Command::new(env!("CARGO_BIN_EXE_wormhole-host"));
        command
            .args(["--id", &id, "--dir"])
            .arg(&hosts_dir)
            .env_clear()
            .env("WORMHOLE_LOG", "debug")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let fd = write.as_raw_fd();
        add_ready_fd(&mut command, fd);
        let child = command.spawn().expect("the host starts");
        drop(write);
        Self { data, hosts_dir, id, child, ready: Some(fs::File::from(read)) }
    }

    pub fn meta_path(&self) -> PathBuf {
        self.hosts_dir.join(format!("{}.meta", self.id))
    }

    pub fn log_path(&self) -> PathBuf {
        self.data.path().join("logs").join(format!("host-{}.log", self.id))
    }

    /// The meta file, once the host has written it (it listens by then).
    pub fn meta(&self) -> wormhole_term::hosted::meta::Meta {
        let until = Instant::now() + PATIENCE;
        loop {
            if let Ok(meta) = wormhole_term::hosted::meta::read(&self.meta_path()) {
                return meta;
            }
            assert!(Instant::now() < until, "no meta file appeared");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The readiness pipe's line, without its newline (blocks until the host says it or closes
    /// the pipe; the host says it within its spawn timeout either way).
    pub fn ready_line(&mut self) -> String {
        let mut line = Vec::new();
        if let Some(mut file) = self.ready.take() {
            let mut byte = [0u8; 1];
            while let Ok(1) = file.read(&mut byte) {
                if byte[0] == b'\n' {
                    break;
                }
                line.push(byte[0]);
            }
        }
        String::from_utf8_lossy(&line).into_owned()
    }

    /// A connected client that said `Hello` as `role` (and was welcomed, or refused: see its log).
    pub fn connect(&self, role: Role) -> Client {
        let mut client = self.connect_raw();
        client.hello(role, false);
        client
    }

    /// A connected client that has said nothing yet.
    pub fn connect_raw(&self) -> Client {
        let socket = self.meta().socket;
        let until = Instant::now() + PATIENCE;
        let stream = loop {
            match UnixStream::connect(&socket) {
                Ok(stream) => break stream,
                Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(5)),
                Err(err) => panic!("cannot connect: {err}"),
            }
        };
        Client::new(stream)
    }

    /// The controller that spawns `program` and attaches `Raw` from its first byte.
    pub fn spawn(&self, program: &str, args: &[&str]) -> (Client, u32) {
        let mut client = self.connect(Role::Controller);
        let pid = client.spawn(program, args);
        client.attach(StreamMode::Raw, 0);
        (client, pid)
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Wait for the host to exit; its status.
    pub fn wait_exit(&mut self) -> std::process::ExitStatus {
        let until = Instant::now() + PATIENCE;
        loop {
            if let Some(status) = self.child.try_wait().expect("waits") {
                return status;
            }
            assert!(Instant::now() < until, "the host did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().expect("waits").is_none()
    }

    /// The host's resident memory, from `ps`, in KiB.
    pub fn rss_kib(&self) -> u64 {
        let out = Command::new("/bin/ps").args(["-o", "rss=", "-p", &self.pid().to_string()]).output();
        out.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok()).unwrap_or(0)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // A test that failed half-way must not leave its host and program behind.
        if self.is_running() {
            signal(self.pid(), libc::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

/// Whether a process with this pid is still there.
pub fn alive(pid: u32) -> bool {
    signal(pid, 0)
}

/// Send `sig` to `pid`; whether it was delivered.
#[allow(unsafe_code)] // libc FFI; plain integers.
pub fn signal(pid: u32, sig: libc::c_int) -> bool {
    // SAFETY: kill takes two integers; the pid is one this test started.
    unsafe { libc::kill(pid as libc::pid_t, sig) == 0 }
}

/// Wait until `pid` is gone.
pub fn wait_gone(pid: u32) {
    let until = Instant::now() + PATIENCE;
    while alive(pid) {
        assert!(Instant::now() < until, "process {pid} is still there");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A pipe whose ends no other child inherits (tests run in parallel): only the `dup2` onto
/// descriptor 3 in the host's child carries the write end over.
#[allow(unsafe_code)] // libc FFI; two descriptors written into a local array.
fn pipe() -> (OwnedFd, OwnedFd) {
    let mut fds = [0; 2];
    // SAFETY: pipe writes two descriptors into the array; both are owned below and only here, and
    // fcntl only sets their close-on-exec flags.
    unsafe {
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "a pipe");
        for fd in fds {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
        (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1]))
    }
}

/// In the child, put `fd` on descriptor 3 and start a session of its own, as the app's
/// `posix_spawn` with `POSIX_SPAWN_SETSID` does.
#[allow(unsafe_code)] // `pre_exec` runs between fork and exec.
fn add_ready_fd(command: &mut Command, fd: libc::c_int) {
    // SAFETY: dup2 and setsid are async-signal-safe and allocate nothing; the closure touches
    // only the child's copy of the descriptor table.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(fd, 3) < 0 {
                return Err(io::Error::last_os_error());
            }
            libc::setsid();
            Ok(())
        });
    }
}

/// A frame as a test reads it.
#[derive(Clone, Debug, PartialEq)]
pub enum Got {
    Welcome(wire::Welcome),
    Info(wire::Welcome),
    Incompatible,
    Error(u8, u16, String),
    Detached(DetachReason),
    Pong(u64, u64),
    Out(u64, Vec<u8>),
    Repaint { epoch: u32, offset: u64, first: bool, last: bool, bytes: Vec<u8> },
    Resized(wire::ResizeApplied),
    Seeds { block: u32, link_seq: u64, image_handle: u64 },
    Exited(wire::ExitInfo),
    Event(u64, String),
    InputAck(u64),
    Counters(wire::Counters),
    Other(u8),
}

impl Got {
    fn of(msg: &Message<'_>) -> Self {
        match msg {
            Message::Welcome(w) => Self::Welcome(*w),
            Message::Info(w) => Self::Info(*w),
            Message::Incompatible(_) => Self::Incompatible,
            Message::Error(e) => Self::Error(e.ref_type, e.code, e.text.to_owned()),
            Message::Detached(r) => Self::Detached(*r),
            Message::Pong { n, pty_progress } => Self::Pong(*n, *pty_progress),
            Message::Out(o) => Self::Out(o.offset, o.bytes.to_vec()),
            Message::Repaint(r) => Self::Repaint {
                epoch: r.epoch,
                offset: r.offset,
                first: r.first,
                last: r.last,
                bytes: r.bytes.to_vec(),
            },
            Message::ResizeApplied(r) => Self::Resized(*r),
            Message::Event(e) => match e.kind {
                EventKind::RepaintSeeds { block, link_seq, image_handle } => {
                    Self::Seeds { block, link_seq, image_handle }
                }
                EventKind::Exited(x) => Self::Exited(x),
                kind => Self::Event(e.offset, format!("{kind:?}")),
            },
            Message::InputAck(n) => Self::InputAck(*n),
            Message::Counters(c) => Self::Counters(*c),
            other => Self::Other(other.ty()),
        }
    }
}

/// A client: the connection, a replica it keeps from the stream, and the order checker.
pub struct Client {
    reader: FrameReader<UnixStream>,
    writer: FrameWriter<UnixStream>,
    pub replica: Terminal,
    pub order: StreamOrder,
    /// Acknowledge each `Out` as it is read (a client that keeps up).
    pub auto_ack: bool,
    /// Every frame read so far.
    pub log: Vec<Got>,
    pub welcome: Option<wire::Welcome>,
}

impl Client {
    pub fn new(stream: UnixStream) -> Self {
        let write = stream.try_clone().expect("a second handle");
        Self {
            reader: FrameReader::new(stream),
            writer: FrameWriter::new(write),
            replica: Terminal::builder()
                .size(usize::from(SIZE.cols), usize::from(SIZE.rows))
                .scrollback(1000)
                .build(),
            order: StreamOrder::new(),
            auto_ack: true,
            log: Vec::new(),
            welcome: None,
        }
    }

    pub fn send(&mut self, msg: &Message<'_>) {
        self.writer.send(CHANNEL_LOCAL, 0, msg).expect("sends");
    }

    /// Read one frame within `timeout`; `None` on timeout or when the host closed the connection.
    pub fn read(&mut self, timeout: Duration) -> Option<Got> {
        self.reader.get_ref().set_read_timeout(Some(timeout.max(Duration::from_millis(1)))).ok()?;
        let got = match self.reader.read_frame() {
            Ok(Some(frame)) => Got::of(&frame.message),
            Ok(None) => return None,
            Err(RecvError::Io(err))
                if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
            {
                return None;
            }
            Err(err) => panic!("the host's stream broke: {err}"),
        };
        self.apply(&got);
        self.log.push(got.clone());
        Some(got)
    }

    /// Keep the replica and the order checker in step with what arrived.
    fn apply(&mut self, got: &Got) {
        match got {
            Got::Out(offset, bytes) => {
                self.order.out(&wire::Out { offset: *offset, bytes }).expect("Out in order");
                self.replica.feed(bytes);
                self.replica.clear_replies();
                if self.auto_ack {
                    let end = offset + bytes.len() as u64;
                    self.send(&Message::Ack(end));
                }
            }
            Got::Repaint { epoch, offset, first, last, bytes } => {
                let part = wire::Repaint {
                    epoch: *epoch,
                    offset: *offset,
                    kind: wire::RepaintKind::Full,
                    history_gap: false,
                    first: *first,
                    last: *last,
                    bytes,
                };
                self.order.repaint(&part).expect("Repaint in order");
                self.replica.feed(bytes);
                self.replica.clear_replies();
            }
            Got::Resized(applied) => {
                self.order.resize_applied(applied).expect("ResizeApplied in order");
                self.replica.resize(usize::from(applied.size.cols), usize::from(applied.size.rows));
            }
            Got::Seeds { block, link_seq, image_handle } => {
                let seeds = wormhole_vt::repaint::Seeds {
                    block: *block,
                    link_seq: *link_seq,
                    image_handle: *image_handle,
                };
                self.replica.seed(&seeds);
            }
            Got::Welcome(w) => self.welcome = Some(*w),
            _ => {}
        }
        let _ = self.replica.events().count();
    }

    /// Read until `pred` holds for a frame; that frame. Fails after [`PATIENCE`].
    pub fn until(&mut self, what: &str, mut pred: impl FnMut(&Got) -> bool) -> Got {
        let until = Instant::now() + PATIENCE;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "no {what} arrived; read: {:?}", self.summary());
            if let Some(got) = self.read(left.min(Duration::from_millis(100)))
                && pred(&got)
            {
                return got;
            }
        }
    }

    /// The first frame read so far for which `pred` holds, or the next one that comes.
    pub fn ever(&mut self, what: &str, mut pred: impl FnMut(&Got) -> bool) -> Got {
        match self.log.iter().find(|g| pred(g)) {
            Some(got) => got.clone(),
            None => self.until(what, pred),
        }
    }

    /// Read until the host closes the connection.
    pub fn until_closed(&mut self) {
        let until = Instant::now() + PATIENCE;
        loop {
            assert!(Instant::now() < until, "the host kept the connection open");
            // macOS refuses the option (EINVAL) once the peer has closed: what this waits for.
            let _ = self.reader.get_ref().set_read_timeout(Some(Duration::from_millis(100)));
            match self.reader.read_frame() {
                Ok(None) | Err(RecvError::UnexpectedEof) => return,
                Ok(Some(frame)) => {
                    let got = Got::of(&frame.message);
                    self.log.push(got);
                }
                Err(RecvError::Io(err))
                    if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
                Err(RecvError::Io(_)) => return,
                Err(err) => panic!("the host's stream broke: {err}"),
            }
        }
    }

    /// Read until the replica's screen shows `text` somewhere.
    pub fn until_text(&mut self, text: &str) {
        let until = Instant::now() + PATIENCE;
        while !self.screen_text().contains(text) {
            assert!(Instant::now() < until, "{text:?} never showed; the screen:\n{}", self.screen_text());
            self.read(Duration::from_millis(50));
        }
    }

    /// Read whatever comes within `quiet` of the last frame.
    pub fn drain(&mut self, quiet: Duration) {
        while self.read(quiet).is_some() {}
    }

    fn summary(&self) -> Vec<String> {
        self.log
            .iter()
            .rev()
            .take(12)
            .map(|g| match g {
                Got::Out(o, b) => format!("Out({o}, {} bytes)", b.len()),
                Got::Repaint { offset, bytes, .. } => format!("Repaint({offset}, {} bytes)", bytes.len()),
                other => format!("{other:?}"),
            })
            .collect()
    }

    pub fn hello(&mut self, role: Role, force: bool) -> Got {
        self.send(&Message::Hello(wire::Hello {
            proto_min: 1,
            proto_max: 1,
            proto_minor: 0,
            role,
            force,
            features: Features::NONE,
            client_version: AppVersion { major: 1, minor: 0, patch: 0 },
            client_name: "wormhole-host tests",
            ticket: None,
        }));
        self.until("Welcome", |g| matches!(g, Got::Welcome(_) | Got::Error(..) | Got::Incompatible))
    }

    /// Spawn `program`; its pid.
    pub fn spawn(&mut self, program: &str, args: &[&str]) -> u32 {
        let env = [("TERM", "xterm-256color"), ("PATH", "/usr/bin:/bin"), ("LANG", "en_US.UTF-8")];
        self.send(&Message::Spawn(wire::Spawn {
            size: SIZE,
            options: wire::Options { scrollback_lines: 1000, flags: 0 },
            program,
            cwd: "/",
            args: wire::List::Borrowed(args),
            env: wire::List::Borrowed(&env),
            colors: None,
        }));
        match self.until("the spawn's answer", |g| matches!(g, Got::Info(_) | Got::Error(..))) {
            Got::Info(info) => info.pid,
            other => panic!("spawn failed: {other:?}"),
        }
    }

    /// Attach: a new stream, so the order check starts over (its first frame sets the point).
    pub fn attach(&mut self, stream: StreamMode, window: u32) {
        self.order = StreamOrder::new();
        self.send(&Message::Attach(Attach { stream, history_lines: 20_000, window, resume_from: None }));
    }

    pub fn input(&mut self, bytes: &[u8]) {
        self.send(&Message::Input(bytes));
    }

    /// The replica's visible rows, trimmed, one per line.
    pub fn screen_text(&self) -> String {
        let rows = self.replica.rows() as i32;
        (0..rows).map(|row| self.replica.row_text(Line(row), true)).collect::<Vec<_>>().join("\n")
    }

    /// The replica's history and screen rows, and its cursor.
    pub fn picture(&self) -> (Vec<String>, Point) {
        let top = -(self.replica.history_len() as i32);
        let rows = self.replica.rows() as i32;
        let text = (top..rows).map(|row| self.replica.row_text(Line(row), true)).collect();
        (text, self.replica.cursor().point)
    }
}
