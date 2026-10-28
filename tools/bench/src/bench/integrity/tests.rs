//! The whole check against stand-ins for terminals: one that keeps its word, ones that don't, and one
//! that shows what the check cannot see.

use std::ops::Range;
use std::time::Duration;

use super::fake::{Fake, Mode, text};
use super::*;
use crate::bench::runner::time_payload;

/// The run the helpers plan for.
const SLOT: Slot<'static> = Slot { test: "logflood", run: 0, attempt: 0 };

/// The first look at a run's answers, as the runner takes it.
fn first_look(v: &mut Verifier, input: &mut Input, asked: u64) -> Check {
    let wait = v.settle();
    v.finish(input, asked, wait)
}

/// One measured run as `bench` makes it: probe, plan, time, count.
fn run(mode: Mode, lost: Range<usize>, payload: &Payload) -> (Option<Why>, Option<Check>, Fake) {
    run_with(mode, mode, lost, payload)
}

/// The same, for a terminal that behaves one way when probed and another when measured.
fn run_with(
    probed: Mode,
    measured: Mode,
    lost: Range<usize>,
    payload: &Payload,
) -> (Option<Why>, Option<Check>, Fake) {
    let (mut input, mut term) = Fake::new(probed, 0..0);
    let mut v = Verifier::for_test(64 * 1024);
    // The probe goes first; the loss, if any, starts after it (offsets count from the payload).
    let why = v.unchecked(&mut input, &mut term).expect("probe");
    (term.offset, term.sent, term.kept, term.calls) = (0, 0, 0, 0);
    term.parsed.clear();
    (term.mode, term.lost) = (measured, lost);
    let plan = v.plan(payload, why.is_none(), SLOT);
    v.begin(&mut input);
    let timed = time_payload(&mut input, &mut term, payload, &plan, Duration::from_secs(10)).expect("run");
    let timed = timed.expect("the cursor report was answered");
    let asked: u64 = timed.writes.iter().map(|w| u64::from(w.checkpoints)).sum();
    assert_eq!(asked, plan.len() as u64, "every planned checkpoint was written");
    let check = why.is_none().then(|| first_look(&mut v, &mut input, asked));
    (why, check, term)
}

#[test]
fn a_terminal_that_answers_everything_is_verified_and_gets_the_payload_unchanged() {
    let p = text(2_000_000);
    let (why, check, term) = run(Mode::Faithful, 0..0, &p);
    assert_eq!(why, None);
    let Some(Check::Verified { checkpoints }) = check else { panic!("{check:?}") };
    assert!(checkpoints >= 16, "{checkpoints}");
    // What the parser got is the payload, with nothing added: the queries are taken out of its input.
    assert!(term.parsed == p.bytes);
    // The check costs no call to the kernel: one per write, and the cursor report's.
    assert_eq!(term.calls, p.write_count() + 1);
    // Four bytes more per query, and the cursor report's question.
    assert_eq!(term.sent, p.bytes.len() + (checkpoints as usize + 1) * 4);
}

#[test]
fn a_terminal_that_loses_a_stretch_is_caught_and_the_count_says_how_much() {
    let p = text(2_000_000);
    // The middle quarter never reaches the parser; the end, and the final cursor report, do.
    let (_, check, _) = run(Mode::Faithful, 750_000..1_250_000, &p);
    let Some(Check::Dropped { answered, checkpoints }) = check else { panic!("{check:?}") };
    assert!(answered > 0 && answered < checkpoints, "{answered} of {checkpoints}");
    // Roughly a quarter of the checkpoints were in the lost stretch.
    assert!((checkpoints - answered) as f64 >= checkpoints as f64 * 0.1, "{answered} of {checkpoints}");
}

#[test]
fn a_terminal_that_keeps_only_the_tail_of_a_ring_is_caught() {
    // What a refterm-style terminal does: only the last 64 KiB of what it read is parsed.
    let p = text(4_000_000);
    let (_, check, _) = run(Mode::Faithful, 0..(p.bytes.len() - 64 * 1024), &p);
    let Some(Check::Dropped { answered, checkpoints }) = check else { panic!("{check:?}") };
    assert!(answered <= 3 && checkpoints >= 16, "{answered} of {checkpoints}");
}

/// What the reviewer found: the first design put each query at the end of a write, so a terminal that
/// parses only the tail of what it reads, in the writer's units, answered every one of them.
#[test]
fn a_terminal_that_keeps_only_the_tail_of_each_write_is_caught() {
    let p = text(6_000_000);
    let (why, check, term) = run(Mode::TailOfEachCall { keep: 4096 }, 0..0, &p);
    assert_eq!(why, None);
    // It parsed about 6% of the run, and the check says so.
    assert!((term.kept as f64) < 0.08 * term.sent as f64, "{} of {}", term.kept, term.sent);
    let Some(Check::Dropped { answered, checkpoints }) = check else { panic!("{check:?}") };
    assert!(answered * 4 < checkpoints, "{answered} of {checkpoints}");
}

/// The same for a ring whose ticks are not the writer's writes.
#[test]
fn a_terminal_that_keeps_the_tail_of_every_tick_is_caught() {
    let p = text(4_000_000);
    for tick in [65_536, 100_000, 1 << 20] {
        let mode = Mode::TailOfEachTick { tick, keep: 4096 };
        let (_, check, term) = run_with(Mode::Faithful, mode, 0..0, &p);
        assert!((term.kept as f64) < 0.1 * term.sent as f64, "{tick}");
        assert!(matches!(check, Some(Check::Dropped { .. })), "tick {tick}: {check:?}");
    }
}

#[test]
fn a_workload_with_room_for_few_checkpoints_is_unverified_not_verified() {
    // Emoji everywhere and a pair of ASCII bytes now and then: a handful of places that may be cut.
    let mut bytes = Vec::new();
    while bytes.len() < 2_000_000 {
        bytes.extend("😀".repeat(100_000).as_bytes());
        bytes.extend(b"ab");
    }
    let p = Payload { bytes, ..Payload::default() };
    let (_, check, _) = run(Mode::Faithful, 0..0, &p);
    assert!(matches!(check, Some(Check::Unchecked(Why::TooFew { .. }))), "{check:?}");
    // A terminal that lost one of the few is still caught.
    let (_, check, _) = run(Mode::Faithful, 0..1_000_000, &p);
    assert!(matches!(check, Some(Check::Dropped { .. })), "{check:?}");
}

#[test]
fn the_terminal_is_judged_on_the_queries_written_not_the_ones_planned() {
    let p = text(2_000_000);
    let (mut input, mut term) = Fake::new(Mode::Faithful, 0..0);
    let mut v = Verifier::for_test(64 * 1024);
    assert_eq!(v.unchecked(&mut input, &mut term).expect("probe"), None);
    let mut plan = v.plan(&p, true, SLOT);
    // A checkpoint no write can carry (past the end of the payload).
    plan.cuts.push(p.bytes.len() + 10);
    v.begin(&mut input);
    let timed = time_payload(&mut input, &mut term, &p, &plan, Duration::from_secs(10)).expect("run");
    let asked: u64 = timed.expect("answered").writes.iter().map(|w| u64::from(w.checkpoints)).sum();
    assert_eq!(asked, plan.len() as u64 - 1);
    assert!(matches!(first_look(&mut v, &mut input, asked), Check::Verified { .. }));
}

#[test]
fn a_terminal_that_never_answers_dsr_5_is_unchecked_not_dropped() {
    let p = text(500_000);
    let (why, check, term) = run(Mode::NoDsr5, 0..0, &p);
    assert_eq!(why, Some(Why::Silent));
    assert_eq!(check, None);
    // Nothing was added to the stream: the payload went out as it always did.
    assert!(term.parsed == p.bytes);
    // Only the probe's burst was ever sent.
    assert_eq!(term.dsr5_seen, probe::BURST);
}

#[test]
fn a_terminal_that_answers_only_some_when_idle_cannot_vouch() {
    let p = text(500_000);
    let (why, check, _) = run(Mode::Rationed(5), 0..0, &p);
    assert_eq!(why, Some(Why::Lossy { answered: 5 }));
    assert_eq!(check, None);
}

/// What the reviewer found: answers that come after the cursor report were counted as if they had come
/// before it, so a terminal that answers from another thread, behind the one that reports the cursor,
/// was verified on a time that ended too soon.
#[test]
fn answers_that_come_after_the_cursor_report_are_all_there_but_not_in_time() {
    let p = text(1_000_000);
    let (_, check, _) = run(Mode::Late, 0..0, &p);
    let Some(Check::Unchecked(Why::OutOfOrder { before_report, checkpoints })) = check else {
        panic!("{check:?}")
    };
    assert_eq!(before_report, 0);
    assert!(checkpoints >= ENOUGH, "{checkpoints}");
}

#[test]
fn the_number_of_answers_in_time_is_the_number_before_the_cursor_report() {
    let p = text(2_000_000);
    // The probe's 16, then the first 10 of the run answered in time and the rest held back.
    let (_, check, _) = run_with(Mode::Faithful, Mode::HoldAfter(16 + 10), 0..0, &p);
    let Some(Check::Unchecked(Why::OutOfOrder { before_report, checkpoints })) = check else {
        panic!("{check:?}")
    };
    assert_eq!(before_report, 10);
    assert!(checkpoints > 10, "{checkpoints}");
}

#[test]
fn more_answers_than_questions_prove_nothing() {
    let p = text(1_000_000);
    // Answering twice already when idle: not a witness.
    let (why, check, _) = run(Mode::Twice, 0..0, &p);
    assert_eq!(why, Some(Why::Surplus { answered: 32, checkpoints: 16 }));
    assert_eq!(check, None);
    // Answering twice only under load: the count no longer means anything.
    let (why, check, _) = run_with(Mode::Faithful, Mode::Twice, 0..0, &p);
    assert_eq!(why, None);
    assert!(matches!(check, Some(Check::Unchecked(Why::Surplus { .. }))), "{check:?}");
}

/// The limit, written down as a test: a terminal that recognises the query and answers it without
/// parsing anything else cannot be told from one that parses everything. The check is a tripwire for
/// lost input, not a proof against a program written to pass it (README: "What it cannot prove").
#[test]
fn a_terminal_written_to_pass_is_not_caught() {
    let p = text(2_000_000);
    let (_, check, term) = run(Mode::PicksQueriesOut, 0..0, &p);
    assert!(matches!(check, Some(Check::Verified { .. })), "{check:?}");
    assert!(term.parsed.is_empty());
}

#[test]
fn the_check_can_be_turned_off() {
    let (mut input, mut term) = Fake::new(Mode::Faithful, 0..0);
    let mut v = Verifier::new(None, None);
    assert_eq!(v.unchecked(&mut input, &mut term).expect("probe"), Some(Why::Off));
    assert_eq!(v.plan(&text(1_000_000), false, SLOT), Plan::none());
    assert_eq!(term.dsr5_seen, 0);
}

#[test]
fn answers_that_are_late_but_all_come_are_not_called_dropped() {
    let p = text(1_000_000);
    let (mut input, mut term) = Fake::new(Mode::Faithful, 0..0);
    let mut v = Verifier::for_test(64 * 1024);
    assert_eq!(v.unchecked(&mut input, &mut term).expect("probe"), None);
    term.mode = Mode::Delayed { ms: 200 };
    let plan = v.plan(&p, true, SLOT);
    v.begin(&mut input);
    let timed = time_payload(&mut input, &mut term, &p, &plan, Duration::from_secs(10)).expect("run");
    let asked: u64 = timed.expect("answered").writes.iter().map(|w| u64::from(w.checkpoints)).sum();
    // The first look (60 ms in the test) sees answers missing ...
    let first = first_look(&mut v, &mut input, asked);
    assert!(first.is_dropped(), "{first:?}");
    // ... the stragglers are waited for, so that none is counted in the repeat ...
    v.await_stragglers(&mut input, asked);
    assert_eq!(input.take_status_replies(), 0);
    // ... and a patient look at a run whose answers all come, late, is neither dropped nor verified.
    let patience = v.patience();
    v.begin(&mut input);
    // The repeat is the run's second try, as the runner plans it.
    let plan = v.plan(&p, true, Slot { attempt: 1, ..SLOT });
    let timed = time_payload(&mut input, &mut term, &p, &plan, Duration::from_secs(10)).expect("run");
    let asked: u64 = timed.expect("answered").writes.iter().map(|w| u64::from(w.checkpoints)).sum();
    let again = v.finish(&mut input, asked, patience);
    assert!(matches!(again, Check::Unchecked(Why::OutOfOrder { before_report: 0, .. })), "{again:?}");
}

#[test]
fn answers_that_never_come_stay_missing_after_the_longer_wait() {
    let p = text(1_000_000);
    let (mut input, mut term) = Fake::new(Mode::TailOfEachCall { keep: 4096 }, 0..0);
    let mut v = Verifier::for_test(64 * 1024);
    // The probe's burst fits in the tail it keeps.
    assert_eq!(v.unchecked(&mut input, &mut term).expect("probe"), None);
    let plan = v.plan(&p, true, SLOT);
    v.begin(&mut input);
    let timed = time_payload(&mut input, &mut term, &p, &plan, Duration::from_secs(10)).expect("run");
    let asked: u64 = timed.expect("answered").writes.iter().map(|w| u64::from(w.checkpoints)).sum();
    assert!(first_look(&mut v, &mut input, asked).is_dropped());
    let patience = v.patience();
    assert!(v.finish(&mut input, asked, patience).is_dropped());
}

#[test]
fn a_repeat_that_verifies_is_unverified_and_one_that_drops_again_is_dropped() {
    let first = Check::Dropped { answered: 20, checkpoints: 30 };
    let ok = Check::Verified { checkpoints: 30 };
    assert_eq!(after_retry(&first, ok), Check::Unchecked(Why::Retried { answered: 20, checkpoints: 30 }));
    let dropped = Check::Dropped { answered: 3, checkpoints: 30 };
    assert_eq!(after_retry(&first, dropped.clone()), dropped);
    let few = Check::Unchecked(Why::TooFew { checkpoints: 9 });
    assert_eq!(after_retry(&first, few.clone()), few);
}

#[test]
fn a_seed_places_the_checkpoints_of_each_run_again_whatever_else_ran() {
    let p = text(2_000_000);
    let slot = |test, run, attempt| Slot { test, run, attempt };
    let v = Verifier::new(Some(64 * 1024), Some(1234));
    assert_eq!(v.seed(), Some(1234));
    let (first, second) =
        (v.plan(&p, true, slot("logflood", 0, 0)), v.plan(&p, true, slot("logflood", 1, 0)));
    // The same seed puts a test's run where it was, in any order and next to any other runs ...
    let other = Verifier::new(Some(64 * 1024), Some(1234));
    other.plan(&p, true, slot("colorflood", 0, 0));
    other.plan(&p, true, slot("logflood", 0, 1));
    assert_eq!(other.plan(&p, true, slot("logflood", 1, 0)), second);
    assert_eq!(other.plan(&p, true, slot("logflood", 0, 0)), first);
    // ... another run, test, try or seed puts it elsewhere.
    assert_ne!(first.cuts, second.cuts);
    assert_ne!(first.cuts, v.plan(&p, true, slot("colorflood", 0, 0)).cuts);
    assert_ne!(first.cuts, v.plan(&p, true, slot("logflood", 0, 1)).cuts);
    assert_ne!(first.cuts, Verifier::new(Some(64 * 1024), Some(1235)).plan(&p, true, SLOT).cuts);
    // Every seed that is reported is one a JSON number holds and the generator takes as it is.
    for plan in [first, second] {
        assert!(plan.seed.is_some_and(|s| (1..=MAX_SEED).contains(&s)), "{:?}", plan.seed);
    }
    for given in [0, 1, MAX_SEED, u64::MAX] {
        let v = Verifier::new(Some(64 * 1024), Some(given));
        assert!(v.seed().is_some_and(|s| (1..=MAX_SEED).contains(&s)));
    }
    assert_eq!(Verifier::new(None, Some(5)).seed(), None);
    assert_eq!(Verifier::new(Some(1), None).plan(&Payload::default(), false, SLOT), Plan::none());
}

/// What the reviewer found: the clock's nanoseconds were clamped to the range, so every invocation
/// got the largest seed and the same places.
#[test]
fn two_invocations_without_a_seed_draw_different_places() {
    let p = text(2_000_000);
    let a = Verifier::new(Some(64 * 1024), None);
    std::thread::sleep(Duration::from_millis(2));
    let b = Verifier::new(Some(64 * 1024), None);
    assert_ne!(a.seed(), b.seed());
    assert!(a.seed().is_some_and(|s| s != MAX_SEED));
    assert_ne!(a.plan(&p, true, SLOT).cuts, b.plan(&p, true, SLOT).cuts);
}
