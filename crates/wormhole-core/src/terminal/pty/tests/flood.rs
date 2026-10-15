//! Volume: a paste larger than the kernel queues, floods of output, input while output floods.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{Control, SIZE, launch, run};
use crate::terminal::events::{EventProxy, PtyEvent, PtyEvents};
#[cfg(target_os = "macos")]
use crate::terminal::pty::exit;
#[cfg(target_os = "macos")]
use crate::terminal::pty::master::Key;
use crate::terminal::pty::{FairLock, Feed, start};

#[test]
fn a_paste_larger_than_the_terminal_takes_does_not_stall_the_output() {
    // 200 KB of lines to `wc`, far beyond what the kernel queues for a program that has not
    // read it yet. The terminal echoes all of it back at once, so the reader must keep reading
    // while the rest of the paste waits for the master to be writable.
    let line = format!("{}\n", "0123456789".repeat(9));
    let paste = line.repeat(2000);
    let run = run("/bin/sh", &["-c", "wc -c"]);
    let started = Instant::now();
    run.control.input(paste.clone().into_bytes().into());
    run.control.input(b"\x04".as_slice().into());
    assert!(started.elapsed() < Duration::from_millis(500), "handing over the paste waited");
    run.wait_for(&format!("{}\r\n", paste.len()));
    assert_eq!(run.exit(Duration::from_secs(10)), Some(Some(0)));
}

/// A screen that counts what it is fed and takes `NS_PER_KIB` nanoseconds of work for every
/// kilobyte, about what a parser does (4 to 7 µs per KiB).
struct Count;

impl Count {
    const NS_PER_KIB: u128 = 5_000;
}

impl Feed<usize> for Count {
    fn feed(&mut self, screen: &mut usize, bytes: &[u8]) {
        *screen += bytes.len();
        let until =
            Instant::now() + Duration::from_nanos((bytes.len() as u128 * Self::NS_PER_KIB / 1024) as u64);
        while Instant::now() < until {
            std::hint::spin_loop();
        }
    }
}

fn run_counting(script: &str) -> (Control, Arc<FairLock<usize>>, PtyEvents) {
    let (proxy, events) = EventProxy::new();
    let screen = Arc::new(FairLock::new(0));
    let started =
        start(&launch("/bin/sh", &["-c", script]), SIZE, screen.clone(), Count, proxy).expect("starts");
    (started.control, screen, events)
}

#[test]
fn a_50_mb_flood_is_read_completely_and_the_ui_gets_the_lock_between_slices() {
    // Exactly 50 MiB, no newline in it for the line discipline to expand.
    let script = "dd if=/dev/zero bs=1048576 count=50 2>/dev/null | tr '\\000' x";
    let (_control, screen, events) = run_counting(script);
    // A UI that wants the screen every millisecond, as a frame loop does: how long it waits.
    let mut waits = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    let exited = loop {
        let asked = Instant::now();
        drop(screen.lock());
        waits.push(asked.elapsed());
        if let Some(PtyEvent::Exited(code)) = events.try_recv() {
            break code;
        }
        assert!(Instant::now() < deadline, "the flood did not finish");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(exited, Some(0));
    assert_eq!(*screen.lock(), 50 << 20, "every byte was parsed");
    waits.sort_unstable();
    let (median, p99, worst) =
        (waits[waits.len() / 2], waits[waits.len() * 99 / 100], waits[waits.len() - 1]);
    eprintln!("{} UI locks during the flood: median {median:?}, p99 {p99:?}, worst {worst:?}", waits.len());
    // A slice takes about 80 µs here. Generous, for a loaded machine; a lock that favoured the
    // reader made the UI wait for the whole flood (seconds).
    assert!(p99 < Duration::from_millis(20), "p99 {p99:?}");
    assert!(worst < Duration::from_millis(500), "worst {worst:?}");
}

#[test]
fn input_is_written_between_slices_of_a_flood_the_program_never_reads_input_of() {
    // `yes` prints for ever and reads nothing; ^C has to get through to the line discipline
    // while the reader is busy with output.
    let (control, screen, events) = run_counting("exec yes 'the quick brown fox jumps over the lazy dog'");
    let deadline = Instant::now() + Duration::from_secs(10);
    while *screen.lock() < (1 << 20) {
        assert!(Instant::now() < deadline, "no flood");
        std::thread::sleep(Duration::from_millis(5));
    }
    control.input(b"\x03".as_slice().into());
    let exited = loop {
        match events.try_recv() {
            Some(PtyEvent::Exited(code)) => break code,
            Some(_) => {}
            None => {
                assert!(Instant::now() < deadline, "^C did not stop the flood");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };
    assert_eq!(exited, None, "killed by SIGINT");
}

#[cfg(target_os = "macos")]
#[test]
fn the_poller_reports_a_childs_exit() {
    use std::process::Command;
    // Without this the exit of a shell that leaves a job holding the terminal would be noticed
    // by polling, 100 ms late at best.
    let poller = polling::Poller::new().expect("a poller");
    let mut child = Command::new("/bin/sleep").arg("0.2").spawn().expect("starts");
    assert_eq!(exit::watch(&poller, &child), exit::Watch::ByPoller, "the system refused to watch the child");
    let mut events = polling::Events::new();
    let started = Instant::now();
    poller.wait(&mut events, Some(Duration::from_secs(5))).expect("waits");
    assert!(events.iter().any(|event| event.key == Key::Exit.raw()), "no event for the exit");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(child.wait().expect("reaped").success());
    exit::unwatch(&poller, &child);
}
