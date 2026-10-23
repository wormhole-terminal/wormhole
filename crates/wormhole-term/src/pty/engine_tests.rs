//! The PTY layer driving the engine, end to end: what a program's output, a flood of it and
//! a resize do to the screen the UI reads.

use std::time::{Duration, Instant};

use std::sync::Arc;

use super::{FairLock, Flow, Sink};
use crate::engine::OwnFeed;
use crate::events::{EventProxy, PtyEvents};
use crate::screen::Screen;
use crate::{GridSize, Pty, PtyEvent, ScreenSnapshot, StartOptions};

const SIZE: GridSize = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 16.0 };

fn spawn(script: &str) -> (Pty, crate::PtyEvents) {
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

/// The engine's own screen behind the reader's sink, fed by hand: no PTY, no clock but the
/// update's own.
fn engine_sink() -> (Sink<Screen, OwnFeed>, Arc<FairLock<Screen>>, PtyEvents) {
    let (proxy, events) = EventProxy::new();
    let screen = Arc::new(FairLock::new(Screen::new(SIZE, 100)));
    let sink = Sink::new(screen.clone(), OwnFeed::new(proxy.clone(), screen.clone()), proxy);
    (sink, screen, events)
}

/// Parse `bytes` and say whether the UI was woken, taking the wakeup so the next one counts.
fn woke(sink: &mut Sink<Screen, OwnFeed>, events: &PtyEvents, bytes: &[u8]) -> bool {
    sink.parse(bytes, || Flow::Continue);
    events.try_recv().is_some()
}

#[test]
fn a_synchronized_update_wakes_the_ui_when_it_begins_and_ends_and_not_between() {
    let (mut sink, screen, events) = engine_sink();
    assert!(woke(&mut sink, &events, b"before \x1b[?2026h"));
    assert!(screen.lock().held_back());
    for part in ["\x1b[2Jone ", "two ", "three"] {
        assert!(!woke(&mut sink, &events, part.as_bytes()), "{part:?} inside the update");
    }
    assert!(woke(&mut sink, &events, b"\x1b[?2026l"), "its end shows the screen");
    assert!(!screen.lock().held_back());
}

#[test]
fn a_begin_inside_an_update_and_an_end_without_one_change_nothing_else() {
    let (mut sink, screen, events) = engine_sink();
    assert!(woke(&mut sink, &events, b"\x1b[?2026h"));
    assert!(!woke(&mut sink, &events, b"\x1b[?2026hx\x1b[?2026h"), "repeated begins are one update");
    assert!(woke(&mut sink, &events, b"\x1b[?2026l"), "one end ends it");
    assert!(!screen.lock().held_back());
    assert!(woke(&mut sink, &events, b"\x1b[?2026l"), "an end with nothing begun is plain output");
    assert!(woke(&mut sink, &events, b"\x1b[?2026hy\x1b[?2026l"), "begun and ended in one slice");
    assert!(!screen.lock().held_back());
}

#[test]
fn an_update_that_gave_up_lets_the_next_slice_wake_the_ui_again() {
    let (mut sink, screen, events) = engine_sink();
    assert!(woke(&mut sink, &events, b"\x1b[?2026hheld"));
    assert!(!woke(&mut sink, &events, b"more"));
    std::thread::sleep(crate::screen::SYNC_TIMEOUT + Duration::from_millis(30));
    sink.expire_sync();
    assert!(events.try_recv().is_some(), "the timeout wakes the UI");
    assert!(!screen.lock().held_back());
    // The program never ended it, and its next output is drawn as it comes.
    assert!(woke(&mut sink, &events, b"later"));
    assert!(woke(&mut sink, &events, b"and later"));
}

#[test]
fn a_synchronized_update_that_never_ends_wakes_the_ui_again_when_it_times_out() {
    // No output arrives after the first read: only the timeout can wake the UI for the second time.
    let (pty, events) = spawn(r"printf '\033[?2026h'; printf 'held back'; sleep 5");
    wait_until("the first wakeup", Duration::from_secs(10), || {
        matches!(events.try_recv(), Some(PtyEvent::Output))
    });
    wait_until("the timeout's wakeup", Duration::from_secs(10), || {
        matches!(events.try_recv(), Some(PtyEvent::Output))
    });
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 'h', "the screen is shown now");
}

#[test]
fn events_a_program_sends_inside_a_synchronized_update_still_arrive() {
    // A bell and a title, between the update's begin and the end it never sends.
    let (_pty, events) = spawn(r"printf '\033[?2026h\007\033]0;in-update\007'; sleep 5");
    let (mut bell, mut title) = (false, false);
    wait_until("the bell and the title", Duration::from_secs(10), || {
        match events.try_recv() {
            Some(PtyEvent::Bell) => bell = true,
            Some(PtyEvent::Title(text)) => title = text == "in-update",
            _ => {}
        }
        bell && title
    });
}

#[test]
fn a_flood_does_not_keep_the_ui_from_the_screen() {
    // 50 MB of log lines, parsed by the real parser.
    let line = "The quick brown fox jumps over the lazy dog 0123456789 abcdefghijklmnopqrstuvwxyz";
    let (pty, events) = spawn(&format!("yes '{line}' | head -c 52428800; echo; echo flood-done"));
    let mut waits = Vec::new();
    let mut snapshots = 0;
    let mut snapshot = crate::ScreenSnapshot::default();
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
    let mut snapshot = crate::ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert_eq!((snapshot.columns(), snapshot.rows().len()), (120, 40));
}
