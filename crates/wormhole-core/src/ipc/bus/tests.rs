use std::cell::Cell;
use std::time::Duration;

use super::{Bus, Grant, Next, QUEUE_ITEMS, Refused, STREAMS_MAX, STREAMS_PER_CALLER, Subscription};
use crate::ipc::control::event::{BlockPhase, Body, CloseReason, Event, EventFilter, EventsSpec, Topic};

const NONE: Duration = Duration::ZERO;

fn filter(topics: &[&str]) -> EventFilter {
    EventsSpec { topics: topics.iter().map(|t| t.to_string()).collect(), ..EventsSpec::default() }
        .filter()
        .expect("valid")
}

fn bell(terminal: i64) -> Event {
    Event { window: Some(1), terminal: Some(terminal), body: Body::Bell }
}

fn block(terminal: i64, command: &str) -> Event {
    Event {
        window: Some(1),
        terminal: Some(terminal),
        body: Body::Block {
            block: 1,
            phase: BlockPhase::Finished,
            exit_code: Some(0),
            took_ms: Some(5),
            command: Some(command.into()),
        },
    }
}

fn subscribe(bus: &std::sync::Arc<Bus>, topics: &[&str]) -> Subscription {
    bus.subscribe(filter(topics), Grant::default(), None).expect("room").0
}

fn seq_of(next: Next) -> u64 {
    match next {
        Next::Event(published) => published.seq,
        other => panic!("expected an event: {other:?}"),
    }
}

#[test]
fn publishing_with_nobody_listening_builds_nothing() {
    let bus = Bus::new();
    let built = Cell::new(0);
    for topic in Topic::ALL {
        assert!(!bus.wants(topic));
        bus.publish(topic, || {
            built.set(built.get() + 1);
            bell(1)
        });
    }
    assert_eq!(built.get(), 0, "the closure never ran");
}

#[test]
fn only_the_topics_somebody_listens_for_are_built() {
    let bus = Bus::new();
    let _watcher = subscribe(&bus, &["bell"]);
    let built = Cell::new(0);
    bus.publish(Topic::Status, || {
        built.set(built.get() + 1);
        bell(1)
    });
    assert_eq!(built.get(), 0, "nobody listens for status");
    assert!(bus.wants(Topic::Bell) && !bus.wants(Topic::Status));
    bus.publish(Topic::Bell, || {
        built.set(built.get() + 1);
        bell(1)
    });
    assert_eq!(built.get(), 1);
}

#[test]
fn interest_falls_to_nothing_when_the_last_subscription_drops() {
    let bus = Bus::new();
    let first = subscribe(&bus, &["bell"]);
    let second = subscribe(&bus, &["status", "bell"]);
    assert!(bus.wants(Topic::Status) && bus.wants(Topic::Bell));
    drop(second);
    assert!(bus.wants(Topic::Bell) && !bus.wants(Topic::Status), "the first still listens for the bell");
    drop(first);
    assert!(Topic::ALL.into_iter().all(|topic| !bus.wants(topic)));
    assert_eq!(bus.subscribers(), 0);
}

#[test]
fn streams_are_capped_in_all_and_for_each_caller() {
    let bus = Bus::new();
    let mut held = Vec::new();
    for _ in 0..STREAMS_PER_CALLER {
        held.push(bus.subscribe(filter(&[]), Grant::default(), Some(7)).expect("room").0);
    }
    let refused = bus.subscribe(filter(&[]), Grant::default(), Some(7)).err();
    assert_eq!(refused, Some(Refused::CallerFull));
    assert!(bus.subscribe(filter(&[]), Grant::default(), Some(8)).is_ok(), "another caller is not held back");
    // Fill the rest from many callers.
    for caller in 100..(100 + STREAMS_MAX as i64) {
        match bus.subscribe(filter(&[]), Grant::default(), Some(caller)) {
            Ok((subscription, _)) => held.push(subscription),
            Err(refused) => {
                assert_eq!(refused, Refused::Full);
                break;
            }
        }
    }
    assert_eq!(bus.subscribers(), STREAMS_MAX);
    assert_eq!(bus.subscribe(filter(&[]), Grant::default(), None).err(), Some(Refused::Full));
    // A place given back is a place to take.
    held.pop();
    assert!(bus.subscribe(filter(&[]), Grant::default(), None).is_ok());
    assert!(Refused::Full.to_string().contains("16") && Refused::CallerFull.to_string().contains('4'));
}

#[test]
fn every_subscriber_sees_one_increasing_order() {
    let bus = Bus::new();
    let (all, started) = bus.subscribe(filter(&[]), Grant::default(), None).expect("room");
    assert_eq!(started.seq, 0);
    assert_eq!(started.boot.len(), 16);
    let only_status = subscribe(&bus, &["status"]);
    for terminal in 1..=3 {
        bus.publish(Topic::Bell, || bell(terminal));
    }
    bus.publish(Topic::Status, || Event {
        window: Some(1),
        terminal: Some(1),
        body: Body::Status {
            status: crate::ipc::control::TerminalState::Working,
            was: crate::ipc::control::TerminalState::Idle,
            agent: None,
            message: None,
        },
    });
    let seen: Vec<u64> = (0..4).map(|_| seq_of(all.next(NONE))).collect();
    assert_eq!(seen, [1, 2, 3, 4]);
    // The other one only heard the status, with its place in the same order: a gap is a filter.
    assert_eq!(seq_of(only_status.next(NONE)), 4);
    assert!(matches!(only_status.next(NONE), Next::Idle));
    assert_eq!(only_status.last_seq(), 4);
    // A later subscription starts after what has been published.
    let (_late, started) = bus.subscribe(filter(&[]), Grant::default(), None).expect("room");
    assert_eq!(started.seq, 4);
}

#[test]
fn an_event_nobody_matches_takes_no_number() {
    let bus = Bus::new();
    let terminal_three = bus
        .subscribe(
            EventsSpec { terminals: vec![3], ..EventsSpec::default() }.filter().expect("valid"),
            Grant::default(),
            None,
        )
        .expect("room")
        .0;
    bus.publish(Topic::Bell, || bell(9));
    bus.publish(Topic::Bell, || bell(3));
    assert_eq!(seq_of(terminal_three.next(NONE)), 1);
}

#[test]
fn a_slow_reader_loses_its_oldest_and_is_told_before_the_next() {
    let bus = Bus::new();
    let slow = subscribe(&bus, &["bell"]);
    let extra = 5;
    for n in 0..(QUEUE_ITEMS + extra) {
        bus.publish(Topic::Bell, || bell(n as i64));
    }
    let Next::Lagged(dropped) = slow.next(NONE) else { panic!("told first") };
    assert_eq!(dropped as usize, extra);
    // What is left is the newest, in order, and the first of it is the one after those dropped.
    assert_eq!(seq_of(slow.next(NONE)) as usize, extra + 1);
}

#[test]
fn what_a_terminal_printed_goes_only_to_those_who_may_read_it() {
    let bus = Bus::new();
    let (nobody, _) = bus.subscribe(filter(&["blocks"]), Grant::default(), None).expect("room");
    let (itself, _) = bus
        .subscribe(filter(&["blocks"]), Grant { reads_all: false, own: vec![8], ..Grant::default() }, Some(8))
        .expect("room");
    let (everything, _) = bus
        .subscribe(filter(&["blocks"]), Grant { reads_all: true, own: vec![], ..Grant::default() }, None)
        .expect("room");
    bus.publish(Topic::Blocks, || block(8, "cargo test"));
    bus.publish(Topic::Blocks, || block(9, "secret"));
    let line = |subscription: &Subscription| match subscription.next(NONE) {
        Next::Event(published) => subscription.line(&published).to_string(),
        other => panic!("{other:?}"),
    };
    assert!(!line(&nobody).contains("cargo test"), "no grant, no command");
    assert!(line(&itself).contains("cargo test"), "a terminal reads its own");
    assert!(line(&everything).contains("cargo test"));
    assert!(!line(&nobody).contains("secret"));
    assert!(!line(&itself).contains("secret"), "but not another's");
    assert!(line(&everything).contains("secret"));
    // The event is still there, with its ids and exit code.
    let seen = bus.subscribe(filter(&["blocks"]), Grant::default(), None).expect("room").0;
    bus.publish(Topic::Blocks, || block(8, "x"));
    let Next::Event(published) = seen.next(NONE) else { panic!("an event") };
    assert!(seen.line(&published).contains(r#""exit_code":0"#));
}

#[test]
fn turning_reads_off_closes_the_streams_that_needed_them_and_keeps_a_terminals_own() {
    let bus = Bus::new();
    let output = EventsSpec { output: true, ..EventsSpec::default() }.filter().expect("valid");
    let (needs_it, _) = bus
        .subscribe(output.clone(), Grant { reads_all: true, own: vec![], ..Grant::default() }, None)
        .expect("room");
    let own_only =
        EventsSpec { output: true, terminals: vec![8], ..EventsSpec::default() }.filter().expect("valid");
    let (of_itself, _) = bus
        .subscribe(own_only, Grant { reads_all: true, own: vec![8], ..Grant::default() }, Some(8))
        .expect("room");
    let (no_output, _) = bus
        .subscribe(filter(&["blocks"]), Grant { reads_all: true, own: vec![], ..Grant::default() }, None)
        .expect("room");
    bus.revoke_reads();
    assert!(matches!(needs_it.next(NONE), Next::Closed(CloseReason::NotAllowed)));
    assert!(matches!(of_itself.next(NONE), Next::Idle), "it reads only itself");
    assert!(matches!(no_output.next(NONE), Next::Idle), "it never needed the grant");
    // And the commands of blocks stop going to a stream that lost the grant.
    bus.publish(Topic::Blocks, || block(9, "secret"));
    let Next::Event(published) = no_output.next(NONE) else { panic!("an event") };
    assert!(!no_output.line(&published).contains("secret"));
}

#[test]
fn closing_all_ends_every_stream() {
    let bus = Bus::new();
    let one = subscribe(&bus, &[]);
    let two = subscribe(&bus, &["bell"]);
    bus.close_all(CloseReason::Quitting);
    assert!(matches!(one.next(NONE), Next::Closed(CloseReason::Quitting)));
    assert!(matches!(two.next(NONE), Next::Closed(CloseReason::Quitting)));
}

#[test]
fn taking_an_approval_back_closes_the_streams_started_under_it_and_only_those() {
    let bus = Bus::new();
    let under = |holder| Grant { holder, ..Grant::default() };
    let (revoked, _) = bus.subscribe(filter(&["bell"]), under(Some(4)), None).expect("room");
    let (other, _) = bus.subscribe(filter(&["bell"]), under(Some(5)), None).expect("room");
    let (needed_none, _) = bus.subscribe(filter(&["bell"]), under(None), None).expect("room");
    bus.revoke_holder(4);
    assert!(matches!(revoked.next(NONE), Next::Closed(CloseReason::NotAllowed)));
    bus.publish(Topic::Bell, || bell(1));
    assert!(matches!(other.next(NONE), Next::Event(_)), "another approval's stream goes on");
    assert!(matches!(needed_none.next(NONE), Next::Event(_)), "so does one that needed none");
}

#[test]
fn the_end_of_open_access_keeps_the_streams_of_programs_that_hold_their_own_read_grant() {
    let bus = Bus::new();
    let reads = |holder| Grant { reads_all: true, holder, ..Grant::default() };
    let (open, _) = bus.subscribe(filter(&["output"]), reads(None), None).expect("room");
    let (held, _) = bus.subscribe(filter(&["output"]), reads(Some(4)), None).expect("room");
    bus.revoke_reads();
    assert!(matches!(open.next(NONE), Next::Closed(CloseReason::NotAllowed)), "what Open let in goes");
    bus.publish(Topic::Bell, || bell(1));
    assert!(!matches!(held.next(NONE), Next::Closed(_)), "an approval of its own stays until it is revoked");
}

#[test]
fn nothing_about_a_hidden_terminal_is_told_except_to_a_stream_inside_it() {
    let bus = Bus::new();
    let outside = subscribe(&bus, &["bell"]);
    let inside =
        bus.subscribe(filter(&["bell"]), Grant { own: vec![2], ..Grant::default() }, None).expect("room").0;
    bus.set_hidden(2, true);
    bus.publish(Topic::Bell, || bell(1));
    bus.publish(Topic::Bell, || bell(2));
    // The stream outside hears terminal 1 and never terminal 2; the one inside hears its own.
    assert_eq!(seq_of(outside.next(NONE)), 1);
    assert!(matches!(outside.next(NONE), Next::Idle), "terminal 2 was hidden");
    assert_eq!(seq_of(inside.next(NONE)), 1);
    assert_eq!(seq_of(inside.next(NONE)), 2);
    // Shown again, it is told to everyone.
    bus.set_hidden(2, false);
    bus.publish(Topic::Bell, || bell(2));
    assert_eq!(seq_of(outside.next(NONE)), 3);
}

#[test]
fn a_hidden_terminal_is_not_in_a_splits_list_and_a_split_of_nothing_else_is_not_told() {
    let bus = Bus::new();
    let splits = subscribe(&bus, &["splits"]);
    bus.set_hidden(3, true);
    let split = |terminals: Vec<i64>, focused| Event {
        window: Some(1),
        terminal: None,
        body: Body::Split { split: 9, terminals, focused },
    };
    bus.publish(Topic::Splits, || split(vec![2, 3], Some(3)));
    bus.publish(Topic::Splits, || split(vec![3], Some(3)));
    let Next::Event(told) = splits.next(NONE) else { panic!("the split was told") };
    let line = told.line(false);
    assert!(line.contains("\"terminals\":[2]") && !line.contains("\"focused\""), "{line}");
    assert!(matches!(splits.next(NONE), Next::Idle), "a split of hidden terminals only is not told");
}

#[test]
fn hiding_a_terminal_ends_a_wait_on_it_by_someone_outside() {
    let bus = Bus::new();
    let (waiter, _) = bus.wait(super::Watch::of_run(5), Grant::default(), None).expect("room");
    bus.set_hidden(5, true);
    assert!(matches!(waiter.next(NONE), Next::Closed(CloseReason::NotAllowed)));
}
