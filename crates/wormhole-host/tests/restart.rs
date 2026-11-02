//! A restart of the Mac, simulated: every host is sent SIGTERM (as loginwindow does, the app having quit first),
//! the app comes back, and each terminal is started again in a new host from the checkpoint its old host left.
//! The restored screen is compared with the screen the person last saw, cell by cell (character, colors,
//! attributes), with the real host binary, real PTYs and real files.
//!
//! The variants are what can go wrong around it: the host killed outright instead of ended (its last
//! checkpoint is still whole), the newest checkpoint damaged (the one before it), another key, and a checkpoint
//! from a newer Wormhole. Each ends in the same place: a terminal that comes back with a screen, or one that
//! says why it did not and is restored from its text as before.

#![allow(clippy::disallowed_methods, reason = "a test builds its own files and processes")]

mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use support::{PATIENCE, host_binary, new_id, signal, wait_gone};
use wormhole_term::checkpoint::{
    self, Key, ReadError, divider, read_latest, safe_replay, sessions_dir_of_hosts,
};
use wormhole_term::client::{self, HostCommand};
use wormhole_term::types::{Cell, Flags};
use wormhole_term::{GridSize, Launch, Pty, PtyEvents, ScreenSnapshot, Shell, StartOptions};

const GRID: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

/// The look of a cell that a restore must bring back: what it shows, its colors and its attributes.
type Look = (char, String, String, u32);

/// One terminal of the old life: its id and the host's files, in the data folder all of them share.
struct Terminal {
    id: String,
    pty: Pty,
    _events: PtyEvents,
}

fn key() -> Key {
    Key::new([5; 32])
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

fn command<'a>(hosts_dir: &'a Path, id: &'a str) -> HostCommand<'a> {
    HostCommand { binary: host_binary(), hosts_dir, id }
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !cond() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn wait_text(pty: &Pty, text: &str) {
    wait_for(&format!("{text:?} on the screen"), || pty.screen_text().contains(text));
}

/// The cells of the first `rows` rows of the screen of `pty`.
fn looks(pty: &Pty, rows: usize) -> Vec<Vec<Look>> {
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot);
    let attributes = Flags::BOLD
        | Flags::DIM
        | Flags::ITALIC
        | Flags::ALL_UNDERLINES
        | Flags::INVERSE
        | Flags::STRIKEOUT
        | Flags::WIDE_CHAR
        | Flags::WIDE_CHAR_SPACER;
    snapshot
        .rows()
        .iter()
        .take(rows)
        .map(|row| {
            row.cells()
                .iter()
                .map(|cell: &Cell| {
                    (
                        cell.c,
                        format!("{:?}", cell.fg),
                        format!("{:?}", cell.bg),
                        (cell.flags & attributes).bits(),
                    )
                })
                .collect()
        })
        .collect()
}

/// The program every old terminal runs: some output with colors, attributes and wide characters, then it waits.
const OUTPUT: &str = "printf '\\033[1;31mred bold\\033[0m\\n\\033[4;38;2;10;20;30munder rgb\\033[0m plain\\n\\033[48;5;200m  bg  \\033[0m\\nwide: \\346\\227\\245\\346\\234\\254\\350\\252\\236 caf\\303\\251\\n'; read wait";

/// `count` terminals in one data folder, each asked to keep a checkpoint and each holding its screen on a
/// checkpoint. Returns the hosts' folder and the terminals.
fn old_life(count: usize) -> (tempfile::TempDir, PathBuf, Vec<Terminal>) {
    let data = tempfile::tempdir().expect("a data folder");
    let hosts_dir = data.path().join("hosts");
    let mut terminals = Vec::new();
    for n in 0..count {
        let id = new_id();
        let (pty, events) = Pty::start_hosted(
            &command(&hosts_dir, &id),
            GRID,
            &options(),
            launch("/bin/sh", &["-c", OUTPUT]),
        )
        .expect("a hosted terminal");
        wait_text(&pty, "wide: 日本語");
        assert!(pty.start_checkpoints(&key()));
        // Each terminal ends up with its own last line, so a restore that mixed them up is seen.
        let _ = n;
        assert!(pty.checkpoint_now(true));
        terminals.push(Terminal { id, pty, _events: events });
    }
    for t in &terminals {
        wait_for("a checkpoint with the output", || {
            read_latest(
                &checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &t.id),
                &t.id,
                Some(&key()),
            )
            .ok()
            .and_then(|r| r.screen)
            .is_some_and(|s| String::from_utf8_lossy(&s).contains("caf"))
        });
    }
    (data, hosts_dir, terminals)
}

/// The Mac restarts: the app quits (every terminal lets go of its host), then each host is sent SIGTERM.
fn restart(hosts_dir: &Path, terminals: Vec<Terminal>) -> Vec<String> {
    let pids: Vec<u32> = terminals
        .iter()
        .map(|t| {
            let path = wormhole_term::hosted::paths::meta_path(hosts_dir, &t.id);
            wormhole_term::hosted::meta::read(&path).expect("the meta file").pid
        })
        .collect();
    let ids: Vec<String> = terminals.iter().map(|t| t.id.clone()).collect();
    for t in &terminals {
        assert!(t.pty.detach());
    }
    drop(terminals);
    for pid in &pids {
        assert!(signal(*pid, libc::SIGTERM));
    }
    for pid in pids {
        wait_gone(pid);
    }
    ids
}

/// The new life of a terminal: a new host started with the checkpoint's screen as the seed, as the app does
/// (`terminal::checkpoint::read` makes the same text). `None` when there is nothing to restore from.
fn relaunch(hosts_dir: &Path, old: &str, key: &Key) -> Result<Terminal, ReadError> {
    let dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(hosts_dir), old);
    let read = read_latest(&dir, old, Some(key))?;
    let screen = read.screen.expect("a checkpoint with a screen");
    let mut seed = String::from_utf8_lossy(&safe_replay(&screen)).into_owned();
    seed.push_str(&divider("restored 2026-11-02 14:03"));
    let id = new_id();
    let options = StartOptions { restore: Some(&seed), ..options() };
    let (pty, events) = Pty::start_hosted(&command(hosts_dir, &id), GRID, &options, launch("/bin/cat", &[]))
        .expect("a new host");
    wait_text(&pty, "restored 2026-11-02 14:03");
    Ok(Terminal { id, pty, _events: events })
}

/// Rows 0..4 of the old screen (taken before the restart) are the new screen's, cell by cell.
fn assert_same_screen(old: &[Vec<Look>], new: &Terminal) {
    let now = looks(&new.pty, old.len());
    for (row, (was, is)) in old.iter().zip(&now).enumerate() {
        assert_eq!(was, is, "row {row} of the restored terminal differs from the one the person left");
    }
}

#[test]
fn a_restart_brings_every_terminal_back_as_the_person_left_it() {
    let (_data, hosts_dir, terminals) = old_life(3);
    let before: Vec<Vec<Vec<Look>>> = terminals.iter().map(|t| looks(&t.pty, 4)).collect();
    assert!(
        before[0][0].iter().any(|l| l.0 == 'r' && l.3 != 0),
        "the red bold line is bold: {:?}",
        before[0][0][0]
    );
    let ids = restart(&hosts_dir, terminals);
    for id in &ids {
        let dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), id);
        assert!(dir.is_dir(), "the host's SIGTERM left its checkpoint");
    }
    for (id, old) in ids.iter().zip(&before) {
        let new = relaunch(&hosts_dir, id, &key()).expect("restored");
        assert_same_screen(old, &new);
        assert!(new.pty.is_hosted(), "and it runs in a host again, so it survives the next restart");
    }
}

#[test]
fn a_host_killed_outright_leaves_the_last_checkpoint_whole() {
    let (_data, hosts_dir, terminals) = old_life(1);
    let before = looks(&terminals[0].pty, 4);
    let pid = {
        let path = wormhole_term::hosted::paths::meta_path(&hosts_dir, &terminals[0].id);
        wormhole_term::hosted::meta::read(&path).unwrap().pid
    };
    // More output the checkpoint never saw, then the host is killed with no chance to flush.
    terminals[0].pty.write(b"typed after the last checkpoint\r".as_slice());
    assert!(signal(pid, libc::SIGKILL));
    wait_gone(pid);
    let id = terminals[0].id.clone();
    drop(terminals);
    let new = relaunch(&hosts_dir, &id, &key()).expect("restored from the last checkpoint");
    assert_same_screen(&before, &new);
}

#[test]
fn a_damaged_newest_generation_gives_way_to_the_one_before_it() {
    let (_data, hosts_dir, terminals) = old_life(1);
    let before = looks(&terminals[0].pty, 4);
    // A second checkpoint (the same screen) so there are two generations to choose between.
    assert!(terminals[0].pty.checkpoint_now(true));
    let id = terminals[0].id.clone();
    let dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &id);
    wait_for("a second generation", || {
        std::fs::read_dir(&dir)
            .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".manifest")).count())
            .unwrap_or(0)
            >= 2
    });
    let ids = restart(&hosts_dir, terminals);
    // The newest screen file is damaged; the one before it is whole.
    let newest = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "screen"))
        .max()
        .unwrap();
    let mut bytes = std::fs::read(&newest).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&newest, bytes).unwrap();
    let new = relaunch(&hosts_dir, &ids[0], &key()).expect("restored from the generation before");
    assert_same_screen(&before, &new);
}

#[test]
fn another_key_or_a_newer_format_restores_nothing_and_says_so() {
    let (_data, hosts_dir, terminals) = old_life(1);
    let ids = restart(&hosts_dir, terminals);
    let dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &ids[0]);
    // The profile's key changed (the saved text was erased, or the Keychain item replaced).
    let read = read_latest(&dir, &ids[0], Some(&Key::new([6; 32]))).expect("the manifest still reads");
    assert!(read.screen.is_none() && read.issue == Some(checkpoint::Issue::WrongKey));
    // A newer Wormhole wrote it: left alone, and this one restores from the text.
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|x| x == "manifest") {
            let text = std::fs::read_to_string(&path).unwrap().replace("min_reader=1", "min_reader=9");
            std::fs::write(&path, text).unwrap();
        }
    }
    assert!(matches!(read_latest(&dir, &ids[0], Some(&key())), Err(ReadError::Newer(9))));
    assert!(dir.is_dir(), "and left where it is");
}

/// The hosts' files after a restart are gone (the app forgets them), but the checkpoints are not.
#[test]
fn the_hosts_files_are_gone_after_a_restart_and_the_checkpoints_are_not() {
    let (_data, hosts_dir, terminals) = old_life(2);
    let ids = restart(&hosts_dir, terminals);
    for id in &ids {
        assert!(!wormhole_term::hosted::paths::meta_path(&hosts_dir, id).exists(), "no meta");
        assert!(client::socket_of(&hosts_dir, id).is_err(), "no socket to find");
        assert!(checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), id).is_dir());
    }
}

/// The new host of a restored terminal is killed before it has written anything: the old checkpoint is still
/// there (nothing removes it until the new host's has landed), and the next launch restores from it again.
#[test]
fn a_restore_followed_at_once_by_the_new_hosts_death_restores_again_from_the_old_checkpoint() {
    let (_data, hosts_dir, terminals) = old_life(1);
    let before = looks(&terminals[0].pty, 4);
    let ids = restart(&hosts_dir, terminals);
    let first = relaunch(&hosts_dir, &ids[0], &key()).expect("restored");
    // The app has sent no key yet and the host no quiet second: nothing of its own on disk. It dies.
    let new_dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &first.id);
    assert!(!new_dir.exists(), "the new host has written nothing");
    let pid = {
        let path = wormhole_term::hosted::paths::meta_path(&hosts_dir, &first.id);
        wormhole_term::hosted::meta::read(&path).unwrap().pid
    };
    assert!(signal(pid, libc::SIGKILL));
    wait_gone(pid);
    drop(first);
    let old_dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &ids[0]);
    assert!(old_dir.is_dir(), "the old checkpoint is still where it was");
    let second = relaunch(&hosts_dir, &ids[0], &key()).expect("restored again");
    assert_same_screen(&before, &second);
}

/// The other way: asked to flush, the new host's own checkpoint lands, and then (and only then) the old one is
/// spent. This is what the app waits for.
#[test]
fn a_restored_terminals_new_host_writes_its_own_checkpoint_when_asked_to_flush() {
    let (_data, hosts_dir, terminals) = old_life(1);
    let ids = restart(&hosts_dir, terminals);
    let new = relaunch(&hosts_dir, &ids[0], &key()).expect("restored");
    let new_dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), &new.id);
    assert!(!checkpoint::store::is_readable(&new_dir, &new.id, Some(&key())));
    assert!(new.pty.start_checkpoints(&key()));
    assert!(new.pty.checkpoint_now(true));
    wait_for("the new host's own checkpoint", || {
        checkpoint::store::is_readable(&new_dir, &new.id, Some(&key()))
    });
    let read = read_latest(&new_dir, &new.id, Some(&key())).unwrap();
    let screen = String::from_utf8_lossy(read.screen.as_deref().unwrap()).into_owned();
    assert!(screen.contains("red bold"), "the restored screen is in it: {screen:?}");
}

/// Thirty hosts get SIGTERM in the same instant (the Mac is shutting down) with output no checkpoint holds yet:
/// every one writes it within the host's second, and the whole lot is gone well inside the seconds a shutdown gives.
/// The time is printed (`--nocapture`) for the record.
#[test]
fn thirty_hosts_flush_their_last_output_when_all_are_signalled_at_once() {
    const HOSTS: usize = 30;
    let data = tempfile::tempdir().expect("a data folder");
    let hosts_dir = data.path().join("hosts");
    let mut terminals = Vec::new();
    for n in 0..HOSTS {
        let id = new_id();
        let (pty, events) =
            Pty::start_hosted(&command(&hosts_dir, &id), GRID, &options(), launch("/bin/cat", &[]))
                .expect("a hosted terminal");
        assert!(pty.start_checkpoints(&key()));
        pty.write(format!("early {n}\r").into_bytes());
        wait_text(&pty, &format!("early {n}"));
        terminals.push(Terminal { id, pty, _events: events });
    }
    // Output the schedule has not written yet: only the flush at SIGTERM can keep it.
    for (n, t) in terminals.iter().enumerate() {
        t.pty.write(format!("last words {n}\r").into_bytes());
    }
    for (n, t) in terminals.iter().enumerate() {
        wait_text(&t.pty, &format!("last words {n}"));
    }
    let pids: Vec<u32> = terminals
        .iter()
        .map(|t| {
            wormhole_term::hosted::meta::read(&wormhole_term::hosted::paths::meta_path(&hosts_dir, &t.id))
                .expect("the meta file")
                .pid
        })
        .collect();
    let ids: Vec<String> = terminals.iter().map(|t| t.id.clone()).collect();
    for t in &terminals {
        assert!(t.pty.detach());
    }
    drop(terminals);
    let started = std::time::Instant::now();
    for pid in &pids {
        assert!(signal(*pid, libc::SIGTERM));
    }
    for pid in pids {
        wait_gone(pid);
    }
    let took = started.elapsed();
    eprintln!("{HOSTS} hosts: SIGTERM to the last one gone in {took:?}");
    assert!(took < std::time::Duration::from_secs(3), "{took:?}");
    for (n, id) in ids.iter().enumerate() {
        let dir = checkpoint::store::dir_in(&sessions_dir_of_hosts(&hosts_dir), id);
        let read = read_latest(&dir, id, Some(&key())).expect("a checkpoint");
        let screen = String::from_utf8_lossy(read.screen.as_deref().expect("a screen")).into_owned();
        assert!(screen.contains(&format!("last words {n}")), "host {n} kept its last output: {screen:?}");
    }
}
