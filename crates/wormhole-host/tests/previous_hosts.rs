//! The current client against the session hosts of earlier releases. A host outlives the app, so
//! after an update the app of today talks to a host built months ago; this drives each one with this
//! build's client code: start, echo, detach and attach again with a repaint, Clear Buffer (gated on
//! the host's protocol minor), a frame of a type it never heard of, a frame over 64 KiB, and `Kill`.
//!
//! It does nothing unless `WORMHOLE_PREVIOUS_HOSTS=<dir>` names a folder of `<version>/wormhole-host`
//! binaries (each re-signed ad hoc so it runs from there: `scripts/previous-hosts.sh` makes it from
//! the last releases' app archives). Without it the test says so and passes, so `cargo test` stays
//! offline and fast.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use support::{PATIENCE, alive, new_id, wait_gone};
use wormhole_term::client::{self, HostCommand};
use wormhole_term::types::Line;
use wormhole_term::{GridSize, Launch, Pty, PtyEvents, Shell, StartOptions};
use wormhole_wire::io::{FrameReader, FrameWriter};
use wormhole_wire::{self as wire, AppVersion, CHANNEL_LOCAL, Features, Message, Role};

const GRID: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

/// The hosts to drive: `(version, binary)` for each `<dir>/<version>/wormhole-host`, oldest first.
fn previous_hosts() -> Option<Vec<(String, PathBuf)>> {
    let dir = PathBuf::from(std::env::var_os("WORMHOLE_PREVIOUS_HOSTS")?);
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("WORMHOLE_PREVIOUS_HOSTS={}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path().join("wormhole-host")))
        .filter(|(_, binary)| binary.is_file())
        .collect();
    found.sort();
    Some(found)
}

fn options() -> StartOptions<'static> {
    StartOptions { scrollback_lines: 1000, ..StartOptions::default() }
}

fn launch() -> Launch {
    // Prints a landmark in 200 lines (so the first scrolls into history), then echoes input.
    let script = "i=1; while [ $i -le 200 ]; do echo scrollmark-$i; i=$((i+1)); done; exec cat";
    let shell = Shell::new("/bin/sh".into(), vec!["-c".into(), script.into()]);
    let env = [("TERM", "xterm-256color"), ("PATH", "/usr/bin:/bin"), ("LANG", "en_US.UTF-8")]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Launch { shell, cwd: Some("/".into()), env }
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn history(pty: &Pty) -> String {
    pty.lines_text(Line(-400), Line(-1))
}

/// SIGKILLs the host when dropped, so a failed assertion does not leave it (and its PTY) running.
struct Reaper(PathBuf, String);

impl Drop for Reaper {
    fn drop(&mut self) {
        let meta = wormhole_term::hosted::paths::meta_path(&self.0, &self.1);
        if let Ok(meta) = wormhole_term::hosted::meta::read(&meta) {
            support::signal(meta.pid, libc::SIGKILL);
        }
    }
}

/// What a raw connection hears, of the few messages these tests care about.
enum Heard {
    Welcome,
    Pong(u64),
    /// Anything else, or nothing before the host closed the connection or time ran out.
    Other,
}

/// A raw connection to the host, for what the client never sends.
struct Raw {
    /// The same socket as `writer`, for bytes that are not a frame this build can encode.
    plain: UnixStream,
    writer: FrameWriter<UnixStream>,
    reader: FrameReader<UnixStream>,
}

impl Raw {
    fn connect(socket: &Path, role: Role) -> Self {
        let stream = UnixStream::connect(socket).expect("connects");
        stream.set_read_timeout(Some(Duration::from_secs(3))).expect("timeout");
        let mut raw = Self {
            plain: stream.try_clone().expect("clone"),
            writer: FrameWriter::new(stream.try_clone().expect("clone")),
            reader: FrameReader::new(stream),
        };
        raw.send(&Message::Hello(wire::Hello {
            proto_min: 1,
            proto_max: 1,
            proto_minor: 1,
            role,
            force: false,
            features: Features::NONE,
            client_version: AppVersion::default(),
            client_name: "previous-hosts",
            ticket: None,
        }));
        assert!(matches!(raw.next(), Heard::Welcome), "a Welcome");
        raw
    }

    fn send(&mut self, msg: &Message<'_>) {
        self.writer.send(CHANNEL_LOCAL, 0, msg).expect("sends");
    }

    /// The type of the next message, if the host sends one before closing or timing out.
    fn next(&mut self) -> Heard {
        match self.reader.read_frame() {
            Ok(Some(frame)) => match frame.message {
                Message::Welcome(_) => Heard::Welcome,
                Message::Pong { n, .. } => Heard::Pong(n),
                _ => Heard::Other,
            },
            _ => Heard::Other,
        }
    }

    /// Bytes as they are, for a frame of a kind this build never writes.
    fn raw(&mut self, bytes: &[u8]) -> bool {
        self.plain.write_all(bytes).is_ok()
    }
}

/// One frame of an unknown type with `len` bytes of payload.
fn unknown_frame(len: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + len);
    bytes.extend_from_slice(&u32::try_from(len).expect("a length").to_le_bytes());
    bytes.extend_from_slice(&[0x77, 0, 1, 0]); // an unknown type, no flags, the local channel
    bytes.extend(std::iter::repeat_n(7u8, len));
    bytes
}

fn drive(version: &str, binary: &Path) {
    let data = tempfile::tempdir().expect("a temp dir");
    let hosts_dir = data.path().join("hosts");
    let id = new_id();
    let _reaper = Reaper(hosts_dir.clone(), id.clone());
    let command = HostCommand { binary, hosts_dir: &hosts_dir, id: &id };
    let (pty, _events): (Pty, PtyEvents) = Pty::start_hosted(&command, GRID, &options(), launch())
        .unwrap_or_else(|e| panic!("{version}: could not start the host: {e:#}"));
    let minor = pty.host_minor().expect("a hosted terminal has a host minor");
    eprintln!("{version}: host protocol minor {minor}");

    // Echo: what is typed comes back.
    wait_for("the landmarks", || pty.screen_text().contains("scrollmark-200"));
    pty.write(&b"hello-from-the-client\n"[..]);
    wait_for("the echo", || pty.screen_text().contains("hello-from-the-client"));
    assert!(history(&pty).contains("scrollmark-1\n"), "{version}: the first landmark is in the history");

    // Detach, and attach again on screen: the repaint shows the same screen and history.
    let socket = client::socket_of(&hosts_dir, &id).expect("the socket");
    assert!(pty.detach());
    drop(pty);
    let (pty, _events) = Pty::attach_hosted(&socket, &id, &options(), true)
        .unwrap_or_else(|e| panic!("{version}: could not attach again: {e}"));
    wait_for("the repaint", || pty.screen_text().contains("hello-from-the-client"));
    wait_for("the history", || history(&pty).contains("scrollmark-1\n"));

    // Clear Buffer: a host of minor 2 or later forgets its history, an older one skips the message
    // and gives it back at the next attach.
    assert_eq!(pty.host_forgets_history(), Some(minor >= 2), "{version}: what the client believes");
    pty.clear_history();
    assert!(pty.detach());
    drop(pty);
    let (pty, _events) = Pty::attach_hosted(&socket, &id, &options(), true).expect("attached again");
    wait_for("the repaint", || pty.screen_text().contains("hello-from-the-client"));
    std::thread::sleep(Duration::from_millis(300));
    let kept = history(&pty).contains("scrollmark-1\n");
    assert_eq!(
        kept,
        minor < 2,
        "{version}: history after Clear Buffer (protocol minor {minor}), kept: {kept}"
    );
    assert!(pty.detach());
    drop(pty);

    // A frame of a type it never heard of is skipped, and the connection goes on.
    let mut raw = Raw::connect(&socket, Role::Observer);
    assert!(raw.raw(&unknown_frame(16)));
    raw.send(&Message::Ping(7));
    assert!(matches!(raw.next(), Heard::Pong(7)), "{version}: answers a ping after an unknown frame");

    // A frame over 64 KiB: hosts before the fix drop the connection (the limit of the handshake
    // stayed); either way the host itself stays up for the next connection. Documented, not required.
    let mut big = Raw::connect(&socket, Role::Observer);
    let sent = big.raw(&unknown_frame(100 * 1024));
    let kept = sent && {
        big.send(&Message::Ping(8));
        matches!(big.next(), Heard::Pong(8))
    };
    eprintln!("{version}: a frame over 64 KiB {}", if kept { "is taken" } else { "ends the connection" });
    let again = Raw::connect(&socket, Role::Observer);
    drop(again);

    // Kill over an admin connection ends the host and removes its files.
    let pid = wormhole_term::hosted::meta::read(&wormhole_term::hosted::paths::meta_path(&hosts_dir, &id))
        .expect("the meta file")
        .pid;
    let mut admin = Raw::connect(&socket, Role::Admin);
    admin.send(&Message::Kill);
    wait_gone(pid);
    assert!(!alive(pid), "{version}: the host ended");
}

#[test]
fn the_current_client_drives_the_hosts_of_earlier_releases() {
    let Some(hosts) = previous_hosts() else {
        eprintln!(
            "WORMHOLE_PREVIOUS_HOSTS is not set: no previous hosts to drive (scripts/previous-hosts.sh)"
        );
        return;
    };
    assert!(!hosts.is_empty(), "WORMHOLE_PREVIOUS_HOSTS holds no <version>/wormhole-host");
    for (version, binary) in &hosts {
        eprintln!("== {version}");
        drive(version, binary);
    }
}
