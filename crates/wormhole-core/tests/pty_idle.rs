//! A terminal that has nothing to do costs nothing: the reader thread sleeps in its poller while
//! the program is quiet, while it has not read what was typed or pasted (the master has no room
//! for it, which a level-triggered poller must not report again and again), and after the
//! program's output ended with a job still holding the terminal.
//!
//! Its own test binary with one test, so that the process's CPU time is the reader's.
#![allow(unsafe_code)] // only the getrusage call below

use std::time::Duration;

use wormhole_core::shells::ShellCommand;
use wormhole_core::terminal::{GridSize, Pty, SpawnOptions};

const SIZE: GridSize = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 16.0 };

/// CPU time this process has used, user and system.
fn cpu_time() -> Duration {
    // SAFETY: getrusage fills the struct, for which all-zero is a valid value.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut usage), 0);
        usage
    };
    let time = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    time(usage.ru_utime) + time(usage.ru_stime)
}

fn terminal(script: &str) -> Pty {
    let options = SpawnOptions {
        shell: Some(ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            name: "sh".into(),
        }),
        ..SpawnOptions::default()
    };
    Pty::spawn(SIZE, &options).expect("starts").0
}

/// CPU time used over the next second, after a moment for the terminal to settle.
fn idle_cost() -> Duration {
    std::thread::sleep(Duration::from_millis(500));
    let before = cpu_time();
    std::thread::sleep(Duration::from_secs(1));
    cpu_time() - before
}

#[test]
fn a_terminal_with_nothing_to_do_uses_no_cpu() {
    // The generous bound is for a loaded machine; a polling loop at 100% costs a second.
    let bound = Duration::from_millis(100);

    let quiet = terminal("sleep 10");
    let cost = idle_cost();
    assert!(cost < bound, "a quiet terminal used {cost:?} of CPU in a second");

    // A paste the program does not read: it sits in the queue, and the rest waits for room.
    quiet.write(vec![b'x'; 300_000]);
    let cost = idle_cost();
    assert!(cost < bound, "a terminal with unread input used {cost:?} of CPU in a second");
    drop(quiet);

    // The shell is gone and a job that ignores the hangup holds the terminal.
    let orphaned = terminal("trap '' HUP; sleep 4 & echo started; exit 0");
    let cost = idle_cost();
    assert!(cost < bound, "a terminal whose shell exited used {cost:?} of CPU in a second");
    drop(orphaned);
}
