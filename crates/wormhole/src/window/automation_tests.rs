//! What a window tells those who watch it (`wh events`), in a real window: the right events with the
//! right ids for what happens to terminals, windows and splits, the commands a shell marks, who may
//! type into a terminal and who may hear its commands, and that watching costs a frame nothing.
//! The harness is `test_harness`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::ipc::bus::{Bus, Grant, Next, Published, Subscription};
use wormhole_core::ipc::control::event::{BlockPhase, Body, EventsSpec};
use wormhole_core::ipc::control::{Action, ErrorCode, Reason, Request, Target, TerminalState};
use wormhole_core::ipc::{Answer, Peer};
use wormhole_term::PtyEvent;

use crate::app::control;
use crate::app_state::OpenWindow;
use crate::settings::update_settings;
use crate::window::reported_tests::{event, terminal};
use crate::window::test_harness::{
    draw, draw_fresh, drive, hook, isolate_in, press, read, start, with_window,
};
use crate::window::window_lifecycle::{open_workspace_window, window_closed};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// The app's bus, and a subscription to the `topics` (all but output without any) that may read
/// the terminals in `own`.
fn watch(cx: &mut TestAppContext, topics: &[&str], own: Vec<i64>) -> (Arc<Bus>, Subscription) {
    let bus = cx.update(|cx| match crate::automation::bus(cx) {
        Some(bus) => bus,
        None => {
            let bus = Bus::new();
            crate::automation::install(bus.clone(), cx);
            bus
        }
    });
    let spec = EventsSpec { topics: topics.iter().map(|t| t.to_string()).collect(), ..EventsSpec::default() };
    let filter = spec.filter().expect("valid topics");
    let (subscription, _) = bus.subscribe(filter, Grant { reads_all: false, own }, None).expect("room");
    (bus, subscription)
}

/// Everything the subscription has been told so far.
fn told(subscription: &Subscription) -> Vec<Arc<Published>> {
    let mut out = Vec::new();
    while let Next::Event(published) = subscription.next(Duration::ZERO) {
        out.push(published);
    }
    out
}

/// The events of one kind.
fn of_kind<'a>(events: &'a [Arc<Published>], kind: &str) -> Vec<&'a Arc<Published>> {
    events.iter().filter(|published| published.event.body.name() == kind).collect()
}

fn ids(published: &Published) -> (Option<i64>, Option<i64>) {
    (published.event.window, published.event.terminal)
}

/// Settle what is in motion until `done`, with the real clock (a shell's output comes from its
/// reader thread).
fn wait_for(cx: &mut TestAppContext, what: &str, mut done: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    cx.run_until_parked();
    while !done() {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(10));
        cx.run_until_parked();
    }
}

#[gpui_kit::test]
fn a_terminals_state_title_and_bell_reach_a_watcher_with_their_ids(cx: &mut TestAppContext) {
    if isolate("a_terminals_state_title_and_bell_reach_a_watcher_with_their_ids") {
        return;
    }
    let window = start(cx, 2);
    let (_bus, watcher) = watch(cx, &[], vec![]);
    let second = terminal(cx, &window, 1);

    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Allow Bash?"));
    let told_now = told(&watcher);
    let status = of_kind(&told_now, "status");
    assert_eq!(status.len(), 1, "{told_now:?}");
    assert_eq!(ids(status[0]), (Some(1), Some(2)));
    let Body::Status { status: now, was, agent, message } = &status[0].event.body else { panic!("a status") };
    assert_eq!((*now, *was), (TerminalState::NeedsYou, TerminalState::Idle));
    assert_eq!((agent.as_deref(), message.as_deref()), (Some("claude"), Some("Allow Bash?")));

    event(cx, &second, PtyEvent::Bell);
    event(cx, &second, PtyEvent::Title("make test".into()));
    cx.run_until_parked();
    let told_now = told(&watcher);
    assert_eq!(of_kind(&told_now, "bell").iter().map(|p| ids(p)).collect::<Vec<_>>(), [(Some(1), Some(2))]);
    let titles = of_kind(&told_now, "title");
    assert_eq!(titles.len(), 1, "{told_now:?}");
    assert_eq!(ids(titles[0]), (Some(1), Some(2)));
    let Body::Title { title, name } = &titles[0].event.body else { panic!("a title") };
    assert!(title.as_deref().is_some_and(|t| t.contains("make")) && !name.is_empty(), "{title:?} {name:?}");

    // Every event has its own, higher number.
    let numbers: Vec<u64> = told(&watcher).iter().map(|p| p.seq).collect();
    assert!(numbers.is_empty(), "all of it was read above: {numbers:?}");
}

#[gpui_kit::test]
fn terminals_splits_names_and_windows_come_and_go_with_their_ids(cx: &mut TestAppContext) {
    if isolate("terminals_splits_names_and_windows_come_and_go_with_their_ids") {
        return;
    }
    let window = start(cx, 1);
    let (_bus, watcher) = watch(cx, &["terminals", "splits", "windows"], vec![]);

    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    let opened = told(&watcher);
    let opened = of_kind(&opened, "terminal_opened");
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].event.window, Some(1));
    let new_terminal = opened[0].event.terminal.expect("its id");
    assert_ne!(new_terminal, 1);

    drive(cx, &window, |ws, _, cx| ws.set_custom_name(0, Some("api".into()), cx));
    let named = told(&watcher);
    let named = of_kind(&named, "renamed");
    assert_eq!(named.len(), 1);
    assert_eq!(ids(named[0]), (Some(1), Some(1)));
    assert_eq!(named[0].event.body, Body::Renamed { name: Some("api".into()) });
    drive(cx, &window, |ws, _, cx| ws.set_custom_name(0, None, cx));
    let cleared = told(&watcher);
    assert_eq!(of_kind(&cleared, "renamed")[0].event.body, Body::Renamed { name: None });

    // ⌘D: a new terminal beside the selected one, and the split they share.
    draw(cx, &window);
    press(cx, &window, "cmd-d");
    let made = told(&watcher);
    let split = of_kind(&made, "split");
    assert_eq!(split.len(), 1, "{made:?}");
    let Body::Split { split: split_id, terminals, focused } = &split[0].event.body else { panic!("a split") };
    assert_eq!(terminals.len(), 2);
    assert_eq!(*focused, terminals.last().copied(), "the new pane has the keyboard");
    assert_eq!(split[0].event.window, Some(1));
    assert_eq!(of_kind(&made, "terminal_opened").len(), 1, "the pane is a terminal of its own");
    let (split_id, pane) = (*split_id, terminals[1]);

    // ⌘W: its pane goes, and with one left the split is gone: the same id, no members.
    draw(cx, &window);
    press(cx, &window, "cmd-w");
    let closed = told(&watcher);
    assert_eq!(
        of_kind(&closed, "terminal_closed").iter().map(|p| ids(p)).collect::<Vec<_>>(),
        [(Some(1), Some(pane))]
    );
    let gone = of_kind(&closed, "split");
    assert_eq!(gone.len(), 1, "{closed:?}");
    assert_eq!(gone[0].event.body, Body::Split { split: split_id, terminals: vec![], focused: None });

    // A second window: it is told before the terminals in it.
    cx.update(|cx| cx.on_window_closed(window_closed).detach());
    cx.update(|cx| open_workspace_window(None, None, cx));
    cx.run_until_parked();
    let windows = told(&watcher);
    let names: Vec<_> = windows.iter().map(|p| p.event.body.name()).collect();
    assert_eq!(names, ["window_opened", "terminal_opened"], "{windows:?}");
    let second = windows[0].event.window.expect("its id");
    assert_ne!(second, 1);
    assert_eq!(windows[0].event.terminal, None);
    assert_eq!(windows[1].event.window, Some(second), "its terminal opened in it");
    let in_it = windows[1].event.terminal.expect("its id");

    // Its end, as the window system does it: its terminals close first, then it does.
    let listed = cx.update(|cx| {
        crate::app_state::app(cx).windows.iter().find(|w| w.id == second).cloned().expect("listed")
    });
    with_window(cx, &listed, |window, _| window.remove_window());
    cx.run_until_parked();
    let after = told(&watcher);
    let shape: Vec<_> = after.iter().map(|p| (p.event.body.name(), ids(p))).collect();
    assert_eq!(
        shape,
        [("terminal_closed", (Some(second), Some(in_it))), ("window_closed", (Some(second), None))],
        "{after:?}"
    );
}

#[gpui_kit::test]
fn a_command_the_shell_marks_is_told_with_its_exit_code_and_its_line_to_those_who_may_read_it(
    cx: &mut TestAppContext,
) {
    if isolate("a_command_the_shell_marks_is_told_with_its_exit_code_and_its_line_to_those_who_may_read_it") {
        return;
    }
    let window = start(cx, 2);
    // Two watchers: one that may read terminal 1 (it runs in it), one that may not.
    let (_bus, inside) = watch(cx, &["blocks"], vec![1]);
    let (_bus, outside) = watch(cx, &["blocks"], vec![]);
    let first = terminal(cx, &window, 0);
    // `cat` is the shell: it prints back what it is given, marks and all, as a shell with
    // integration would print them around a command that fails with 3.
    let marks = b"\x1b]133;A\x07$ \x1b]133;B\x07ls\r\x1b]133;C\x07a.rs\r\x1b]133;D;3\x07\r";
    cx.update(|cx| first.update(cx, |t, _| t.write_input(marks.to_vec())));
    let mut all = Vec::new();
    wait_for(cx, "the command's end arrives", || {
        all.extend(told(&inside));
        !of_kind(&all, "block")
            .iter()
            .all(|p| matches!(p.event.body, Body::Block { phase: BlockPhase::Started, .. }))
            || of_kind(&all, "block").len() >= 2
    });
    let blocks = of_kind(&all, "block");
    assert_eq!(blocks.len(), 2, "started, then finished: {all:?}");
    assert_eq!(ids(blocks[0]), (Some(1), Some(1)));
    let Body::Block { phase, exit_code, took_ms, .. } = &blocks[0].event.body else { panic!("a block") };
    assert_eq!((*phase, *exit_code, *took_ms), (BlockPhase::Started, None, None));
    let Body::Block { phase, exit_code, took_ms, block, .. } = &blocks[1].event.body else {
        panic!("a block")
    };
    assert_eq!((*phase, *exit_code), (BlockPhase::Finished, Some(3)));
    assert!(took_ms.is_some() && *block >= 1);

    // The line typed is terminal text: told to the watcher inside the terminal, not to the other.
    let line = |watcher: &Subscription, published: &Published| watcher.line(published).to_string();
    assert!(line(&inside, blocks[1]).contains(r#""command":"ls""#), "{}", line(&inside, blocks[1]));
    let outside_told = told(&outside);
    let outside_finished = of_kind(&outside_told, "block");
    assert_eq!(outside_finished.len(), 2);
    let text = line(&outside, outside_finished[1]);
    assert!(text.contains(r#""exit_code":3"#) && !text.contains("command"), "{text}");
}

/// The events a watcher hears cost a frame nothing: nothing publishes while a window draws.
#[gpui_kit::test]
fn idle_watchers_cost_a_frame_nothing(cx: &mut TestAppContext) {
    if isolate("idle_watchers_cost_a_frame_nothing") {
        return;
    }
    use crate::perf::allocations::thread_counts;
    let window = start(cx, 3);
    let frame = |cx: &mut TestAppContext, window: &OpenWindow| {
        let before = thread_counts();
        draw_fresh(cx, window);
        thread_counts().since(before).allocations
    };
    for _ in 0..3 {
        draw(cx, &window);
    }
    let without = frame(cx, &window);
    // Four watchers on everything, of terminals that are not there: the bus is listening for every
    // topic, and nothing matches.
    let bus = Bus::new();
    cx.update(|cx| crate::automation::install(bus.clone(), cx));
    let spec = EventsSpec { terminals: vec![999], ..EventsSpec::default() };
    let watchers: Vec<_> = (0..4)
        .map(|n| bus.subscribe(spec.filter().expect("valid"), Grant::default(), Some(n)).expect("room").0)
        .collect();
    for _ in 0..2 {
        draw_fresh(cx, &window);
    }
    let with = frame(cx, &window);
    assert_eq!(with, without, "a frame with four idle watchers allocates what it does with none");
    assert!(watchers.iter().all(|w| told(w).is_empty()), "and tells them nothing");
}

/// One round of what a terminal does while a command runs: an agent's state moves, the bell rings, the
/// shell marks a command's start and end, output arrives, and a frame is drawn.
fn activity(cx: &mut TestAppContext, window: &OpenWindow) -> u64 {
    use crate::perf::allocations::thread_counts;
    use wormhole_term::{Mark, PtyEvent};

    let t = terminal(cx, window, 0);
    let before = thread_counts();
    hook(cx, window, 1, "UserPromptSubmit", None);
    event(cx, &t, PtyEvent::Bell);
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart));
    event(cx, &t, PtyEvent::Output);
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(0))));
    hook(cx, window, 1, "Stop", None);
    draw_fresh(cx, window);
    thread_counts().since(before).allocations
}

/// Watching costs what changes, not the watcher: with subscribers who listen for something else, the
/// same activity allocates exactly what it does with none; one who listens for it makes the app build
/// the events (so the comparison measures something).
#[gpui_kit::test]
fn watchers_of_other_things_cost_activity_nothing(cx: &mut TestAppContext) {
    if isolate("watchers_of_other_things_cost_activity_nothing") {
        return;
    }
    let window = start(cx, 2);
    for _ in 0..3 {
        activity(cx, &window);
    }
    let without = activity(cx, &window);

    // Four watchers of windows opening: the bus is listening, but not for any of this.
    let bus = Bus::new();
    cx.update(|cx| crate::automation::install(bus.clone(), cx));
    let spec = EventsSpec { topics: vec!["windows".into()], ..EventsSpec::default() };
    let others: Vec<_> = (0..4)
        .map(|n| bus.subscribe(spec.filter().expect("valid"), Grant::default(), Some(n)).expect("room").0)
        .collect();
    for _ in 0..2 {
        activity(cx, &window);
    }
    let with_others = activity(cx, &window);
    assert_eq!(with_others, without, "activity allocates what it does with nobody watching");
    assert!(others.iter().all(|w| told(w).is_empty()));

    let (_bus, listener) = watch(cx, &["status", "bell", "blocks"], vec![]);
    activity(cx, &window);
    let listened = activity(cx, &window);
    assert!(listened > without, "{listened} against {without}: the events are built for someone who listens");
    let heard = told(&listener);
    // (The marks here are events, not bytes in the grid, so there is no block to name: those are
    // told by the tests that feed a shell's output.)
    for kind in ["status", "bell"] {
        assert!(!of_kind(&heard, kind).is_empty(), "{kind} in {heard:?}");
    }
}

#[gpui_kit::test]
fn a_caller_that_runs_inside_a_terminal_cannot_type_into_it_however_it_asks(cx: &mut TestAppContext) {
    if isolate("a_caller_that_runs_inside_a_terminal_cannot_type_into_it_however_it_asks") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| update_settings(cx, |settings| settings.control_write = true));
    let shell = |cx: &mut TestAppContext, ix: usize| {
        terminal(cx, &window, ix).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs")
    };
    let (first, second) = (shell(cx, 0), shell(cx, 1));
    let send = |cx: &mut TestAppContext, from: Option<&str>, peer: Peer, to: i64| {
        let request = Request::new(
            Action::SendText { target: Target::Terminal(to), text: "ls".into(), submit: false },
            from.map(str::to_string),
        );
        cx.update(|cx| control::handle(request, &peer, cx))
    };
    let code = |reply: &wormhole_core::ipc::control::Reply| reply.err().map(|e| e.code);
    // Running under terminal 1's shell, saying nothing about itself, or something false: refused.
    let inside = Peer::new(900_001, vec![first, 1]);
    assert_eq!(code(&send(cx, None, inside.clone(), 1)), Some(ErrorCode::Refused), "no claim at all");
    assert_eq!(code(&send(cx, Some("2"), inside.clone(), 1)), Some(ErrorCode::Refused), "a false claim");
    assert_eq!(code(&send(cx, Some("1"), Peer::default(), 1)), Some(ErrorCode::Refused), "the claim alone");
    // Its parent gone (`(wh send-text … &)` is handed to launchd): no chain, but the same terminal
    // device as the shell it was started in, which is as good.
    let tty = wormhole_core::platform::process::tty(first).expect("a shell has a terminal");
    let orphan = Peer::new(900_003, vec![]).on_tty(tty);
    assert_eq!(code(&send(cx, None, orphan.clone(), 1)), Some(ErrorCode::Refused), "an orphan, no claim");
    assert_eq!(code(&send(cx, None, orphan, 2)), None, "and another terminal is another device");
    // Under terminal 1, typing into terminal 2 is another terminal: allowed.
    assert_eq!(code(&send(cx, None, inside, 2)), None);
    // From outside every terminal, anywhere.
    let outside = Peer::new(900_002, vec![1]);
    assert_eq!(code(&send(cx, None, outside, 1)), None);
    let _ = second;
}

#[gpui_kit::test]
fn events_are_answered_with_the_state_and_a_stream_or_a_reason_why_not(cx: &mut TestAppContext) {
    if isolate("events_are_answered_with_the_state_and_a_stream_or_a_reason_why_not") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| crate::automation::install(Bus::new(), cx));
    let ask = |cx: &mut TestAppContext, spec: EventsSpec, from: Option<&str>, peer: Peer| {
        let request = Request::new(Action::Events(spec), from.map(str::to_string));
        cx.update(|cx| control::answer(request, &peer, cx))
    };
    let refusal = |answer: Answer| match answer {
        Answer::Reply(reply) => reply.error.expect("an error"),
        Answer::Stream { .. } | Answer::Read(_) => panic!("not a refusal"),
    };

    // The state as of the moment, then the stream: what happens after has a higher number.
    let Answer::Stream { reply, subscription } = ask(cx, EventsSpec::default(), None, Peer::default()) else {
        panic!("a stream")
    };
    let result = reply.result.expect("a result");
    assert_eq!(result["subscribed"], true);
    assert_eq!(result["boot"].as_str().map(str::len), Some(16));
    let first_seq = result["seq"].as_u64().expect("a seq");
    assert_eq!(result["listing"]["windows"][0]["terminals"].as_array().map(Vec::len), Some(2));
    assert!(result["topics"].as_array().is_some_and(|t| t.len() == 8 && !t.iter().any(|n| n == "output")));
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Allow?"));
    let heard = told(&subscription);
    assert!(!heard.is_empty() && heard.iter().all(|p| p.seq > first_seq));

    // What is wrong with it is said.
    let error = refusal(ask(
        cx,
        EventsSpec { topics: vec!["nope".into()], ..Default::default() },
        None,
        Peer::default(),
    ));
    assert_eq!(error.code, ErrorCode::BadRequest);
    // What terminals print needs the right to read (the setting is off, and the caller is outside).
    let error = refusal(ask(cx, EventsSpec { output: true, ..Default::default() }, None, Peer::default()));
    assert_eq!(error.code, ErrorCode::NotAllowed);
    let wrong_version = Request { version: 99, ..Request::new(Action::Events(EventsSpec::default()), None) };
    let answered = cx.update(|cx| control::answer(wrong_version, &Peer::default(), cx));
    assert_eq!(refusal(answered).code, ErrorCode::Version);

    // Four streams to a caller, and the fifth is turned away.
    let mut held = vec![subscription];
    for _ in 0..3 {
        let Answer::Stream { subscription, .. } = ask(cx, EventsSpec::default(), None, Peer::default())
        else {
            panic!("a stream")
        };
        held.push(subscription);
    }
    let error = refusal(ask(cx, EventsSpec::default(), None, Peer::default()));
    assert_eq!(error.code, ErrorCode::RateLimited);
    assert!(error.message.contains('4'), "{}", error.message);
    drop(held);

    // With the flag off, the new action says so.
    cx.update(|cx| {
        update_settings(cx, |settings| {
            settings.feature_flags.insert("automation".into(), false);
        });
    });
    let error = refusal(ask(cx, EventsSpec::default(), None, Peer::default()));
    assert_eq!((error.code, error.reason), (ErrorCode::Unsupported, Some(Reason::FlagOff)));
}

#[gpui_kit::test]
fn what_a_caller_may_read_is_the_terminals_it_runs_in(cx: &mut TestAppContext) {
    if isolate("what_a_caller_may_read_is_the_terminals_it_runs_in") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| crate::automation::install(Bus::new(), cx));
    let shell = terminal(cx, &window, 0).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs");
    let events_for = |cx: &mut TestAppContext, peer: Peer| {
        let request = Request::new(Action::Events(EventsSpec::default()), None);
        match cx.update(|cx| control::answer(request, &peer, cx)) {
            Answer::Stream { subscription, .. } => subscription,
            Answer::Reply(_) | Answer::Read(_) => panic!("not a stream"),
        }
    };
    let inside = events_for(cx, Peer::new(900_001, vec![shell]));
    let outside = events_for(cx, Peer::new(900_002, vec![1]));
    // A command's line is told to the one inside terminal 1 and to nobody else.
    let marks = b"\x1b]133;A\x07$ \x1b]133;B\x07whoami\r\x1b]133;C\x07me\r\x1b]133;D;0\x07\r";
    let first = terminal(cx, &window, 0);
    cx.update(|cx| first.update(cx, |t, _| t.write_input(marks.to_vec())));
    let mut all = Vec::new();
    wait_for(cx, "the command ends", || {
        all.extend(told(&inside));
        of_kind(&all, "block").len() >= 2
    });
    let finished = of_kind(&all, "block")[1];
    assert!(inside.line(finished).contains(r#""command":"whoami""#));
    let for_outsider = told(&outside);
    let theirs = of_kind(&for_outsider, "block");
    assert!(!theirs.is_empty() && theirs.iter().all(|p| !outside.line(p).contains("whoami")));
    let _ = read(cx, &window, |ws| ws.sessions().len());
}
