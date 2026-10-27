use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use wormhole_term::ScreenReader;
use wormhole_term::testing::TestTerm;

use super::WaitJob;
use crate::ipc::bus::{Bus, Grant, Watch};
use crate::ipc::control::event::{BlockPhase, Body, CloseReason, Event, Topic};
use crate::ipc::control::{ErrorCode, Met, Ran, Reason, Reply, Target, TerminalState, WaitSpec, Waited};

const TERMINAL: i64 = 3;

fn spec() -> WaitSpec {
    WaitSpec {
        target: Target::Terminal(TERMINAL),
        status: Vec::new(),
        pattern: None,
        gone: None,
        stable_ms: None,
        block_done: false,
        after: None,
        timeout_ms: Some(5000),
    }
}

fn reading() -> Grant {
    Grant { reads_all: true, own: Vec::new() }
}

/// A wait on terminal 3 for `spec`, running on a thread, with the grant given.
struct Rig {
    bus: Arc<Bus>,
    reader: ScreenReader,
    done: JoinHandle<Reply>,
}

fn start(spec: WaitSpec, grant: Grant, term: TestTerm) -> Rig {
    let bus = Bus::new();
    let reader = term.into_reader();
    let plan = spec.plan().expect("a plan");
    let (subscription, _) = bus.wait(Watch::of_wait(TERMINAL, &plan), grant, None).expect("room");
    let job = WaitJob::wait(subscription, reader.downgrade(), TERMINAL, plan);
    Rig { bus, reader, done: std::thread::spawn(move || job.finish(|| false)) }
}

impl Rig {
    fn event(&self, topic: Topic, body: Body) {
        self.bus.publish(topic, || Event { window: Some(1), terminal: Some(TERMINAL), body });
    }

    /// The terminal printed `bytes`: the screen changes and the app's wake-up reaches the waiter.
    fn print(&self, bytes: &[u8]) {
        self.reader.print(bytes);
        self.bus.pulse(1, TERMINAL, || None);
    }

    fn reply(self) -> Reply {
        self.done.join().expect("the wait's thread")
    }

    fn finished(&self) -> bool {
        self.done.is_finished()
    }
}

fn waited(reply: &Reply) -> Waited {
    serde_json::from_value(reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"))).expect("a wait")
}

fn failure(reply: &Reply) -> (ErrorCode, Option<Reason>) {
    let error = reply.err().unwrap_or_else(|| panic!("an error: {reply:?}"));
    (error.code, error.reason)
}

fn settle() {
    std::thread::sleep(Duration::from_millis(150));
}

fn status(now: TerminalState) -> Body {
    Body::Status { status: now, was: TerminalState::Working, agent: None, message: None }
}

#[test]
fn a_state_asked_for_ends_the_wait_and_another_does_not() {
    let rig = start(
        WaitSpec { status: vec![TerminalState::Idle, TerminalState::NeedsYou], ..spec() },
        Grant::default(),
        TestTerm::new(20, 3, 10),
    );
    rig.event(Topic::Status, status(TerminalState::Failed));
    settle();
    assert!(!rig.finished(), "failed is not among them");
    rig.event(Topic::Status, status(TerminalState::NeedsYou));
    let found = waited(&rig.reply());
    assert_eq!(
        (found.met, found.status, found.terminal),
        (Met::Status, Some(TerminalState::NeedsYou), TERMINAL)
    );
}

#[test]
fn a_command_finishing_ends_a_wait_for_one_with_its_block_and_code() {
    let rig = start(WaitSpec { block_done: true, ..spec() }, Grant::default(), TestTerm::new(20, 3, 10));
    let block =
        |phase, exit_code| Body::Block { block: 58, phase, exit_code, took_ms: Some(40), command: None };
    rig.event(Topic::Blocks, block(BlockPhase::Started, None));
    settle();
    assert!(!rig.finished(), "starting is not finishing");
    rig.event(Topic::Blocks, block(BlockPhase::Finished, Some(101)));
    let found = waited(&rig.reply());
    assert_eq!((found.met, found.block, found.exit_code), (Met::BlockDone, Some(58), Some(101)));
}

#[test]
fn text_that_appears_on_the_screen_ends_the_wait_with_what_it_found() {
    let mut term = TestTerm::new(30, 4, 10);
    term.feed(b"building...");
    let rig = start(WaitSpec { pattern: Some(r"tests? ok\b".into()), ..spec() }, reading(), term);
    settle();
    assert!(!rig.finished());
    rig.print(b"\r\ntest ok");
    let found = waited(&rig.reply());
    assert_eq!((found.met, found.matched.as_deref()), (Met::Text, Some("test ok")));
    assert!(found.revision.is_some());
}

#[test]
fn text_already_there_meets_a_wait_at_once() {
    let mut term = TestTerm::new(30, 4, 10);
    term.feed(b"$ ");
    let started = Instant::now();
    // A line's trailing blanks are not text: the prompt "$ " is the line "$".
    let rig = start(WaitSpec { pattern: Some(r"^\$$".into()), ..spec() }, reading(), term);
    let found = waited(&rig.reply());
    assert_eq!(found.met, Met::Text);
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
}

#[test]
fn text_that_goes_meets_gone_and_text_that_is_not_there_meets_it_at_the_start() {
    let mut term = TestTerm::new(30, 4, 10);
    term.feed(b"loading...");
    let rig = start(WaitSpec { gone: Some("loading".into()), ..spec() }, reading(), term);
    settle();
    assert!(!rig.finished());
    rig.print(b"\x1b[2J\x1b[Hdone");
    assert_eq!(waited(&rig.reply()).met, Met::Gone);
    let rig = start(WaitSpec { gone: Some("loading".into()), ..spec() }, reading(), TestTerm::new(30, 4, 10));
    assert_eq!(waited(&rig.reply()).met, Met::Gone, "not there at all");
}

#[test]
fn a_full_screen_programs_screen_is_the_one_read() {
    let mut term = TestTerm::new(30, 4, 10);
    term.feed(b"shell prompt\x1b[?1049h\x1b[2;3Hmenu: quit");
    let rig = start(WaitSpec { pattern: Some("menu: quit".into()), ..spec() }, reading(), term);
    assert_eq!(waited(&rig.reply()).matched.as_deref(), Some("menu: quit"));
}

#[test]
fn the_text_conditions_need_the_right_to_read_when_they_read() {
    let mut term = TestTerm::new(30, 4, 10);
    term.feed(b"secret");
    let rig = start(WaitSpec { pattern: Some("secret".into()), ..spec() }, Grant::default(), term);
    assert_eq!(failure(&rig.reply()), (ErrorCode::NotAllowed, None));
    // Quiet reads nothing, so it needs no right.
    let rig = start(WaitSpec { stable_ms: Some(50), ..spec() }, Grant::default(), TestTerm::new(30, 4, 10));
    assert_eq!(waited(&rig.reply()).met, Met::Stable);
}

#[test]
fn the_setting_going_off_ends_a_wait_on_text() {
    let rig =
        start(WaitSpec { pattern: Some("never".into()), ..spec() }, reading(), TestTerm::new(30, 4, 10));
    settle();
    rig.bus.revoke_reads();
    assert_eq!(failure(&rig.reply()), (ErrorCode::NotAllowed, None));
}

#[test]
fn quiet_is_no_output_for_the_time_asked_and_printing_keeps_pushing_it_back() {
    let rig = start(WaitSpec { stable_ms: Some(300), ..spec() }, Grant::default(), TestTerm::new(30, 4, 10));
    let busy_until = Instant::now() + Duration::from_millis(600);
    while Instant::now() < busy_until {
        rig.print(b".");
        std::thread::sleep(Duration::from_millis(40));
    }
    assert!(!rig.finished(), "it printed every 40 ms: never quiet for 300");
    let quiet_from = Instant::now();
    let found = waited(&rig.reply());
    assert_eq!(found.met, Met::Stable);
    assert!(quiet_from.elapsed() >= Duration::from_millis(250), "{:?}", quiet_from.elapsed());
}

#[test]
fn quiet_without_the_wake_up_still_ends_by_the_revision() {
    // A print the wake-up never told of (it is only a hint) moves the quiet's start when it is found.
    let rig = start(WaitSpec { stable_ms: Some(200), ..spec() }, Grant::default(), TestTerm::new(30, 4, 10));
    std::thread::sleep(Duration::from_millis(100));
    rig.reader.print(b"x");
    let after_print = Instant::now();
    let found = waited(&rig.reply());
    assert_eq!(found.met, Met::Stable);
    assert!(after_print.elapsed() >= Duration::from_millis(150), "{:?}", after_print.elapsed());
}

#[test]
fn a_wait_that_runs_out_of_time_says_timeout() {
    let rig = start(
        WaitSpec { status: vec![TerminalState::Idle], timeout_ms: Some(200), ..spec() },
        Grant::default(),
        TestTerm::new(30, 4, 10),
    );
    let started = Instant::now();
    let reply = rig.reply();
    assert_eq!(failure(&reply), (ErrorCode::Failed, Some(Reason::Timeout)));
    assert!(started.elapsed() >= Duration::from_millis(150) && started.elapsed() < Duration::from_secs(3));
}

#[test]
fn the_terminal_going_ends_the_wait_with_a_reason() {
    for body in [Body::TerminalClosed, status(TerminalState::Exited)] {
        let rig = start(WaitSpec { block_done: true, ..spec() }, Grant::default(), TestTerm::new(30, 4, 10));
        rig.event(Topic::Terminals, Body::Renamed { name: None });
        settle();
        assert!(!rig.finished(), "a rename is not the end");
        let topic = if matches!(body, Body::TerminalClosed) { Topic::Terminals } else { Topic::Status };
        rig.event(topic, body);
        assert_eq!(failure(&rig.reply()), (ErrorCode::NotFound, Some(Reason::Exited)));
    }
    // An exit is what was waited for, when it is.
    let rig = start(
        WaitSpec { status: vec![TerminalState::Exited], ..spec() },
        Grant::default(),
        TestTerm::new(30, 4, 10),
    );
    rig.event(Topic::Status, status(TerminalState::Exited));
    assert_eq!(waited(&rig.reply()).status, Some(TerminalState::Exited));
}

#[test]
fn the_screen_going_away_ends_a_wait_on_it() {
    let rig =
        start(WaitSpec { pattern: Some("never".into()), ..spec() }, reading(), TestTerm::new(30, 4, 10));
    settle();
    let Rig { bus, reader, done } = rig;
    drop(reader);
    bus.pulse(1, TERMINAL, || None);
    let reply = done.join().expect("thread");
    assert_eq!(failure(&reply), (ErrorCode::NotFound, Some(Reason::Exited)));
}

#[test]
fn the_app_quitting_ends_every_wait() {
    let rig = start(WaitSpec { block_done: true, ..spec() }, Grant::default(), TestTerm::new(30, 4, 10));
    settle();
    rig.bus.close_all(CloseReason::Quitting);
    let reply = rig.reply();
    assert_eq!(failure(&reply), (ErrorCode::Failed, None));
    assert!(reply.err().is_some_and(|e| e.message.contains("quitting")));
}

#[test]
fn a_waiter_gives_its_place_back_when_it_ends() {
    let rig = start(
        WaitSpec { status: vec![TerminalState::Idle], ..spec() },
        Grant::default(),
        TestTerm::new(30, 4, 10),
    );
    assert_eq!(rig.bus.subscribers(), 1);
    let bus = rig.bus.clone();
    rig.event(Topic::Status, status(TerminalState::Idle));
    let _ = rig.reply();
    assert_eq!(bus.subscribers(), 0);
    assert!(!bus.wants(Topic::Status) && !bus.wants_prints());
}

/// A `run` in a terminal whose shell has printed its first prompt, waiting after block `baseline`.
fn run_rig(grant: Grant, timeout: Duration) -> (Rig, u64) {
    let bus = Bus::new();
    let term = TestTerm::new(40, 12, 100);
    let reader = term.into_reader();
    let mut prompts = 0;
    reader.print_marked(b"\x1b]133;A\x07$ \x1b]133;B\x07", &mut prompts);
    let (subscription, _) = bus.wait(Watch::of_run(TERMINAL), grant, None).expect("room");
    let claim = bus.claim_run(TERMINAL).expect("the terminal is free");
    let first = reader.current_prompt().expect("a prompt");
    let job = WaitJob::run(subscription, reader.downgrade(), TERMINAL, first, timeout, claim);
    (Rig { bus, reader, done: std::thread::spawn(move || job.finish(|| false)) }, prompts)
}

fn ran(reply: &Reply) -> Ran {
    serde_json::from_value(reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"))).expect("a run")
}

fn block(phase: BlockPhase, id: i64, code: Option<i32>, took: Option<u64>) -> Body {
    Body::Block { block: id, phase, exit_code: code, took_ms: took, command: None }
}

#[test]
fn a_run_returns_the_exit_code_the_time_and_the_output_of_its_block() {
    let (rig, mut prompts) = run_rig(reading(), Duration::from_secs(5));
    rig.reader.print_marked(b"cargo test\r\n\x1b]133;C\x07test a ... ok\r\nFAILED\r\n", &mut prompts);
    rig.event(Topic::Blocks, block(BlockPhase::Started, 1, None, None));
    settle();
    assert!(!rig.finished());
    rig.reader.print_marked(b"\x1b]133;D;101\x07\x1b]133;A\x07$ \x1b]133;B\x07", &mut prompts);
    rig.event(Topic::Blocks, block(BlockPhase::Finished, 1, Some(101), Some(3512)));
    let done = ran(&rig.reply());
    assert_eq!((done.block, done.exit_code, done.duration_ms), (1, Some(101), 3512));
    assert_eq!(done.output.as_deref(), Some("test a ... ok\nFAILED"));
    assert!(!done.truncated && done.output_hidden.is_none());
}

#[test]
fn a_run_without_the_right_to_read_returns_the_code_and_says_why_not_the_output() {
    let (rig, mut prompts) = run_rig(Grant::default(), Duration::from_secs(5));
    rig.reader.print_marked(b"ls\r\n\x1b]133;C\x07secret\r\n\x1b]133;D;0\x07", &mut prompts);
    rig.event(Topic::Blocks, block(BlockPhase::Finished, 1, Some(0), Some(5)));
    let done = ran(&rig.reply());
    assert_eq!((done.exit_code, done.output.as_deref()), (Some(0), None));
    assert_eq!(done.output_hidden.as_deref(), Some("control_read is off"));
}

#[test]
fn a_command_that_printed_nothing_has_an_empty_output() {
    let (rig, mut prompts) = run_rig(reading(), Duration::from_secs(5));
    rig.reader
        .print_marked(b"true\r\n\x1b]133;C\x07\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07", &mut prompts);
    rig.event(Topic::Blocks, block(BlockPhase::Finished, 1, Some(0), None));
    let done = ran(&rig.reply());
    assert_eq!(done.output.as_deref(), Some(""));
    assert!(done.exit_code == Some(0));
}

#[test]
fn only_the_block_after_the_one_there_was_counts() {
    let bus = Bus::new();
    let reader = TestTerm::new(40, 6, 20).into_reader();
    let (subscription, _) = bus.wait(Watch::of_run(TERMINAL), Grant::default(), None).expect("room");
    // The shell was at prompt 8 when the command was typed: blocks before it are earlier commands.
    let claim = bus.claim_run(TERMINAL).expect("free");
    let job = WaitJob::run(subscription, reader.downgrade(), TERMINAL, 8, Duration::from_millis(400), claim);
    let done = std::thread::spawn(move || job.finish(|| false));
    let event =
        |body| bus.publish(Topic::Blocks, || Event { window: Some(1), terminal: Some(TERMINAL), body });
    event(block(BlockPhase::Finished, 7, Some(1), None));
    event(block(BlockPhase::Finished, 6, Some(1), None));
    let reply = done.join().expect("thread");
    assert_eq!(failure(&reply), (ErrorCode::Failed, Some(Reason::Timeout)), "those were earlier commands");
    assert!(reply.err().is_some_and(|e| e.message.contains("did not start")));
}

#[test]
fn a_run_that_times_out_names_the_block_still_running() {
    let (rig, _) = run_rig(reading(), Duration::from_millis(300));
    rig.event(Topic::Blocks, block(BlockPhase::Started, 4, None, None));
    let reply = rig.reply();
    assert_eq!(failure(&reply), (ErrorCode::Failed, Some(Reason::Timeout)));
    assert!(reply.err().is_some_and(|e| e.message.contains("block 4") && e.block == Some(4)), "{reply:?}");
}

#[test]
fn a_shell_that_ends_or_a_terminal_that_closes_ends_a_run() {
    let (rig, _) = run_rig(reading(), Duration::from_secs(5));
    rig.event(Topic::Status, status(TerminalState::Exited));
    assert_eq!(failure(&rig.reply()), (ErrorCode::NotFound, Some(Reason::Exited)));
    let (rig, _) = run_rig(reading(), Duration::from_secs(5));
    rig.event(Topic::Terminals, Body::TerminalClosed);
    assert_eq!(failure(&rig.reply()), (ErrorCode::NotFound, Some(Reason::Exited)));
}

#[test]
fn an_output_of_characters_that_grow_when_written_is_cut_to_fit_the_reply() {
    use crate::ipc::control::printable::json_line;
    use crate::ipc::control::read::REPLY_MAX_BYTES;

    // A byte of text that is six bytes written (DEL as an escape): the most a character can grow.
    let mut huge = Ran {
        terminal: 3,
        block: 1,
        exit_code: Some(0),
        duration_ms: 5,
        output: Some(format!("start{}end", "\u{7f}".repeat(250_000))),
        truncated: false,
        output_hidden: None,
    };
    assert!(json_line(&huge).len() > REPLY_MAX_BYTES);
    super::run::fit(&mut huge);
    assert!(json_line(&huge).len() <= REPLY_MAX_BYTES, "{} bytes", json_line(&huge).len());
    assert!(
        huge.truncated && huge.output.as_deref().is_some_and(|text| text.ends_with("end")),
        "the newest stays"
    );
}

#[test]
fn a_waiter_whose_client_went_away_ends_and_gives_its_place_back() {
    let bus = Bus::new();
    let reader = TestTerm::new(30, 4, 10).into_reader();
    let plan = WaitSpec { status: vec![TerminalState::Idle], ..spec() }.plan().expect("a plan");
    let (subscription, _) = bus.wait(Watch::of_wait(TERMINAL, &plan), Grant::default(), None).expect("room");
    let job = WaitJob::wait(subscription, reader.downgrade(), TERMINAL, plan);
    let started = Instant::now();
    let asked = std::cell::Cell::new(0);
    // Gone from the second time it is asked.
    let reply = job.finish(|| {
        asked.set(asked.get() + 1);
        asked.get() >= 2
    });
    assert_eq!(failure(&reply), (ErrorCode::Failed, None));
    assert!(started.elapsed() >= Duration::from_secs(2) && started.elapsed() < Duration::from_secs(5));
    assert_eq!(bus.subscribers(), 0);
    // And a run, the same way.
    let (subscription, _) = bus.wait(Watch::of_run(TERMINAL), Grant::default(), None).expect("room");
    let claim = bus.claim_run(TERMINAL).expect("free");
    let job = WaitJob::run(subscription, reader.downgrade(), TERMINAL, 1, Duration::from_secs(30), claim);
    let started = Instant::now();
    assert_eq!(failure(&job.finish(|| true)).0, ErrorCode::Failed);
    assert!(started.elapsed() < Duration::from_secs(4), "{:?}", started.elapsed());
    assert_eq!(bus.subscribers(), 0);
}

#[test]
fn a_run_holds_its_terminal_until_it_ends_and_a_second_waits_outside() {
    let (rig, _) = run_rig(reading(), Duration::from_millis(300));
    assert!(rig.bus.claim_run(TERMINAL).is_none(), "the run is under way");
    let bus = rig.bus.clone();
    let _ = rig.reply();
    assert!(bus.claim_run(TERMINAL).is_some(), "given back when it ended, by its timeout here");
    // And when the job is dropped before it ever ran.
    let reader = TestTerm::new(30, 4, 10).into_reader();
    let claim = bus.claim_run(TERMINAL).expect("free");
    let (subscription, _) = bus.wait(Watch::of_run(TERMINAL), Grant::default(), None).expect("room");
    let job = WaitJob::run(subscription, reader.downgrade(), TERMINAL, 1, Duration::from_secs(1), claim);
    assert!(bus.claim_run(TERMINAL).is_none());
    drop(job);
    assert!(bus.claim_run(TERMINAL).is_some());
}

#[test]
fn a_command_that_ended_before_the_wait_began_meets_it_at_once_by_after() {
    let bus = Bus::new();
    let reader = TestTerm::new(40, 8, 50).into_reader();
    let mut prompts = 0;
    reader.print_marked(
        b"\x1b]133;A\x07$ \x1b]133;B\x07a\r\n\x1b]133;C\x07\x1b]133;D;4\x07\x1b]133;A\x07$ \x1b]133;B\x07",
        &mut prompts,
    );
    let start = |after: Option<u64>| {
        let plan = WaitSpec { block_done: true, after, ..spec() }.plan().expect("a plan");
        let (subscription, _) =
            bus.wait(Watch::of_wait(TERMINAL, &plan), Grant::default(), None).expect("room");
        WaitJob::wait(subscription, reader.downgrade(), TERMINAL, plan)
    };
    // Block 1 finished with 4 before the wait: after 0 it counts, after 1 it does not.
    let found = waited(&start(Some(0)).finish(|| false));
    assert_eq!((found.met, found.block, found.exit_code), (Met::BlockDone, Some(1), Some(4)));
    let waiting = start(Some(1));
    let done = std::thread::spawn(move || waiting.finish(|| false));
    std::thread::sleep(Duration::from_millis(150));
    assert!(!done.is_finished(), "nothing after block 1 has finished");
    // An event for block 1 again does not count; one for block 2 does.
    let block = |id| Body::Block {
        block: id,
        phase: BlockPhase::Finished,
        exit_code: Some(0),
        took_ms: None,
        command: None,
    };
    bus.publish(Topic::Blocks, || Event { window: Some(1), terminal: Some(TERMINAL), body: block(1) });
    std::thread::sleep(Duration::from_millis(100));
    assert!(!done.is_finished());
    bus.publish(Topic::Blocks, || Event { window: Some(1), terminal: Some(TERMINAL), body: block(2) });
    assert_eq!(waited(&done.join().expect("thread")).block, Some(2));
}

#[test]
fn batched_marks_are_told_by_the_prompt_they_carry() {
    // A fast command and then an empty Enter, both ended before the window heard of the first: a run
    // typed at prompt 1 is the block 1 whatever else is on the grid by then.
    let (rig, mut prompts) = run_rig(reading(), Duration::from_secs(5));
    rig.reader.print_marked(
        b"true\r\n\x1b]133;C\x07\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07\r\n\x1b]133;D\x07\x1b]133;A\x07$ \x1b]133;B\x07",
        &mut prompts,
    );
    rig.event(Topic::Blocks, block(BlockPhase::Started, 1, None, None));
    rig.event(Topic::Blocks, block(BlockPhase::Finished, 1, Some(0), Some(2)));
    let done = ran(&rig.reply());
    assert_eq!((done.block, done.exit_code, done.output.as_deref()), (1, Some(0), Some("")));
}

#[test]
fn a_run_reads_its_output_after_the_history_was_cleared() {
    let (rig, mut prompts) = run_rig(reading(), Duration::from_secs(5));
    rig.reader.print_marked(b"make\r\n\x1b]133;C\x07built\r\n\x1b]133;D;0\x07\x1b[3J", &mut prompts);
    rig.event(Topic::Blocks, block(BlockPhase::Finished, 1, Some(0), Some(2)));
    let done = ran(&rig.reply());
    assert_eq!(done.exit_code, Some(0), "the code is the shell's, not the grid's");
    assert!(done.output.is_some() || done.output_hidden.is_some());
}
