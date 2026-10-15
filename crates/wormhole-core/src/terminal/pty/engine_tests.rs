//! The PTY layer driving the engine, end to end: what a program's output, a flood of it and
//! a resize do to the screen the UI reads.

use std::time::{Duration, Instant};

use crate::terminal::{GridSize, Pty, PtyEvent, StartOptions};

const SIZE: GridSize = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 16.0 };

fn spawn(script: &str) -> (Pty, crate::terminal::PtyEvents) {
    Pty::spawn_program(SIZE, &StartOptions::default(), "/bin/sh", &["-c", script]).expect("spawn")
}

fn wait_until(what: &str, within: Duration, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + within;
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The parser holds what a synchronized update (mode 2026) prints back until it ends, and gives
/// up on an update that never ends after its timeout: the reader has to wake for that with no
/// output arriving to wake it.
#[test]
fn a_synchronized_update_that_never_ends_is_shown_when_its_timeout_passes() {
    let (pty, _events) =
        spawn(r"printf '\033[?2026h'; printf 'held back'; sleep 4; printf '\033[?2026l'; sleep 5");
    let started = Instant::now();
    wait_until("the update's text", Duration::from_secs(3), || pty.screen_text().contains("held back"));
    assert!(
        started.elapsed() < Duration::from_millis(3500),
        "the update ended after 4 s, this is its timeout"
    );
}

#[test]
fn a_flood_does_not_keep_the_ui_from_the_screen() {
    // 50 MB of log lines, parsed by the real parser.
    let line = "The quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnopqrstuvwxyz";
    let (pty, events) = spawn(&format!("yes '{line}' | head -c 52428800; echo; echo flood-done"));
    let mut waits = Vec::new();
    let mut snapshots = 0;
    let mut snapshot = crate::terminal::ScreenSnapshot::default();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        // What a frame does: the lock for a copy of the screen, and the lock to ask for a mode.
        let asked = Instant::now();
        let _ = pty.mode();
        waits.push(asked.elapsed());
        let asked = Instant::now();
        pty.snapshot(&mut snapshot, 0);
        waits.push(asked.elapsed());
        snapshots += 1;
        if let Some(PtyEvent::Exited(code)) = events.try_recv() {
            assert_eq!(code, Some(0));
            break;
        }
        assert!(Instant::now() < deadline, "the flood did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(pty.screen_text().contains("flood-done"), "the end of the output arrived");
    waits.sort_unstable();
    let (median, p99, worst) =
        (waits[waits.len() / 2], waits[waits.len() * 99 / 100], waits[waits.len() - 1]);
    eprintln!(
        "{snapshots} frames during the flood: lock and copy median {median:?}, p99 {p99:?}, worst {worst:?}"
    );
    // A frame's copy of the screen takes tens of microseconds; the reader's slice at most 0.4 ms.
    // The bounds are loose for a loaded machine and still far below a flood's length (a second).
    assert!(median < Duration::from_millis(2), "median {median:?}");
    assert!(p99 < Duration::from_millis(25), "p99 {p99:?}");
    assert!(worst < Duration::from_millis(500), "worst {worst:?}");
}

#[test]
fn resizing_changes_the_screen_and_the_programs_size() {
    let script = "trap 'echo size:$(stty size)' WINCH; echo ready; while :; do sleep 0.05; done";
    let (mut pty, _events) = spawn(script);
    wait_until("the program", Duration::from_secs(10), || pty.screen_text().contains("ready"));
    pty.resize(GridSize { cols: 120, rows: 40, cell_width: 8.0, cell_height: 16.0 });
    wait_until("the program's view of the size", Duration::from_secs(10), || {
        pty.screen_text().contains("size:40 120")
    });
    let mut snapshot = crate::terminal::ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert_eq!((snapshot.columns(), snapshot.rows().len()), (120, 40));
}
