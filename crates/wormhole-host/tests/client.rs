//! The app's client (`wormhole_term::client`) against the real host binary, with a real PTY and
//! real processes: starting a hosted terminal, detaching and attaching again to the same screen,
//! resizes in the stream, closing, a host that is killed, and a headless attachment.
//!
//! Each test runs one host with a small program (`/bin/cat`, `/bin/sh -c …`), never a login shell,
//! in a scratch data dir: one PTY per test, far from the system's limit.

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{PATIENCE, alive, new_id, signal, wait_gone};
use wormhole_term::client::{self, HostCommand};
use wormhole_term::{GridSize, Launch, Pty, PtyEvent, PtyEvents, Shell, StartOptions};

const GRID: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

/// A scratch data dir with a hosts folder, and an id for the host.
struct Place {
    _data: tempfile::TempDir,
    hosts_dir: PathBuf,
    id: String,
}

impl Place {
    fn new() -> Self {
        let data = tempfile::tempdir().expect("a temp dir");
        let hosts_dir = data.path().join("hosts");
        Self { _data: data, hosts_dir, id: new_id() }
    }

    fn command(&self) -> HostCommand<'_> {
        HostCommand {
            binary: env!("CARGO_BIN_EXE_wormhole-host").as_ref(),
            hosts_dir: &self.hosts_dir,
            id: &self.id,
        }
    }

    fn host_pid(&self) -> u32 {
        let path = wormhole_term::hosted::paths::meta_path(&self.hosts_dir, &self.id);
        wormhole_term::hosted::meta::read(&path).expect("the meta file").pid
    }

    fn attach(&self, visible: bool) -> (Pty, PtyEvents) {
        let socket = client::socket_of(&self.hosts_dir, &self.id).expect("the host's socket");
        Pty::attach_hosted(&socket, &self.id, &options(), visible).expect("attached")
    }
}

fn options() -> StartOptions<'static> {
    StartOptions { scrollback_lines: 1000, ..StartOptions::default() }
}

fn launch(program: &str, args: &[&str]) -> Launch {
    let shell = Shell::new(program.into(), args.iter().map(|a| a.to_string()).collect());
    let env = [("TERM", "xterm-256color"), ("PATH", "/usr/bin:/bin"), ("LANG", "en_US.UTF-8")]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Launch { shell, cwd: Some("/".into()), env }
}

fn start(place: &Place, program: &str, args: &[&str]) -> (Pty, PtyEvents) {
    Pty::start_hosted(&place.command(), GRID, &options(), launch(program, args)).expect("a hosted terminal")
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn wait_text(pty: &Pty, text: &str) {
    let until = Instant::now() + PATIENCE;
    while !pty.screen_text().contains(text) {
        assert!(Instant::now() < until, "{text:?} never showed; the screen:\n{}", pty.screen_text());
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn next_event(events: &PtyEvents, what: &str, mut pred: impl FnMut(&PtyEvent) -> bool) -> PtyEvent {
    let until = Instant::now() + PATIENCE;
    loop {
        assert!(Instant::now() < until, "no {what} event came");
        match events.try_recv() {
            Some(event) if pred(&event) => return event,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[test]
fn a_hosted_terminal_echoes_what_is_typed() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    assert!(pty.is_hosted());
    assert_eq!(pty.host_id(), Some(place.id.as_str()));
    let pid = pty.shell_pid().expect("the program runs");
    assert!(alive(pid));
    pty.write(b"hello hosted\r".as_slice());
    wait_text(&pty, "hello hosted");
}

#[test]
fn a_terminal_attached_again_after_detach_shows_the_screen_it_had() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    for i in 0..40 {
        pty.write(format!("line {i} \u{6f22} \x1b[31mred\x1b[0m\r").into_bytes());
    }
    wait_text(&pty, "line 39");
    // Let the echo settle: the screen must not change between the reading and the detach.
    let mut before = pty.screen_text();
    wait_for("a quiet screen", || {
        std::thread::sleep(Duration::from_millis(100));
        let now = pty.screen_text();
        std::mem::replace(&mut before, now.clone()) == now
    });
    let pid = pty.shell_pid().expect("the program runs");
    assert!(pty.detach());
    drop(pty);
    assert!(alive(pid), "detached, the program keeps running");

    let (again, _events) = place.attach(true);
    wait_for("the repaint", || again.screen_text() == before);
    assert_eq!(again.shell_pid(), Some(pid));
    again.write(b"after the attach\r".as_slice());
    wait_text(&again, "after the attach");
}

#[test]
fn a_resize_reaches_the_program_in_order() {
    let place = Place::new();
    let (mut pty, _events) = start(&place, "/bin/sh", &["-c", "stty size; read x; stty size; read y"]);
    wait_text(&pty, "24 80");
    pty.resize(GridSize { cols: 100, rows: 30, ..GRID });
    // The replica takes the size when the host's `ResizeApplied` comes, not before.
    wait_for("the resize", || pty.bottommost_line() == wormhole_term::types::Line(29));
    pty.write(b"\r".as_slice());
    wait_text(&pty, "30 100");
}

#[test]
fn closing_a_hosted_terminal_ends_its_program_and_its_host() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    let pid = pty.shell_pid().expect("the program runs");
    let host = place.host_pid();
    drop(pty);
    wait_gone(pid);
    wait_gone(host);
}

#[test]
fn a_killed_host_is_a_lost_host() {
    let place = Place::new();
    let (pty, events) = start(&place, "/bin/cat", &[]);
    let host = place.host_pid();
    assert!(signal(host, libc::SIGKILL));
    let got = next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(_)));
    assert!(matches!(got, PtyEvent::Exited(None)));
    assert!(pty.host_lost());
    assert!(!pty.is_alive());
}

#[test]
fn a_headless_terminal_hears_the_title_and_the_exit() {
    let place = Place::new();
    let script = "read x; printf '\\033]0;from the host\\007'; read y; exit 3";
    let (pty, _events) = start(&place, "/bin/sh", &["-c", script]);
    assert!(pty.detach());
    drop(pty);
    let (headless, events) = place.attach(false);
    headless.write(b"\r".as_slice());
    let got = next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_)));
    assert!(matches!(got, PtyEvent::Title(title) if title == "from the host"));
    headless.write(b"\r".as_slice());
    let got = next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(_)));
    assert!(matches!(got, PtyEvent::Exited(Some(3))));
    assert!(!headless.host_lost());
}

#[test]
fn a_headless_attach_after_a_relaunch_is_told_the_title_it_missed() {
    let place = Place::new();
    let script = "printf '\\033]2;hello\\007'; read x";
    let (pty, events) = start(&place, "/bin/sh", &["-c", script]);
    // Heard by the host once the first client has it.
    let got = next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_)));
    assert!(matches!(got, PtyEvent::Title(title) if title == "hello"));
    assert!(pty.detach());
    drop(pty);
    let (_headless, events) = place.attach(false);
    let got = next_event(&events, "title", |e| matches!(e, PtyEvent::Title(_)));
    assert!(matches!(got, PtyEvent::Title(title) if title == "hello"));
}
