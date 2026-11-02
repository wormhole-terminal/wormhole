//! The app's client (`wormhole_term::client`) against the real host binary, with a real PTY and
//! real processes: starting a hosted terminal, detaching and attaching again to the same screen,
//! resizes in the stream, closing, a host that is killed, and a headless attachment.
//!
//! Each test runs one host with a small program (`/bin/cat`, `/bin/sh -c …`), never a login shell,
//! in a scratch data dir: one PTY per test, far from the system's limit.

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use support::{PATIENCE, alive, host_binary, new_id, signal, wait_gone};
use wormhole_term::checkpoint::{self, Key};
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
        HostCommand { binary: host_binary(), hosts_dir: &self.hosts_dir, id: &self.id }
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

/// A terminal restored from text after its host is gone runs in a new host that draws the text first: the
/// spawning client sees it, and so does a client that attaches later (the host's own screen has it).
#[test]
fn a_terminal_restored_from_text_runs_in_a_new_host_that_seeds_the_screen() {
    let place = Place::new();
    let options = StartOptions { restore: Some("old line one\nold line two\n"), ..options() };
    let (pty, _events) = Pty::start_hosted(&place.command(), GRID, &options, launch("/bin/cat", &[]))
        .expect("a hosted terminal");
    assert!(pty.is_hosted(), "it runs in a host, not in the app");
    assert!(pty.host_minor() >= Some(4), "a host that draws the seed: {:?}", pty.host_minor());
    wait_text(&pty, "old line two");
    wait_text(&pty, "restored");
    pty.write(b"typed now\r".as_slice());
    wait_text(&pty, "typed now");
    let seen = pty.screen_text();
    let (old, typed) = (seen.find("old line one"), seen.find("typed now"));
    assert!(old.expect("the old text is on the screen") < typed.expect("and so is the typed line"), "{seen}");
    // A later attach gets the same from the host itself.
    assert!(pty.detach());
    drop(pty);
    let (again, _events) = place.attach(true);
    wait_text(&again, "old line one");
    wait_text(&again, "typed now");
}

/// An attach shows the host's screen and never draws saved text of its own: the host's repaint has what it
/// has, so a replica that added the saved text would differ from it (a headless attach would keep it until
/// shown).
#[test]
fn an_attach_never_replays_saved_text() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    pty.write(b"the host's line\r".as_slice());
    wait_text(&pty, "the host's line");
    assert!(pty.detach());
    drop(pty);
    let socket = client::socket_of(&place.hosts_dir, &place.id).expect("the host's socket");
    let stale = StartOptions { restore: Some("stale saved text\n"), ..options() };
    for visible in [true, false] {
        let (again, _events) = Pty::attach_hosted(&socket, &place.id, &stale, visible).expect("attached");
        if visible {
            wait_text(&again, "the host's line");
        }
        assert!(
            !again.screen_text().contains("stale saved text"),
            "visible {visible}: {}",
            again.screen_text()
        );
        assert!(!again.screen_text().contains("restored"));
        assert!(again.detach());
    }
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

fn checkpoint_key() -> Key {
    Key::new([9; 32])
}

impl Place {
    /// This terminal's checkpoint directory, where the host puts it.
    fn checkpoint_dir(&self) -> PathBuf {
        checkpoint::store::dir_in(&checkpoint::sessions_dir_of_hosts(&self.hosts_dir), &self.id)
    }

    /// Wait until the newest checkpoint's screen has `text`; what it holds.
    fn wait_checkpoint(&self, text: &str) -> checkpoint::Restored {
        let until = Instant::now() + PATIENCE;
        loop {
            if let Ok(read) =
                checkpoint::read_latest(&self.checkpoint_dir(), &self.id, Some(&checkpoint_key()))
                && read.screen.as_deref().is_some_and(|s| String::from_utf8_lossy(s).contains(text))
            {
                return read;
            }
            assert!(Instant::now() < until, "no checkpoint with {text:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// A host that was never asked writes nothing, whatever the program prints.
#[test]
fn a_host_that_was_not_asked_keeps_no_checkpoint() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    pty.write(b"nothing to keep\r".as_slice());
    wait_text(&pty, "nothing to keep");
    assert!(pty.checkpoint_now(true), "the host can, it was just not asked to start");
    std::thread::sleep(Duration::from_millis(300));
    assert!(!place.checkpoint_dir().exists(), "no directory either");
}

/// The whole path of a restart: the app asks for checkpoints, the terminal is used, the host is sent SIGTERM
/// (as loginwindow does), and the checkpoint is there with the screen as it was, while the host's meta and
/// socket are gone as before.
#[test]
fn a_checkpoint_survives_sigterm_with_the_screen_the_person_left() {
    let place = Place::new();
    let (pty, _events) = start(&place, "/bin/cat", &[]);
    assert!(pty.start_checkpoints(Some(&checkpoint_key()), "login"));
    pty.write(b"before the restart\r".as_slice());
    wait_text(&pty, "before the restart");
    let host = place.host_pid();
    let socket = client::socket_of(&place.hosts_dir, &place.id).unwrap();
    // Typed just before the signal: no quiet time has passed, only the flush before the hangup can have it.
    pty.write(b"last words\r".as_slice());
    wait_text(&pty, "last words");
    let started = Instant::now();
    assert!(signal(host, libc::SIGTERM));
    wait_gone(host);
    let read = place.wait_checkpoint("last words");
    let text = String::from_utf8_lossy(read.screen.as_deref().unwrap()).into_owned();
    assert!(text.contains("before the restart"), "{text:?}");
    assert_eq!(read.manifest.shell.as_deref(), Some("login"));
    assert!(read.manifest.cwd.is_some(), "the folder the shell is in");
    assert!(started.elapsed() < Duration::from_secs(3), "the flush and the hangup fit before the kill");
    assert!(place.checkpoint_dir().is_dir(), "the host leaves the checkpoint alone");
    assert!(!socket.exists(), "and removes its socket as always");
    drop(pty);
}

/// A program that exits leaves a checkpoint that says how, and the host does not delete it; closing the terminal
/// ends the host and still leaves the directory, which is the app's to remove.
#[test]
fn a_program_that_exits_leaves_its_checkpoint_and_closing_does_not_remove_it() {
    let place = Place::new();
    let (pty, events) = start(&place, "/bin/sh", &["-c", "read x; echo goodbye; exit 3"]);
    assert!(pty.start_checkpoints(Some(&checkpoint_key()), "login"));
    pty.write(b"\r".as_slice());
    next_event(&events, "exit", |e| matches!(e, PtyEvent::Exited(_)));
    let read = place.wait_checkpoint("goodbye");
    let until = Instant::now() + PATIENCE;
    let ended = loop {
        let latest =
            checkpoint::read_latest(&place.checkpoint_dir(), &place.id, Some(&checkpoint_key())).unwrap();
        if latest.manifest.ended.is_some() {
            break latest.manifest.ended;
        }
        assert!(Instant::now() < until, "the last checkpoint never said how it ended: {:?}", read.manifest);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(ended.as_deref(), Some("exit:3"));
    let host = place.host_pid();
    drop(pty); // closing the terminal ends the host
    wait_gone(host);
    assert!(place.checkpoint_dir().is_dir(), "the app removes a checkpoint, the host never does");
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
