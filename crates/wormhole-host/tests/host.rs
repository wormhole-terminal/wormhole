//! The host binary with a real PTY and real processes (`session-host.md`, section 12,
//! "Integration"): starting and the readiness pipe, the stream, detach and attach, resizes, the
//! ways a host ends, and the clients that misbehave.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::*;
use wormhole_wire::{self as wire, CHANNEL_LOCAL, DetachReason, HostState, Message, Role, StreamMode};

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).expect("exists").permissions().mode() & 0o777
}

#[test]
fn a_host_starts_listens_privately_and_says_when_the_program_runs() {
    let mut host = Host::start();
    let (client, pid) = host.spawn("/bin/cat", &[]);
    assert!(pid > 0);
    assert_eq!(host.ready_line(), format!("ok {pid}"));
    let meta = host.meta();
    assert_eq!((meta.id.as_str(), meta.pid, meta.child_pid), (host.id.as_str(), host.pid(), Some(pid)));
    assert_eq!(meta.socket, host.hosts_dir.join(format!("{}.sock", host.id)));
    assert_eq!(meta.data_dir, host.data.path());
    assert_eq!(mode(&host.hosts_dir), 0o700);
    assert_eq!(mode(&host.meta_path()), 0o600);
    assert_eq!(mode(&meta.socket), 0o600);
    assert!(alive(pid));
    let welcome = client.welcome.expect("a welcome");
    assert_eq!(&welcome.session_id, host.id.as_bytes());
    assert!(host.log_path().exists(), "it logs to the data dir's logs");
}

#[test]
fn raw_output_echoes_what_is_typed_and_input_is_acknowledged() {
    let host = Host::start();
    let (mut client, _) = host.spawn("/bin/cat", &[]);
    client.input(b"hello host\r");
    client.until_text("hello host");
    client.ever("InputAck", |g| matches!(g, Got::InputAck(n) if *n >= 11));
    client.send(&Message::Ping(9));
    assert!(
        matches!(client.until("Pong", |g| matches!(g, Got::Pong(..))), Got::Pong(9, progress) if progress > 0)
    );
}

#[test]
fn the_host_answers_a_programs_scheme_query_with_the_scheme_the_app_set() {
    let host = Host::start();
    let (mut client, _) = host.spawn("/bin/cat", &[]);
    client.send(&Message::SetColorScheme(wire::ColorScheme::Light));
    // cat prints the query, the host's engine answers it into the PTY, and the terminal's echo
    // shows the answer (as `^[[?997;2n`, or the bytes themselves without ECHOCTL).
    client.input(b"\x1b[?996n\r");
    client.ever(
        "the light scheme's report",
        |g| matches!(g, Got::Out(_, bytes) if bytes.windows(8).any(|w| w == b"[?997;2n")),
    );
}

#[test]
fn an_attach_after_a_detach_repaints_the_screen_the_first_client_had() {
    let host = Host::start();
    let (mut first, _) = host.spawn("/bin/cat", &[]);
    // Enough to scroll into the history, with a wide character and some color.
    for i in 0..40 {
        first.input(format!("line {i} \u{6f22} \x1b[31mred\x1b[0m\r").as_bytes());
    }
    first.until_text("line 39");
    first.drain(Duration::from_millis(300));
    let picture = first.picture();
    first.send(&Message::Detach);
    first.until_closed();

    let mut second = host.connect(Role::Controller);
    second.attach(StreamMode::Sync, 0);
    second.until("the repaint's last part", |g| matches!(g, Got::Repaint { last: true, .. }));
    let seeds = second.log.iter().position(|g| matches!(g, Got::Seeds { .. }));
    let size = second.log.iter().position(|g| matches!(g, Got::Resized(_)));
    let repaint = second.log.iter().position(|g| matches!(g, Got::Repaint { first: true, .. }));
    assert!(size < seeds && seeds < repaint, "size, seeds, then the repaint: {:?}", (size, seeds, repaint));
    assert_eq!(second.picture(), picture, "the repaint is the screen the first client had");
    second.input(b"after the attach\r");
    second.until_text("after the attach");
}

#[test]
fn clearing_the_history_empties_the_hosts_so_an_attach_does_not_bring_it_back() {
    let host = Host::start();
    let (mut first, _) = host.spawn("/bin/cat", &[]);
    for i in 0..40 {
        first.input(format!("line {i}\r").as_bytes());
    }
    first.until_text("line 39");
    first.drain(Duration::from_millis(300));
    assert!(first.replica.history_len() > 0, "enough output to scroll");
    // The Ping's Pong says the host has handled the ClearHistory before it.
    first.send(&Message::ClearHistory);
    first.send(&Message::Ping(1));
    first.until("Pong", |g| matches!(g, Got::Pong(1, _)));
    first.send(&Message::Detach);
    first.until_closed();

    let mut second = host.connect(Role::Controller);
    second.attach(StreamMode::Sync, 0);
    second.until("the repaint's last part", |g| matches!(g, Got::Repaint { last: true, .. }));
    assert_eq!(second.replica.history_len(), 0, "the host's history is gone");
    assert!(second.screen_text().contains("line 39"), "the screen stays");
}

#[test]
fn a_resize_is_in_the_stream_where_it_was_applied() {
    let host = Host::start();
    let (mut client, _) = host.spawn("/bin/sh", &["-c", "while read line; do stty size; done"]);
    client.input(b"\r");
    client.until_text("24 80");
    let size = wire::Size { cols: 100, rows: 30, ..SIZE };
    client.send(&Message::Resize(wire::Resize { req: 7, size }));
    let resized = client.until("ResizeApplied", |g| matches!(g, Got::Resized(r) if r.req == 7));
    let Got::Resized(applied) = resized else { unreachable!() };
    assert_eq!(applied.size, size);
    assert_eq!((client.replica.cols(), client.replica.rows()), (100, 30));
    client.input(b"\r");
    client.until_text("30 100");
    // A storm: the host applies the latest of what came within its 8 ms.
    for cols in 81..=90 {
        client.send(&Message::Resize(wire::Resize {
            req: 100 + u32::from(cols),
            size: wire::Size { cols, ..size },
        }));
    }
    client.until("the storm's last size", |g| matches!(g, Got::Resized(r) if r.size.cols == 90));
    client.input(b"\r");
    client.until_text("30 90");
    // Every Out, Repaint and ResizeApplied passed the order check on the way (`Client::apply`).
}

#[test]
fn close_ends_the_program_and_the_host_with_its_files() {
    let mut host = Host::start();
    let (mut client, pid) = host.spawn("/bin/cat", &[]);
    let socket = host.meta().socket;
    client.send(&Message::Close { grace_ms: 1000 });
    client.until_closed();
    assert!(client.log.contains(&Got::Detached(DetachReason::HostClosing)), "{:?}", client.log);
    assert!(host.wait_exit().success());
    wait_gone(pid);
    assert!(!socket.exists() && !host.meta_path().exists(), "the socket and the meta are gone");
}

#[test]
fn sigterm_is_a_close() {
    let mut host = Host::start();
    let (_client, pid) = host.spawn("/bin/cat", &[]);
    let socket = host.meta().socket;
    assert!(signal(host.pid(), libc::SIGTERM));
    assert!(host.wait_exit().success());
    wait_gone(pid);
    assert!(!socket.exists() && !host.meta_path().exists());
}

/// A client that sends `Hello` and retries while the host is still letting go of the last ones.
fn connect_retrying(host: &Host, role: Role) -> Client {
    let until = Instant::now() + PATIENCE;
    loop {
        let mut client = host.connect_raw();
        match client.hello(role, false) {
            Got::Welcome(_) => return client,
            other => assert!(Instant::now() < until, "never welcomed: {other:?}"),
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_client_killed_mid_frame_leaves_the_host_serving() {
    let mut host = Host::start();
    let (mut controller, pid) = host.spawn("/bin/cat", &[]);
    let socket = host.meta().socket;
    let mut hello = Vec::new();
    let observer = wire::Hello {
        proto_min: 1,
        proto_max: 1,
        proto_minor: 0,
        role: Role::Observer,
        force: false,
        features: wire::Features::NONE,
        client_version: wire::AppVersion::default(),
        client_name: "cut",
        ticket: None,
    };
    wire::encode(&mut hello, CHANNEL_LOCAL, 0, &Message::Hello(observer)).expect("encodes");
    wire::encode(&mut hello, CHANNEL_LOCAL, 0, &Message::Ping(1)).expect("encodes");
    // A socket that goes away mid-frame is what the kernel leaves of a killed client; cut at
    // byte positions across both frames (a dozen connections, no PTY).
    for cut in [1, 4, 7, 8, 9, 30, hello.len() / 2, hello.len() - 9, hello.len() - 1] {
        let mut stream = UnixStream::connect(&socket).expect("connects");
        let _ = stream.write_all(&hello[..cut]);
        drop(stream);
        std::thread::sleep(Duration::from_millis(5));
    }
    // And a real process, killed with SIGKILL while it holds half a frame.
    if Path::new("/usr/bin/nc").exists() {
        let mut nc = Command::new("/usr/bin/nc")
            .arg("-U")
            .arg(&socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("nc starts");
        if let Some(stdin) = nc.stdin.as_mut() {
            let _ = stdin.write_all(&hello[..hello.len() / 2]);
            let _ = stdin.flush();
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(signal(nc.id(), libc::SIGKILL));
        let _ = nc.wait();
    }
    assert!(host.is_running() && alive(pid));
    controller.input(b"still here\r");
    controller.until_text("still here");
    let mut observer = connect_retrying(&host, Role::Observer);
    observer.send(&Message::Ping(2));
    observer.until("Pong", |g| matches!(g, Got::Pong(2, _)));
}

#[test]
fn a_stalled_client_is_cut_loose_while_yes_runs_and_the_host_stays_small() {
    let mut host = Host::start();
    let (mut stalled, pid) = host.spawn("/usr/bin/yes", &[]);
    // It reads nothing for a while: no acks, a full socket.
    stalled.auto_ack = false;
    std::thread::sleep(Duration::from_millis(1500));
    let rss = host.rss_kib();
    assert!(rss > 0 && rss < 96 * 1024, "the host's memory stays bounded: {rss} KiB");
    let mut watcher = connect_retrying(&host, Role::Observer);
    watcher.attach(StreamMode::None, 0);
    watcher.send(&Message::Ping(1));
    let Got::Pong(_, before) = watcher.until("Pong", |g| matches!(g, Got::Pong(1, _))) else {
        unreachable!()
    };
    std::thread::sleep(Duration::from_millis(300));
    watcher.send(&Message::Ping(2));
    let Got::Pong(_, after) = watcher.until("Pong", |g| matches!(g, Got::Pong(2, _))) else { unreachable!() };
    assert!(after > before, "the program keeps running and the host keeps parsing ({before} -> {after})");
    stalled.until("Detached(TooSlow)", |g| matches!(g, Got::Detached(DetachReason::TooSlow)));
    let sent: usize = stalled.log.iter().map(|g| if let Got::Out(_, b) = g { b.len() } else { 0 }).sum();
    assert!(sent <= 1024 * 1024, "never past the window it never acknowledged: {sent} bytes");
    // Back with a repaint.
    stalled.auto_ack = true;
    stalled.attach(StreamMode::Sync, 0);
    stalled.until("the repaint", |g| matches!(g, Got::Repaint { last: true, .. }));
    stalled.until_text("y");
    stalled.send(&Message::Close { grace_ms: 500 });
    assert!(host.wait_exit().success());
    wait_gone(pid);
}

#[test]
fn a_program_that_exits_while_nobody_is_attached_stays_exited() {
    let mut host = Host::start();
    let mut spawner = host.connect(Role::Controller);
    let pid = spawner.spawn("/bin/sh", &["-c", "echo goodbye from sh; exit 3"]);
    spawner.send(&Message::Detach);
    spawner.until_closed();
    wait_gone(pid);
    // The host notices the exit a moment after the program is reaped.
    let until = Instant::now() + PATIENCE;
    loop {
        let mut admin = connect_retrying(&host, Role::Admin);
        admin.send(&Message::GetInfo);
        if let Got::Info(info) = admin.until("Info", |g| matches!(g, Got::Info(_)))
            && info.state == HostState::Exited
        {
            break;
        }
        assert!(Instant::now() < until, "the host never said the program exited");
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut later = connect_retrying(&host, Role::Controller);
    let welcome = later.welcome.expect("a welcome");
    assert_eq!(welcome.state, HostState::Exited);
    assert_eq!(welcome.exit.map(|x| (x.code, x.signal)), Some((3, 0)));
    later.attach(StreamMode::Sync, 0);
    later.until("Exited", |g| matches!(g, Got::Exited(x) if x.code == 3));
    assert!(later.screen_text().contains("goodbye from sh"), "{}", later.screen_text());
    assert!(host.is_running(), "an exited program's host waits for its client");
    later.send(&Message::Close { grace_ms: 0 });
    assert!(host.wait_exit().success());
    assert!(!host.meta_path().exists());
}

#[test]
fn a_second_host_for_the_same_id_is_refused_and_the_first_is_untouched() {
    let mut first = Host::start();
    let meta = first.meta();
    let other_data = tempfile::tempdir().expect("a temp dir");
    let mut second = Host::start_in(other_data, first.hosts_dir.clone(), first.id.clone());
    assert!(second.ready_line().starts_with("err "), "the second says why it could not start");
    assert!(!second.wait_exit().success());
    assert!(first.is_running());
    assert_eq!(first.meta(), meta, "its meta file is still its own");
    let mut client = connect_retrying(&first, Role::Controller);
    client.send(&Message::Ping(3));
    client.until("Pong", |g| matches!(g, Got::Pong(3, _)));
}
