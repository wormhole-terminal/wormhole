//! The checkpointer against a fake terminal and real files: nothing is written until the app asks, the work is
//! done on the checkpoint thread and nowhere else, a flush is bounded, the host never deletes what it wrote,
//! and a program that ended is said to have.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use wormhole_term::checkpoint::{Capture, Key, Snapshot, Written, read_latest};

use super::worker::THREAD_NAME;
use super::*;

const ID: &str = "abcdefghij12";

fn key() -> Key {
    Key::new([3; 32])
}

/// What the scripted terminal shows, and the threads that read it.
type Shown = Arc<Mutex<Option<Vec<u8>>>>;
type Names = Arc<Mutex<Vec<String>>>;
/// The writes the counting sink saw: the thread, and how the program ended if the snapshot said.
type Writes = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A terminal that shows `screen`, and notes the thread each read came on.
struct Fake {
    screen: Shown,
    threads: Names,
}

impl Source for Fake {
    fn capture(&self) -> Capture {
        self.threads.lock().unwrap().push(thread_name());
        Capture {
            cols: 80,
            rows: 24,
            cwd: Some("/Users/me/src".into()),
            title: Some("t".into()),
            screen: self.screen.lock().unwrap().clone(),
        }
    }
}

fn thread_name() -> String {
    std::thread::current().name().unwrap_or("?").to_owned()
}

fn fake(screen: Option<&[u8]>) -> (Fake, Shown, Names) {
    let screen = Arc::new(Mutex::new(screen.map(<[u8]>::to_vec)));
    let threads = Arc::new(Mutex::new(Vec::new()));
    (Fake { screen: screen.clone(), threads: threads.clone() }, screen, threads)
}

/// A sink that counts writes and notes their threads, and can be slow.
struct Counting {
    writes: Writes,
    slow: Duration,
}

impl Sink for Counting {
    fn write(&mut self, _key: Option<&Key>, snapshot: &Snapshot<'_>) -> io::Result<Written> {
        std::thread::sleep(self.slow);
        self.writes.lock().unwrap().push((thread_name(), snapshot.ended.map(str::to_owned)));
        Ok(Written { generation: 1, bytes: 10, screen: snapshot.screen.is_some() })
    }
}

fn counting(slow: Duration) -> (Box<Counting>, Writes) {
    let writes = Arc::new(Mutex::new(Vec::new()));
    (Box::new(Counting { writes: writes.clone(), slow }), writes)
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !cond() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn nothing_is_written_until_the_app_asks() {
    let sessions = tempfile::tempdir().unwrap();
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    let t0 = Instant::now();
    assert!(!cp.is_on());
    assert_eq!(cp.pump(100, t0), None, "no deadline either");
    assert_eq!(cp.pump(200, t0 + secs(1000)), None);
    cp.request();
    assert_eq!(cp.flush(secs(1)), Flushed::Off);
    assert_eq!(std::fs::read_dir(sessions.path()).unwrap().count(), 0, "not even the directory");
}

#[test]
fn the_work_is_done_on_the_checkpoint_thread_and_nowhere_else() {
    let (source, _, reads) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::ZERO);
    let mut cp = Checkpointer::new(std::env::temp_dir(), ID.to_owned());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), "login".into(), t0);
    // The driver's turns: the first checkpoint, output, quiet, a request, a flush.
    cp.pump(0, t0);
    cp.pump(500, t0 + secs(1));
    cp.pump(500, t0 + secs(5));
    cp.request();
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    wait_for("the writes", || writes.lock().unwrap().len() >= 3);
    let on = |names: &[String]| names.iter().all(|n| n == THREAD_NAME);
    assert!(on(&reads.lock().unwrap()), "reads: {:?}", reads.lock().unwrap());
    let writer_threads: Vec<String> = writes.lock().unwrap().iter().map(|w| w.0.clone()).collect();
    assert!(on(&writer_threads), "writes: {writer_threads:?}");
    assert!(!reads.lock().unwrap().is_empty());
}

#[test]
fn a_checkpoint_is_due_once_after_quiet_and_idle_adds_none() {
    let (source, _, _) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::ZERO);
    let mut cp = Checkpointer::new(std::env::temp_dir(), ID.to_owned());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), "login".into(), t0);
    // The first one is due after the quiet time.
    assert_eq!(cp.pump(0, t0), Some(t0 + schedule::QUIET));
    assert_eq!(writes.lock().unwrap().len(), 0, "not before");
    cp.pump(0, t0 + secs(4));
    wait_for("the first checkpoint", || writes.lock().unwrap().len() == 1);
    // Idle: no deadline, and nothing more however long.
    assert_eq!(cp.pump(0, t0 + secs(5)), None);
    cp.pump(0, t0 + secs(7200));
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(writes.lock().unwrap().len(), 1);
    // Output, then quiet: one more.
    cp.pump(10, t0 + secs(7201));
    cp.pump(10, t0 + secs(7205));
    wait_for("the second checkpoint", || writes.lock().unwrap().len() == 2);
}

/// The restart gives the host three seconds before the kill: the flush waits one second at the most, whatever
/// the disk is doing, and the shell is hung up after it.
#[test]
fn a_flush_is_bounded_even_when_the_disk_is_not() {
    let (source, _, _) = fake(Some(b"screen"));
    let (sink, _) = counting(secs(5));
    let mut cp = Checkpointer::new(std::env::temp_dir(), ID.to_owned());
    cp.configure_with(Box::new(source), sink, Some(key()), "login".into(), Instant::now());
    let start = Instant::now();
    assert_eq!(cp.flush(FLUSH_WITHIN), Flushed::TimedOut);
    let took = start.elapsed();
    assert!(took >= FLUSH_WITHIN && took < FLUSH_WITHIN + Duration::from_millis(300), "{took:?}");
}

/// A real directory and a screen of four mebibytes: sealed, written and committed well inside the second.
#[test]
fn a_flush_of_a_big_screen_finishes_well_inside_its_bound() {
    let sessions = tempfile::tempdir().unwrap();
    let big: Vec<u8> =
        b"a line of output that is not short\r\n".iter().copied().cycle().take(4 * 1024 * 1024).collect();
    let (source, _, _) = fake(Some(&big));
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    cp.configure(Box::new(source), Some(key()), "login".into(), Instant::now());
    let start = Instant::now();
    let flushed = cp.flush(FLUSH_WITHIN);
    let took = start.elapsed();
    assert!(matches!(flushed, Flushed::Done(bytes) if bytes > 4 * 1024 * 1024), "{flushed:?}");
    assert!(took < Duration::from_millis(900), "a flush of 4 MiB took {took:?}");
    let read = read_latest(&sessions.path().join(ID), ID, Some(&key())).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::len), Some(big.len()));
    assert_eq!(read.manifest.cwd.as_deref(), Some("/Users/me/src"));
    assert_eq!(read.manifest.shell.as_deref(), Some("login"));
}

#[test]
fn without_a_key_the_folder_and_the_shell_are_written_and_no_screen() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, _, _) = fake(Some(b"secret text"));
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    cp.configure(Box::new(source), None, "fish".into(), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let read = read_latest(&sessions.path().join(ID), ID, None).unwrap();
    assert!(read.screen.is_none() && read.issue.is_none());
    assert_eq!(read.manifest.shell.as_deref(), Some("fish"));
    for entry in std::fs::read_dir(sessions.path().join(ID)).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(!bytes.windows(6).any(|w| w == b"secret"));
    }
}

#[test]
fn a_full_screen_program_leaves_the_last_screen_in_place() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, screen, _) = fake(Some(b"the prompt"));
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    cp.configure(Box::new(source), Some(key()), "login".into(), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(n) if n > 0));
    *screen.lock().unwrap() = None; // vim is up
    assert_eq!(cp.flush(secs(2)), Flushed::Done(0), "nothing written");
    let read = read_latest(&sessions.path().join(ID), ID, Some(&key())).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"the prompt"[..]));
}

/// A program that exits ends in a last generation that says how; the host keeps it and every earlier one it
/// still holds. Nothing the host does removes the directory: only the app knows a terminal was closed.
#[test]
fn a_program_that_ended_leaves_a_checkpoint_that_says_so_and_the_host_keeps_it() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, _, _) = fake(Some(b"last words"));
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    cp.configure(Box::new(source), Some(key()), "login".into(), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let info = wire::ExitInfo { code: 0, signal: 0, at_ms: 1 };
    assert!(matches!(cp.ended(&info, secs(2)), Flushed::Done(_)));
    // Later writes (a flush at SIGTERM) go on saying so.
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    drop(cp);
    let dir = sessions.path().join(ID);
    assert!(dir.is_dir(), "dropping the checkpointer removes nothing");
    let read = read_latest(&dir, ID, Some(&key())).unwrap();
    assert_eq!(read.manifest.ended.as_deref(), Some("exit:0"));
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"last words"[..]));
    let killed = wire::ExitInfo { code: 0, signal: 1, at_ms: 2 };
    assert_eq!(ended_text(&killed), "signal:1");
}

#[test]
fn a_new_key_restarts_the_writer_and_keeps_what_was_said_of_the_end() {
    let sessions = tempfile::tempdir().unwrap();
    let mut cp = Checkpointer::new(sessions.path().to_owned(), ID.to_owned());
    let (source, _, _) = fake(Some(b"one"));
    cp.configure(Box::new(source), Some(key()), "login".into(), Instant::now());
    cp.ended(&wire::ExitInfo { code: 2, signal: 0, at_ms: 1 }, secs(2));
    let (source, _, _) = fake(Some(b"two"));
    cp.configure(Box::new(source), Some(Key::new([4; 32])), "login".into(), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let read = read_latest(&sessions.path().join(ID), ID, Some(&Key::new([4; 32]))).unwrap();
    assert_eq!(read.manifest.ended.as_deref(), Some("exit:2"));
}
