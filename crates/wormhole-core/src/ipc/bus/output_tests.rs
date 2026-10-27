//! The output worker: pulses in, `output` events out, in the time and the size the design says.

use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_term::ScreenReader;
use wormhole_term::testing::TestTerm;

use super::{Bus, Grant, Next, Subscription};
use crate::ipc::control::event::{Body, EventsSpec};

fn reader() -> ScreenReader {
    TestTerm::new(40, 5, 1000).into_reader()
}

fn watch(bus: &Arc<Bus>, terminals: Vec<i64>, grant: Grant) -> Subscription {
    let spec = EventsSpec { topics: vec!["output".into()], terminals, ..EventsSpec::default() };
    bus.subscribe(spec.filter().expect("valid"), grant, None).expect("room").0
}

fn reads_all() -> Grant {
    Grant { reads_all: true, own: vec![], ..Grant::default() }
}

/// The next `output` event within `wait`.
fn heard(subscription: &Subscription, wait: Duration) -> Option<(Option<i64>, Body)> {
    match subscription.next(wait) {
        Next::Event(published) => Some((published.event.terminal, published.event.body.clone())),
        _ => None,
    }
}

fn lines(body: &Body) -> Vec<&str> {
    match body {
        Body::Output { lines, .. } => lines.iter().map(String::as_str).collect(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn what_a_terminal_prints_is_told_soon_with_its_ids_and_the_cursors_line() {
    let bus = Bus::new();
    let subscription = watch(&bus, vec![], reads_all());
    let screen = reader();
    screen.feed(b"before\r\n$ ");
    bus.watch_output(1, 7, &screen);
    screen.feed(b"ls\r\na.rs\r\nb.rs\r\n$ ");
    bus.pulse(1, 7, || Some(screen.downgrade()));
    let (terminal, body) = heard(&subscription, Duration::from_secs(2)).expect("an output event");
    assert_eq!(terminal, Some(7));
    assert_eq!(lines(&body), ["$ ls", "a.rs", "b.rs"]);
    let Body::Output { screen: kind, skipped, lost, .. } = &body else { panic!() };
    assert_eq!((kind.as_str(), *skipped, *lost), ("main", 0, false));
    assert!(heard(&subscription, Duration::from_millis(300)).is_none(), "nothing more was printed");
}

#[test]
fn a_busy_terminal_is_told_at_most_every_hundred_milliseconds() {
    let bus = Bus::new();
    let subscription = watch(&bus, vec![], reads_all());
    let screen = reader();
    bus.watch_output(1, 2, &screen);
    let start = Instant::now();
    let mut told = Vec::new();
    let mut n = 0;
    // Output every 5 ms for 450 ms, each with a pulse.
    while start.elapsed() < Duration::from_millis(450) {
        screen.feed(format!("line {n}\r\n").as_bytes());
        bus.pulse(1, 2, || Some(screen.downgrade()));
        n += 1;
        std::thread::sleep(Duration::from_millis(5));
        while let Some((_, body)) = heard(&subscription, Duration::ZERO) {
            told.push((start.elapsed(), body));
        }
    }
    while let Some((_, body)) = heard(&subscription, Duration::from_millis(250)) {
        told.push((start.elapsed(), body));
    }
    assert!((2..=7).contains(&told.len()), "{} events for {n} lines", told.len());
    assert!(told[0].0 < Duration::from_millis(60), "the first is not held back: {:?}", told[0].0);
    let gaps: Vec<Duration> = told.windows(2).map(|pair| pair[1].0 - pair[0].0).collect();
    assert!(gaps.iter().all(|gap| *gap >= Duration::from_millis(80)), "{gaps:?}");
    // Nothing lost, nothing twice.
    let all: Vec<&str> = told.iter().flat_map(|(_, body)| lines(body)).collect();
    assert_eq!(all.len(), n);
    assert!(all.iter().enumerate().all(|(at, line)| *line == format!("line {at}")));
}

#[test]
fn a_full_screen_program_is_told_as_the_screen_changing_and_a_closed_terminal_costs_nothing() {
    let bus = Bus::new();
    let subscription = watch(&bus, vec![], reads_all());
    let screen = reader();
    bus.watch_output(1, 4, &screen);
    screen.feed(b"\x1b[?1049h\x1b[2JVIM");
    bus.pulse(1, 4, || Some(screen.downgrade()));
    // The test screen has no PTY thread to bump the revision, so the look sees an alternate screen
    // that "did not change"; the event is for the real revision counter, tested with a window.
    let told = heard(&subscription, Duration::from_millis(400));
    assert!(told.is_none() || matches!(told, Some((_, Body::Output { ref screen, .. })) if screen == "alt"));
    let weak = screen.downgrade();
    drop(screen);
    assert!(weak.upgrade().is_none(), "the worker keeps no closed terminal alive");
    bus.pulse(1, 4, || weak.upgrade().map(|s| s.downgrade()));
}

#[test]
fn only_those_who_may_read_the_terminal_hear_it_and_nothing_is_built_for_nobody() {
    let bus = Bus::new();
    let screen = reader();
    let asked = Cell::new(0);
    let pulse = |bus: &Arc<Bus>, terminal: i64| {
        bus.pulse(1, terminal, || {
            asked.set(asked.get() + 1);
            Some(screen.downgrade())
        });
    };
    pulse(&bus, 3);
    assert_eq!(asked.get(), 0, "with nobody listening the handle is never made");
    let stranger = watch(&bus, vec![], Grant::default());
    let own = watch(&bus, vec![3], Grant { reads_all: false, own: vec![3], ..Grant::default() });
    assert!(bus.wants(super::Topic::Output));
    assert!(!bus.output_wanted(1, 9), "the stranger may read nothing and the owner asked for 3 only");
    pulse(&bus, 9);
    assert_eq!(asked.get(), 0, "nobody may read terminal 9");
    bus.watch_output(1, 3, &screen);
    screen.feed(b"secret\r\n");
    pulse(&bus, 3);
    let (terminal, body) = heard(&own, Duration::from_secs(2)).expect("the terminal's own caller hears it");
    assert_eq!((terminal, lines(&body)), (Some(3), vec!["secret"]));
    assert!(heard(&stranger, Duration::from_millis(200)).is_none(), "and nobody else");
    // The setting goes off: a stream that needed it ends; the terminal's own caller keeps hearing.
    let wide = watch(&bus, vec![], reads_all());
    bus.revoke_reads();
    assert!(matches!(wide.next(Duration::from_millis(50)), Next::Closed(_)));
    assert!(bus.output_wanted(1, 3));
}

#[test]
fn the_last_watcher_leaving_stops_the_work() {
    let bus = Bus::new();
    let screen = reader();
    let subscription = watch(&bus, vec![], reads_all());
    bus.watch_output(1, 5, &screen);
    drop(subscription);
    assert!(!bus.wants(super::Topic::Output));
    let made = Cell::new(false);
    bus.pulse(1, 5, || {
        made.set(true);
        Some(screen.downgrade())
    });
    assert!(!made.get());
    // Dropping the bus ends the worker: nothing to wait for, and nothing panics.
    drop(bus);
}

#[test]
fn a_second_watcher_does_not_wipe_what_the_first_has_not_been_told() {
    let bus = Bus::new();
    let first = watch(&bus, vec![], reads_all());
    let screen = reader();
    bus.watch_output(1, 6, &screen);
    screen.feed(b"not yet told\r\n");
    // Another stream begins before the worker has looked.
    let second = watch(&bus, vec![], reads_all());
    bus.watch_output(1, 6, &screen);
    bus.pulse(1, 6, || Some(screen.downgrade()));
    let (_, body) = heard(&first, Duration::from_secs(2)).expect("the first stream is told");
    assert_eq!(lines(&body), ["not yet told"]);
    let (_, body) = heard(&second, Duration::from_secs(2)).expect("so is the second");
    assert_eq!(lines(&body), ["not yet told"]);
}

#[test]
fn a_terminal_first_heard_of_by_its_output_is_told_from_its_start() {
    let bus = Bus::new();
    let subscription = watch(&bus, vec![], reads_all());
    let screen = reader();
    screen.feed(b"first words\r\n");
    bus.pulse(1, 8, || Some(screen.downgrade()));
    let (_, body) = heard(&subscription, Duration::from_secs(2)).expect("told");
    assert_eq!(lines(&body), ["first words"]);
}

#[test]
fn a_stream_that_begins_right_after_the_last_one_ended_keeps_its_place() {
    let bus = Bus::new();
    let screen = reader();
    drop(watch(&bus, vec![], reads_all()));
    let subscription = watch(&bus, vec![], reads_all());
    bus.watch_output(1, 2, &screen);
    // Give the worker time to act on the reset the first stream's end left.
    std::thread::sleep(Duration::from_millis(150));
    screen.feed(b"after the reset\r\n");
    bus.pulse(1, 2, || Some(screen.downgrade()));
    let (_, body) = heard(&subscription, Duration::from_secs(2)).expect("told");
    assert_eq!(lines(&body), ["after the reset"]);
}

/// A terminal in the stream's scope (the program opened it, or the user said yes about it) is not the one the
/// program runs inside: hiding it from agents stops what the stream is told of it.
#[test]
fn hiding_a_terminal_in_a_streams_scope_stops_its_output_for_it() {
    let bus = Bus::new();
    let screen = reader();
    let scoped = watch(&bus, vec![], Grant { readable: vec![4], ..Grant::default() });
    let inside = watch(&bus, vec![], Grant { own: vec![4], ..Grant::default() });
    bus.watch_output(1, 4, &screen);
    screen.feed(b"first\r\n");
    bus.pulse(1, 4, || Some(screen.downgrade()));
    assert!(heard(&scoped, Duration::from_secs(2)).is_some(), "told while it is not hidden");
    assert!(heard(&inside, Duration::from_secs(2)).is_some());
    bus.set_hidden(4, true);
    screen.feed(b"second\r\n");
    bus.pulse(1, 4, || Some(screen.downgrade()));
    let (terminal, body) = heard(&inside, Duration::from_secs(2)).expect("a program inside still hears it");
    assert_eq!((terminal, lines(&body)), (Some(4), vec!["second"]));
    assert!(heard(&scoped, Duration::from_millis(300)).is_none(), "nothing more arrives once it is hidden");
}
