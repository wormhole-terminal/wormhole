//! `wait` and `run` from the control line in a real window whose terminals run `/bin/cat`, as the other
//! tests of the control line do: `cat` prints back what it is given, marks and all, which is how a test
//! makes the engine see a shell's OSC 133 marks. The job a request returns is waited out on a thread of its
//! own, as the socket's stream thread would, while the window goes on here. The harness is `test_harness`.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::{
    Action, ErrorCode, Met, Ran, Reason, Reply, Request, RunSpec, Target, TerminalState, WaitSpec, Waited,
};
use wormhole_core::ipc::{Answer, Peer};
use wormhole_term::{PtyEvent, ReadScratch};

use crate::app::control;
use crate::app_state::OpenWindow;
use crate::settings::update_settings;
use crate::window::reported_tests::{event, terminal};
use crate::window::test_harness::{draw_fresh, drive, hook, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn ask(cx: &mut TestAppContext, action: Action, from: Option<&str>, peer: &Peer) -> Answer {
    let request = Request::new(action, from.map(str::to_string));
    cx.update(|cx| control::answer(request, peer, cx))
}

fn wait_for(target: i64, f: impl FnOnce(&mut WaitSpec)) -> Action {
    let mut spec = WaitSpec {
        target: Target::Terminal(target),
        status: Vec::new(),
        pattern: None,
        gone: None,
        stable_ms: None,
        block_done: false,
        after: None,
        timeout_ms: Some(10_000),
    };
    f(&mut spec);
    Action::Wait(spec)
}

fn run_of(target: i64, command: &str) -> Action {
    Action::Run(RunSpec {
        target: Target::Terminal(target),
        command: command.into(),
        timeout_ms: Some(10_000),
    })
}

/// The reply of an answer that is one at once.
fn at_once(answer: Answer) -> Reply {
    match answer {
        Answer::Reply(reply) => reply,
        other => panic!("expected an answer at once: {other:?}"),
    }
}

/// A job on a thread of its own, its reply delivered when it ends.
fn begin(answer: Answer) -> mpsc::Receiver<Reply> {
    let Answer::Wait(job) = answer else { panic!("expected a wait: {answer:?}") };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(job.finish(|| false));
    });
    rx
}

/// Run the window until `done`, with the real clock (a terminal's output comes from its reader thread).
fn settle(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    cx.run_until_parked();
    while !done(cx) {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(10));
        cx.run_until_parked();
    }
}

/// The Return that follows a pasted command comes a moment after it (the program reads the paste first):
/// let that moment pass.
fn typed(cx: &mut TestAppContext) {
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
}

/// The reply of a running job, keeping the window going until it comes.
fn ended(cx: &mut TestAppContext, job: &mpsc::Receiver<Reply>) -> Reply {
    let mut reply = None;
    settle(cx, "the wait ends", |_| {
        reply = job.try_recv().ok();
        reply.is_some()
    });
    reply.expect("a reply")
}

fn code(reply: &Reply) -> (Option<ErrorCode>, Option<Reason>) {
    (reply.err().map(|e| e.code), reply.err().and_then(|e| e.reason))
}

fn waited(reply: &Reply) -> Waited {
    serde_json::from_value(reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"))).expect("a wait")
}

fn ran(reply: &Reply) -> Ran {
    serde_json::from_value(reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"))).expect("a run")
}

fn allow(cx: &mut TestAppContext, read: bool, write: bool) {
    crate::agent_access::testing::allow(cx, read, write);
}

fn print(cx: &mut TestAppContext, window: &OpenWindow, ix: usize, bytes: &[u8]) {
    let t = terminal(cx, window, ix);
    cx.update(|cx| t.update(cx, |t, _| t.write_input(bytes.to_vec())));
}

/// What terminal `id` shows now, as a read of it sees it.
fn shown(cx: &mut TestAppContext, id: i64) -> String {
    let reader = cx
        .update(|cx| {
            let window = crate::app_state::app(cx).windows[0].clone();
            window.workspace.read_with(cx, |ws, cx| ws.control_reader(id, cx)).ok().flatten()
        })
        .expect("a reader");
    let spec = wormhole_term::CaptureSpec {
        range: wormhole_term::CaptureRange::Screen,
        join: true,
        styles: false,
        max_bytes: 100_000,
        max_rows: 100,
    };
    reader.reader.capture(&spec, &mut ReadScratch::default()).expect("readable").lines.join("\n")
}

fn app_bus(cx: &mut TestAppContext) -> std::sync::Arc<Bus> {
    cx.update(|cx| match crate::automation::bus(cx) {
        Some(bus) => bus,
        None => {
            let bus = Bus::new();
            crate::automation::install(bus.clone(), cx);
            bus
        }
    })
}

struct Callers {
    outside: Peer,
    inside_first: Peer,
}

fn callers(cx: &mut TestAppContext, window: &OpenWindow) -> Callers {
    let shell = terminal(cx, window, 0).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs");
    Callers { outside: Peer::new(900_002, vec![1]), inside_first: Peer::new(900_001, vec![shell]) }
}

/// Make terminal `ix` look like a shell with integration at its prompt (the marks `cat` prints back).
fn at_a_prompt(cx: &mut TestAppContext, window: &OpenWindow, ix: usize, id: i64) {
    print(cx, window, ix, b"\x1b]133;A\x07$ \x1b]133;B\x07\r");
    settle(cx, "the prompt mark", |cx| {
        cx.update(|cx| {
            let w = crate::app_state::app(cx).windows[0].clone();
            w.workspace
                .read_with(cx, |ws, cx| {
                    ws.control_reader(id, cx).is_some_and(|r| r.reader.at_prompt() == Some(true))
                })
                .unwrap_or(false)
        })
    });
}

#[gpui_kit::test]
fn a_wait_is_checked_before_anything_is_registered(cx: &mut TestAppContext) {
    if isolate("a_wait_is_checked_before_anything_is_registered") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, false, false);
    // What is wrong with the request, the terminal and the right to read.
    let bad = at_once(ask(cx, wait_for(1, |_| {}), None, &who.outside));
    assert_eq!(code(&bad).0, Some(ErrorCode::BadRequest));
    let broken = at_once(ask(cx, wait_for(1, |s| s.pattern = Some("foo(".into())), None, &who.outside));
    assert_eq!(code(&broken).0, Some(ErrorCode::BadRequest));
    let missing = at_once(ask(cx, wait_for(9, |s| s.block_done = true), None, &who.outside));
    assert_eq!(code(&missing).0, Some(ErrorCode::NotFound));
    let text = |s: &mut WaitSpec| s.pattern = Some("ok".into());
    let unread = at_once(ask(cx, wait_for(1, text), None, &who.outside));
    assert_eq!(code(&unread).0, Some(ErrorCode::NotAllowed), "reading is off, and a claim is no proof");
    let claimed = at_once(ask(cx, wait_for(1, text), Some("1"), &who.outside));
    assert_eq!(code(&claimed).0, Some(ErrorCode::NotAllowed));
    let without_marks = at_once(ask(cx, wait_for(1, |s| s.block_done = true), None, &who.outside));
    assert_eq!(code(&without_marks), (Some(ErrorCode::Unsupported), Some(Reason::NoIntegration)));
    assert_eq!(bus.subscribers(), 0, "nothing was registered by any of these");
    // The terminal's own caller may look at its text without the switch, and another's it may not.
    assert!(matches!(ask(cx, wait_for(1, text), None, &who.inside_first), Answer::Wait(_)));
    assert_eq!(
        code(&at_once(ask(cx, wait_for(2, text), None, &who.inside_first))).0,
        Some(ErrorCode::NotAllowed)
    );
    // Quiet and states need no right.
    assert!(matches!(ask(cx, wait_for(2, |s| s.stable_ms = Some(50)), None, &who.outside), Answer::Wait(_)));
    // The flag.
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert("automation".into(), false);
        })
    });
    let off = at_once(ask(cx, wait_for(1, |s| s.block_done = true), None, &who.outside));
    assert_eq!(code(&off), (Some(ErrorCode::Unsupported), Some(Reason::FlagOff)));
}

#[gpui_kit::test]
fn a_state_that_holds_is_answered_at_once_and_one_that_comes_is_waited_for(cx: &mut TestAppContext) {
    if isolate("a_state_that_holds_is_answered_at_once_and_one_that_comes_is_waited_for") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    let idle = |s: &mut WaitSpec| s.status = vec![TerminalState::Idle, TerminalState::Finished];
    let now = at_once(ask(cx, wait_for(2, idle), None, &who.outside));
    let held = waited(&now);
    assert_eq!((held.met, held.status, held.after_ms), (Met::Status, Some(TerminalState::Idle), 0));
    assert_eq!(bus.subscribers(), 0, "no thread, no place among the streams");
    // Waiting for a terminal to need the user: the agent's hook says so.
    let needs = |s: &mut WaitSpec| s.status = vec![TerminalState::NeedsYou];
    let job = begin(ask(cx, wait_for(2, needs), None, &who.outside));
    assert_eq!(bus.subscribers(), 1);
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Allow?"));
    cx.run_until_parked();
    assert!(job.try_recv().is_err(), "terminal 1 is another terminal");
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Allow Bash?"));
    let done = waited(&ended(cx, &job));
    assert_eq!((done.met, done.status, done.terminal), (Met::Status, Some(TerminalState::NeedsYou), 2));
    settle(cx, "the waiter's place is given back", |_| bus.subscribers() == 0);
}

#[gpui_kit::test]
fn text_on_the_screen_is_waited_for_and_found_as_it_is_printed(cx: &mut TestAppContext) {
    if isolate("text_on_the_screen_is_waited_for_and_found_as_it_is_printed") {
        return;
    }
    let window = start(cx, 1);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, true, false);
    assert!(!bus.wants_prints(), "nobody waits: nothing is asked of a print");
    let job = begin(ask(cx, wait_for(1, |s| s.pattern = Some(r"deploy (\w+)".into())), None, &who.outside));
    assert!(bus.wants_prints());
    print(cx, &window, 0, b"building\r");
    settle(cx, "it is on the screen", |cx| shown(cx, 1).contains("building"));
    assert!(job.try_recv().is_err(), "not what was waited for");
    print(cx, &window, 0, b"deploy done\r");
    let done = waited(&ended(cx, &job));
    assert_eq!((done.met, done.matched.as_deref()), (Met::Text, Some("deploy done")));
    settle(cx, "the waiter's place is given back", |_| bus.subscribers() == 0);
    assert!(!bus.wants_prints(), "and nobody is asked about prints again");
    // Gone: the text is cleared from the screen.
    let job = begin(ask(cx, wait_for(1, |s| s.gone = Some("deploy".into())), None, &who.outside));
    cx.run_until_parked();
    assert!(job.try_recv().is_err());
    print(cx, &window, 0, b"\x1b[2J\x1b[H\r");
    let done = waited(&ended(cx, &job));
    assert_eq!(done.met, Met::Gone);
}

#[gpui_kit::test]
fn a_terminal_gone_quiet_ends_a_wait_for_quiet_and_closing_it_ends_every_wait(cx: &mut TestAppContext) {
    if isolate("a_terminal_gone_quiet_ends_a_wait_for_quiet_and_closing_it_ends_every_wait") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    let started = Instant::now();
    let job = begin(ask(cx, wait_for(1, |s| s.stable_ms = Some(300)), None, &who.outside));
    let done = waited(&ended(cx, &job));
    assert_eq!(done.met, Met::Stable);
    assert!(started.elapsed() >= Duration::from_millis(250), "{:?}", started.elapsed());
    // A wait on a terminal that closes ends with the reason.
    let job = begin(ask(cx, wait_for(2, |s| s.status = vec![TerminalState::NeedsYou]), None, &who.outside));
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    let gone = ended(cx, &job);
    assert_eq!(code(&gone), (Some(ErrorCode::NotFound), Some(Reason::Exited)), "{gone:?}");
    settle(cx, "its place is given back", |_| bus.subscribers() == 0);
}

#[gpui_kit::test]
fn a_command_finishing_is_waited_for_by_its_marks(cx: &mut TestAppContext) {
    if isolate("a_command_finishing_is_waited_for_by_its_marks") {
        return;
    }
    let window = start(cx, 1);
    app_bus(cx);
    let who = callers(cx, &window);
    at_a_prompt(cx, &window, 0, 1);
    let job = begin(ask(cx, wait_for(1, |s| s.block_done = true), None, &who.outside));
    print(cx, &window, 0, b"make\r\x1b]133;C\x07\x1b]133;D;2\x07\r");
    let done = waited(&ended(cx, &job));
    assert_eq!((done.met, done.exit_code), (Met::BlockDone, Some(2)));
    assert!(done.block.is_some());
}

#[gpui_kit::test]
fn run_types_the_command_waits_for_its_block_and_tells_its_output(cx: &mut TestAppContext) {
    if isolate("run_types_the_command_waits_for_its_block_and_tells_its_output") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, true, true);
    at_a_prompt(cx, &window, 1, 2);
    let job = begin(ask(cx, run_of(2, "cargo test"), None, &who.outside));
    typed(cx);
    settle(cx, "the command was typed", |cx| shown(cx, 2).matches("cargo test").count() >= 2);
    assert!(job.try_recv().is_err(), "the command has not ended");
    // The program's side: output, then the shell's end mark with the code.
    print(
        cx,
        &window,
        1,
        b"\x1b]133;C\x07test a ... ok\rFAILED\r\x1b]133;D;101\x07\x1b]133;A\x07$ \x1b]133;B\x07\r",
    );
    let done = ran(&ended(cx, &job));
    assert_eq!(done.exit_code, Some(101));
    assert_eq!(done.output.as_deref(), Some("test a ... ok\nFAILED"));
    assert!(!done.truncated && done.output_hidden.is_none());
    settle(cx, "the waiter's place is given back", |_| bus.subscribers() == 0);
}

#[gpui_kit::test]
fn run_without_the_right_to_read_returns_the_code_and_says_the_output_is_hidden(cx: &mut TestAppContext) {
    if isolate("run_without_the_right_to_read_returns_the_code_and_says_the_output_is_hidden") {
        return;
    }
    let window = start(cx, 2);
    app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, false, true);
    at_a_prompt(cx, &window, 1, 2);
    let job = begin(ask(cx, run_of(2, "ls"), None, &who.outside));
    typed(cx);
    settle(cx, "the command was typed", |cx| shown(cx, 2).matches("ls").count() >= 2);
    print(cx, &window, 1, b"\x1b]133;C\x07secret\r\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07\r");
    let done = ran(&ended(cx, &job));
    assert_eq!((done.exit_code, done.output.as_deref()), (Some(0), None));
    assert_eq!(done.output_hidden.as_deref(), Some("control_read is off"));
}

#[gpui_kit::test]
fn run_is_refused_where_it_must_be_and_types_nothing_then(cx: &mut TestAppContext) {
    if isolate("run_is_refused_where_it_must_be_and_types_nothing_then") {
        return;
    }
    let window = start(cx, 3);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, true, false);
    // Typing is behind its switch.
    assert_eq!(code(&at_once(ask(cx, run_of(2, "ls"), None, &who.outside))).0, Some(ErrorCode::NotAllowed));
    allow(cx, true, true);
    // A command is one printable line.
    for bad in ["", "a\nb", "a\u{1b}b"] {
        assert_eq!(
            code(&at_once(ask(cx, run_of(2, bad), None, &who.outside))).0,
            Some(ErrorCode::BadRequest)
        );
    }
    assert_eq!(code(&at_once(ask(cx, run_of(9, "ls"), None, &who.outside))).0, Some(ErrorCode::NotFound));
    // A shell without integration: its end could not be told.
    let plain = at_once(ask(cx, run_of(2, "ls"), None, &who.outside));
    assert_eq!(code(&plain), (Some(ErrorCode::Unsupported), Some(Reason::NoIntegration)));
    at_a_prompt(cx, &window, 1, 2);
    at_a_prompt(cx, &window, 0, 1);
    // Never the caller's own terminal, by claim or by where it runs.
    let own = at_once(ask(cx, run_of(1, "ls"), Some("1"), &who.outside));
    assert_eq!(code(&own).0, Some(ErrorCode::Refused));
    let inside = at_once(ask(cx, run_of(1, "ls"), None, &who.inside_first));
    assert_eq!(code(&inside).0, Some(ErrorCode::Refused));
    // Not while the user's answer is awaited.
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Allow?"));
    let waiting = at_once(ask(cx, run_of(2, "ls"), None, &who.outside));
    assert_eq!(code(&waiting).0, Some(ErrorCode::Refused));
    // Not while a command runs in it.
    print(cx, &window, 2, b"\x1b]133;A\x07$ \x1b]133;B\x07\r");
    at_a_prompt(cx, &window, 2, 3);
    print(cx, &window, 2, b"sleep 9\r\x1b]133;C\x07\r");
    settle(cx, "the command is running", |cx| {
        cx.update(|cx| {
            let w = crate::app_state::app(cx).windows[0].clone();
            w.workspace
                .read_with(cx, |ws, cx| {
                    ws.control_reader(3, cx).is_some_and(|r| r.reader.at_prompt() == Some(false))
                })
                .unwrap_or(false)
        })
    });
    let busy = at_once(ask(cx, run_of(3, "ls"), None, &who.outside));
    assert_eq!(code(&busy), (Some(ErrorCode::Refused), Some(Reason::NotAtPrompt)));
    assert_eq!(bus.subscribers(), 0, "nothing was registered, and nothing typed");
    assert!(!shown(cx, 3).contains("\nls"), "{}", shown(cx, 3));
}

#[gpui_kit::test]
fn the_setting_going_off_ends_a_wait_on_text_and_a_waiter_costs_the_window_nothing(cx: &mut TestAppContext) {
    if isolate("the_setting_going_off_ends_a_wait_on_text_and_a_waiter_costs_the_window_nothing") {
        return;
    }
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::perf::allocations::thread_counts;

    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, true, false);
    let first = terminal(cx, &window, 0);
    let frame = |cx: &mut TestAppContext, window: &OpenWindow| {
        let before = thread_counts();
        draw_fresh(cx, window);
        thread_counts().since(before).allocations
    };
    for _ in 0..3 {
        draw_fresh(cx, &window);
    }
    let without = frame(cx, &window);
    // Output reaching a waiter does not wake the workspace.
    let notified = Rc::new(Cell::new(0usize));
    let counter = notified.clone();
    let workspace = window.workspace.upgrade().expect("the window is open");
    let _observer = cx.update(|cx| cx.observe(&workspace, move |_, _| counter.set(counter.get() + 1)));
    let round = |cx: &mut TestAppContext| {
        for _ in 0..20 {
            event(cx, &first, PtyEvent::Output);
        }
        cx.run_until_parked();
    };
    round(cx);
    notified.set(0);
    round(cx);
    let quiet = notified.get();
    let job = begin(ask(cx, wait_for(1, |s| s.pattern = Some("never printed".into())), None, &who.outside));
    let other = begin(ask(cx, wait_for(2, |s| s.stable_ms = Some(60_000)), None, &who.outside));
    round(cx);
    notified.set(0);
    round(cx);
    assert_eq!(
        notified.get(),
        quiet,
        "prints woke the workspace {} times more",
        notified.get().saturating_sub(quiet)
    );
    for _ in 0..2 {
        draw_fresh(cx, &window);
    }
    assert_eq!(frame(cx, &window), without, "a frame allocates what it did with nobody waiting");
    // Reading going off ends the wait that needs it, and not the one that does not.
    allow(cx, false, false);
    cx.update(|cx| crate::automation::sync_grants(cx, false));
    let ended_by_setting = ended(cx, &job);
    assert_eq!(code(&ended_by_setting).0, Some(ErrorCode::NotAllowed));
    assert!(other.try_recv().is_err(), "waiting for quiet needs no right");
    drop(other);
    settle(cx, "places given back", |_| bus.subscribers() <= 1);
}

#[gpui_kit::test]
fn a_second_run_in_the_same_terminal_is_refused_until_the_first_has_ended(cx: &mut TestAppContext) {
    if isolate("a_second_run_in_the_same_terminal_is_refused_until_the_first_has_ended") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    allow(cx, true, true);
    at_a_prompt(cx, &window, 1, 2);
    let first = begin(ask(cx, run_of(2, "echo one"), None, &who.outside));
    // The shell still looks to be at its prompt: the Return has not gone yet. The second must not clear the
    // first's line and type over it.
    let second = at_once(ask(cx, run_of(2, "echo two"), None, &who.outside));
    assert_eq!(code(&second), (Some(ErrorCode::Refused), Some(Reason::RunPending)), "{second:?}");
    typed(cx);
    settle(cx, "the first command was typed", |cx| shown(cx, 2).matches("echo one").count() >= 2);
    assert!(!shown(cx, 2).contains("echo two"), "{}", shown(cx, 2));
    // It stays refused while the command runs, and that costs nothing of the budgets.
    print(cx, &window, 1, b"\x1b]133;C\x07one\r");
    settle(cx, "it runs", |cx| shown(cx, 2).contains("one\n") || shown(cx, 2).lines().any(|l| l == "one"));
    let still = at_once(ask(cx, run_of(2, "echo two"), None, &who.outside));
    assert!(matches!(code(&still).0, Some(ErrorCode::Refused)));
    // Another terminal is its own.
    at_a_prompt(cx, &window, 0, 1);
    assert!(matches!(ask(cx, run_of(1, "echo other"), None, &who.outside), Answer::Wait(_)));
    // The first ends, and the terminal takes the next run.
    print(cx, &window, 1, b"\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07\r");
    let done = ran(&ended(cx, &first));
    assert_eq!(done.exit_code, Some(0));
    // (The line that ends it is echoed by the terminal's line discipline into the output; that is the test's.)
    assert!(done.output.is_some_and(|output| output.starts_with("one")));
    settle(cx, "the claim is given back", |_| bus.claim_run(2).is_some());
}

#[gpui_kit::test]
fn a_wait_on_a_terminal_whose_shell_ended_answers_at_once_unless_that_is_what_it_waits_for(
    cx: &mut TestAppContext,
) {
    if isolate("a_wait_on_a_terminal_whose_shell_ended_answers_at_once_unless_that_is_what_it_waits_for") {
        return;
    }
    let window = start(cx, 2);
    let bus = app_bus(cx);
    let who = callers(cx, &window);
    let first = terminal(cx, &window, 0);
    event(cx, &first, PtyEvent::Exited(Some(0)));
    cx.run_until_parked();
    let gone = at_once(ask(cx, wait_for(1, |s| s.status = vec![TerminalState::Idle]), None, &who.outside));
    assert_eq!(code(&gone), (Some(ErrorCode::NotFound), Some(Reason::Exited)), "{gone:?}");
    let by_text = at_once(ask(cx, wait_for(1, |s| s.stable_ms = Some(500)), None, &who.outside));
    assert_eq!(code(&by_text).0, Some(ErrorCode::NotFound));
    assert_eq!(bus.subscribers(), 0);
    let exited =
        at_once(ask(cx, wait_for(1, |s| s.status = vec![TerminalState::Exited]), None, &who.outside));
    assert_eq!(waited(&exited).status, Some(TerminalState::Exited));
}

#[gpui_kit::test]
fn block_events_carry_their_prompts_number_even_when_marks_are_heard_in_a_batch(cx: &mut TestAppContext) {
    if isolate("block_events_carry_their_prompts_number_even_when_marks_are_heard_in_a_batch") {
        return;
    }
    use wormhole_core::ipc::bus::Grant;
    use wormhole_core::ipc::control::EventsSpec;
    use wormhole_core::ipc::control::event::{BlockPhase, Body};

    let window = start(cx, 1);
    let bus = app_bus(cx);
    let spec = EventsSpec { topics: vec!["blocks".into()], ..EventsSpec::default() };
    let (stream, _) = bus
        .subscribe(
            spec.filter().expect("valid"),
            Grant { reads_all: true, own: Vec::new(), ..Grant::default() },
            None,
        )
        .expect("room");
    // A fast command and then an empty Enter, in one print: by the time the window hears of the first
    // command's end, the shell has marked two more prompts.
    print(
        cx,
        &window,
        0,
        b"\x1b]133;A\x07$ \x1b]133;B\x07true\r\x1b]133;C\x07\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07\r\x1b]133;D\x07\x1b]133;A\x07$ \x1b]133;B\x07\r",
    );
    let mut seen = Vec::new();
    settle(cx, "the command's end", |_| {
        while let wormhole_core::ipc::bus::Next::Event(published) = stream.next(Duration::ZERO) {
            if let Body::Block { block, phase, exit_code, .. } = &published.event.body {
                seen.push((*block, *phase, *exit_code));
            }
        }
        seen.iter().any(|(_, phase, _)| *phase == BlockPhase::Finished)
    });
    cx.run_until_parked();
    // The command is the first block: told started and finished, by its own number, and the empty
    // Enter that came after it says nothing at all.
    assert_eq!(seen, [(1, BlockPhase::Started, None), (1, BlockPhase::Finished, Some(0))], "{seen:?}");
}
