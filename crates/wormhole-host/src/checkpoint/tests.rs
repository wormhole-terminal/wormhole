//! The checkpointer against a fake terminal and real files: nothing is written until the app asks, the work is
//! done on the one checkpoint thread and nowhere else, a flush is bounded, a stop and a close end the writing, the
//! host never deletes what it wrote, and a program that ended is said to have.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use wormhole_term::checkpoint::{Key, Snapshot, Written, read_latest};

use super::worker::THREAD_NAME;
use super::*;

const ID: &str = "abcdefghij12";

fn key() -> Key {
    Key::new([3; 32])
}

/// What the scripted terminal shows, and the threads that read it.
type Shown = Arc<Mutex<Option<Vec<u8>>>>;
type Names = Arc<Mutex<Vec<String>>>;
/// The writes the counting sink saw: the thread, whether it dropped the generations before, and how the program
/// ended if the snapshot said.
type Writes = Arc<Mutex<Vec<(String, bool, Option<String>)>>>;

/// A terminal that shows `screen`, and notes the thread each read came on.
struct Fake {
    screen: Shown,
    threads: Names,
}

impl Source for Fake {
    fn screen(&self) -> Option<Vec<u8>> {
        self.threads.lock().unwrap().push(thread_name());
        self.screen.lock().unwrap().clone()
    }
}

fn thread_name() -> String {
    format!("{:?}/{}", std::thread::current().id(), std::thread::current().name().unwrap_or("?"))
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
    fn write(&mut self, _key: &Key, snapshot: &Snapshot<'_>, only: bool) -> io::Result<Written> {
        std::thread::sleep(self.slow);
        let mut writes = self.writes.lock().unwrap();
        writes.push((thread_name(), only, snapshot.ended.map(str::to_owned)));
        Ok(Written { generation: writes.len() as u64, bytes: 10 })
    }
}

fn counting(slow: Duration) -> (Box<Counting>, Writes) {
    let writes = Arc::new(Mutex::new(Vec::new()));
    (Box::new(Counting { writes: writes.clone(), slow }), writes)
}

fn checkpointer(dir: &std::path::Path) -> Checkpointer {
    Checkpointer::new(dir.to_owned(), ID.to_owned(), Arc::new(|| {}))
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
    let mut cp = checkpointer(sessions.path());
    let t0 = Instant::now();
    assert!(!cp.is_on());
    assert_eq!(cp.pump(100, t0), (None, vec![]), "no deadline either");
    assert_eq!(cp.pump(200, t0 + secs(1000)), (None, vec![]));
    cp.request(true);
    cp.cleared();
    assert_eq!(cp.flush(secs(1)), Flushed::Off);
    assert_eq!(std::fs::read_dir(sessions.path()).unwrap().count(), 0, "not even the directory");
}

#[test]
fn the_work_is_done_on_the_one_checkpoint_thread_and_nowhere_else() {
    let (source, _, reads) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::ZERO);
    let mut cp = checkpointer(&std::env::temp_dir());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), t0);
    cp.pump(0, t0);
    cp.pump(500, t0 + secs(1));
    cp.pump(500, t0 + secs(5));
    cp.request(true);
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    wait_for("the writes", || writes.lock().unwrap().len() >= 3);
    let me = thread_name();
    let on_worker = |names: &[String]| names.iter().all(|n| n.ends_with(THREAD_NAME) && *n != me);
    assert!(on_worker(&reads.lock().unwrap()), "reads: {:?}", reads.lock().unwrap());
    let write_threads: Vec<String> = writes.lock().unwrap().iter().map(|w| w.0.clone()).collect();
    assert!(on_worker(&write_threads), "writes: {write_threads:?}");
    assert!(!reads.lock().unwrap().is_empty());
}

/// A new key goes over the channel to the thread that is there; it does not start another.
#[test]
fn a_new_key_does_not_start_a_second_thread() {
    let (source, _, _) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::ZERO);
    let mut cp = checkpointer(&std::env::temp_dir());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), t0);
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    for byte in [4u8, 5, 6] {
        let (other_source, _, _) = fake(Some(b"other"));
        let (other_sink, _) = counting(Duration::ZERO);
        cp.configure_with(Box::new(other_source), other_sink, Some(Key::new([byte; 32])), t0);
        assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    }
    let threads: std::collections::HashSet<String> =
        writes.lock().unwrap().iter().map(|w| w.0.clone()).collect();
    assert_eq!(writes.lock().unwrap().len(), 4, "all four went through the first sink");
    assert_eq!(threads.len(), 1, "one thread: {threads:?}");
}

#[test]
fn a_checkpoint_is_due_once_after_quiet_and_idle_adds_none() {
    let (source, _, _) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::ZERO);
    let mut cp = checkpointer(&std::env::temp_dir());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), t0);
    assert_eq!(cp.pump(0, t0).0, Some(t0 + schedule::QUIET));
    assert_eq!(writes.lock().unwrap().len(), 0, "not before");
    cp.pump(0, t0 + secs(4));
    wait_for("the first checkpoint", || writes.lock().unwrap().len() == 1);
    assert_eq!(cp.pump(0, t0 + secs(5)).0, None);
    cp.pump(0, t0 + secs(7200));
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(writes.lock().unwrap().len(), 1);
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
    let mut cp = checkpointer(&std::env::temp_dir());
    cp.configure_with(Box::new(source), sink, Some(key()), Instant::now());
    let start = Instant::now();
    assert_eq!(cp.flush(FLUSH_WITHIN), Flushed::TimedOut);
    let took = start.elapsed();
    assert!(took >= FLUSH_WITHIN && took < FLUSH_WITHIN + Duration::from_millis(300), "{took:?}");
}

/// A real directory and a screen of the most a checkpoint holds: sealed, written and committed well inside the
/// second.
#[test]
fn a_flush_of_a_full_size_screen_finishes_well_inside_its_bound() {
    let sessions = tempfile::tempdir().unwrap();
    let big: Vec<u8> = b"a line of output that is not short\r\n"
        .iter()
        .copied()
        .cycle()
        .take(wormhole_wire::SEED_MAX)
        .collect();
    let (source, _, _) = fake(Some(&big));
    let mut cp = checkpointer(sessions.path());
    cp.configure(Box::new(source), Some(key()), Instant::now());
    let start = Instant::now();
    let flushed = cp.flush(FLUSH_WITHIN);
    let took = start.elapsed();
    assert!(matches!(flushed, Flushed::Done(bytes) if bytes > big.len() as u64), "{flushed:?}");
    assert!(took < Duration::from_millis(900), "a flush of {} bytes took {took:?}", big.len());
    let read = read_latest(&sessions.path().join(ID), ID, Some(&key())).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::len), Some(big.len()));
}

/// Told to stop (the feature went off), the host writes nothing more, whatever is printed or asked.
#[test]
fn an_empty_key_stops_the_writing() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, _, _) = fake(Some(b"screen"));
    let mut cp = checkpointer(sessions.path());
    let t0 = Instant::now();
    cp.configure(Box::new(source), Some(key()), t0);
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let (source, _, _) = fake(Some(b"screen"));
    cp.configure(Box::new(source), None, t0);
    assert!(!cp.is_on());
    assert_eq!(cp.pump(999, t0 + secs(100)), (None, vec![]));
    cp.request(true);
    cp.cleared();
    assert_eq!(cp.flush(secs(1)), Flushed::Off);
    let generations = std::fs::read_dir(sessions.path().join(ID)).unwrap().count();
    assert_eq!(generations, 2, "the one generation it wrote (manifest and screen), no more");
    // And a key again starts it again.
    let (source, _, _) = fake(Some(b"screen"));
    cp.configure(Box::new(source), Some(key()), t0);
    assert!(cp.is_on() && matches!(cp.flush(secs(2)), Flushed::Done(_)));
}

/// A closed terminal leaves nothing: what was queued is dropped, the program's exit afterwards writes nothing,
/// and a key sent later does not bring it back.
#[test]
fn a_closed_terminal_writes_nothing_more_whatever_was_queued() {
    let (source, _, _) = fake(Some(b"screen"));
    let (sink, writes) = counting(Duration::from_millis(100));
    let mut cp = checkpointer(&std::env::temp_dir());
    let t0 = Instant::now();
    cp.configure_with(Box::new(source), sink, Some(key()), t0);
    cp.request(true);
    cp.request(true);
    cp.request(true);
    cp.close();
    cp.ended(&wire::ExitInfo { code: 0, signal: 1, at_ms: 1 });
    assert_eq!(cp.flush(secs(1)), Flushed::Off);
    let (source, _, _) = fake(Some(b"again"));
    cp.configure(Box::new(source), Some(key()), t0);
    assert!(!cp.is_on(), "closed for good");
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        writes.lock().unwrap().len() <= 1,
        "only the job already running at the close: {:?}",
        writes.lock().unwrap()
    );
}

#[test]
fn a_full_screen_program_leaves_the_last_screen_in_place() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, screen, _) = fake(Some(b"the prompt"));
    let mut cp = checkpointer(sessions.path());
    cp.configure(Box::new(source), Some(key()), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(n) if n > 0));
    *screen.lock().unwrap() = None; // vim is up
    assert_eq!(cp.flush(secs(2)), Flushed::Done(0), "nothing written");
    let read = read_latest(&sessions.path().join(ID), ID, Some(&key())).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"the prompt"[..]));
}

/// A program that exits leaves a checkpoint that says how, written without making the driver wait; the host keeps
/// it, and every earlier one it still holds. Nothing the host does removes the directory.
#[test]
fn a_program_that_ended_leaves_a_checkpoint_that_says_so_and_the_host_keeps_it() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, _, _) = fake(Some(b"last words"));
    let mut cp = checkpointer(sessions.path());
    cp.configure(Box::new(source), Some(key()), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let start = Instant::now();
    cp.ended(&wire::ExitInfo { code: 0, signal: 0, at_ms: 1 });
    assert!(start.elapsed() < Duration::from_millis(50), "the driver does not wait for the disk");
    // Later writes (a flush at SIGTERM) go on saying so; the flush queues behind the exit's write.
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    drop(cp);
    let dir = sessions.path().join(ID);
    assert!(dir.is_dir(), "dropping the checkpointer removes nothing");
    let read = read_latest(&dir, ID, Some(&key())).unwrap();
    assert_eq!(read.manifest.ended.as_deref(), Some("exit:0"));
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"last words"[..]));
    assert_eq!(ended_text(&wire::ExitInfo { code: 0, signal: 1, at_ms: 2 }), "signal:1");
}

/// `Checkpoint{flush: true}` is acknowledged with the generation when it is on disk; a request without it is not,
/// and writes only if something was printed.
#[test]
fn a_flush_request_is_acknowledged_and_a_plain_one_is_not() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, _, _) = fake(Some(b"screen"));
    let mut cp = checkpointer(sessions.path());
    let t0 = Instant::now();
    cp.configure(Box::new(source), Some(key()), t0);
    cp.request(true);
    let mut acks = Vec::new();
    wait_for("the acknowledgement", || {
        acks.extend(cp.pump(0, t0).1);
        !acks.is_empty()
    });
    assert_eq!(acks, [1]);
    assert!(
        read_latest(&sessions.path().join(ID), ID, Some(&key())).unwrap().manifest.generation == 1,
        "it was on disk when it was said"
    );
    // The first checkpoint was already asked for (the flush took its place); a plain request with nothing
    // printed since writes nothing and says nothing.
    cp.request(false);
    std::thread::sleep(Duration::from_millis(100));
    assert!(cp.pump(0, t0 + secs(1)).1.is_empty());
    assert_eq!(std::fs::read_dir(sessions.path().join(ID)).unwrap().count(), 2, "one generation");
    // Something printed: a plain request writes, still without a word.
    cp.pump(50, t0 + secs(1));
    cp.request(false);
    wait_for("the second generation", || std::fs::read_dir(sessions.path().join(ID)).unwrap().count() == 4);
    assert!(cp.pump(50, t0 + secs(2)).1.is_empty());
}

/// Clear Buffer: a generation after the clear, and none from before it.
#[test]
fn clearing_the_buffer_writes_a_new_generation_and_drops_the_old_ones() {
    let sessions = tempfile::tempdir().unwrap();
    let (source, screen, _) = fake(Some(b"secret scrollback"));
    let mut cp = checkpointer(sessions.path());
    cp.configure(Box::new(source), Some(key()), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    *screen.lock().unwrap() = Some(b"fresh".to_vec());
    cp.cleared();
    let dir = sessions.path().join(ID);
    wait_for("the cleared generation alone", || std::fs::read_dir(&dir).is_ok_and(|d| d.count() == 2));
    let read = read_latest(&dir, ID, Some(&key())).unwrap();
    assert_eq!(read.screen.as_deref().map(Vec::as_slice), Some(&b"fresh"[..]));
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        assert!(!std::fs::read(entry.path()).unwrap().windows(6).any(|w| w == b"secret"));
    }
}

#[test]
fn a_new_key_keeps_what_was_said_of_the_end() {
    let sessions = tempfile::tempdir().unwrap();
    let mut cp = checkpointer(sessions.path());
    let (source, _, _) = fake(Some(b"one"));
    cp.configure(Box::new(source), Some(key()), Instant::now());
    cp.ended(&wire::ExitInfo { code: 2, signal: 0, at_ms: 1 });
    let (source, _, _) = fake(Some(b"two"));
    cp.configure(Box::new(source), Some(Key::new([4; 32])), Instant::now());
    assert!(matches!(cp.flush(secs(2)), Flushed::Done(_)));
    let read = read_latest(&sessions.path().join(ID), ID, Some(&Key::new([4; 32]))).unwrap();
    assert_eq!(read.manifest.ended.as_deref(), Some("exit:2"));
}
