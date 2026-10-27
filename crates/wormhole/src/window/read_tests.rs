//! Reading a terminal, watching what it prints and driving a program in it from the control line, in a real window whose
//! terminals run `/bin/cat`: its line discipline echoes what is typed (keys as `^[[A`), which shows
//! the bytes `send-keys` and `mouse` wrote, and a line sent back to it prints raw, which is how a test
//! makes the engine see a program's mode switches. The harness is `test_harness`.

use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::ipc::bus::{Bus, Next};
use wormhole_core::ipc::control::event::{Body, CloseReason, EventsSpec};
use wormhole_core::ipc::control::mouse::{ButtonName, Direction, Pointer, Wheel};
use wormhole_core::ipc::control::read::{Blocks, Range, Read};
use wormhole_core::ipc::control::{
    Action, BlocksSpec, ErrorCode, KeyItem, KeysSent, MouseOp, MouseSpec, ReadSpec, Reason, Reply, Request,
    SendKeysSpec, Target, TerminalState,
};
use wormhole_core::ipc::{Answer, Peer};
use wormhole_term::{PtyEvent, ReadScratch};

use crate::app::control;
use crate::app_state::OpenWindow;
use crate::settings::update_settings;
use crate::window::reported_tests::{event, terminal};
use crate::window::test_harness::{hook, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn ask(cx: &mut TestAppContext, action: Action, from: Option<&str>, peer: &Peer) -> Answer {
    let request = Request::new(action, from.map(str::to_string));
    cx.update(|cx| control::answer(request, peer, cx))
}

/// The reply, running the read the socket thread would.
fn reply(answer: Answer) -> Reply {
    match answer {
        Answer::Reply(reply) => reply,
        Answer::Read(job) => job.run(&mut ReadScratch::default()),
        Answer::Stream { .. } | Answer::Wait(_) => panic!("a stream"),
    }
}

/// A stream of `output` from the terminals `terminals` (all without any), for a caller with `peer`.
fn watch_output(
    cx: &mut TestAppContext,
    terminals: Vec<i64>,
    peer: &Peer,
) -> Result<wormhole_core::ipc::bus::Subscription, ErrorCode> {
    let spec = EventsSpec { terminals, output: true, topics: vec!["output".into()], ..EventsSpec::default() };
    match ask(cx, Action::Events(spec), None, peer) {
        Answer::Stream { subscription, .. } => Ok(subscription),
        Answer::Reply(reply) => Err(code(&reply).expect("a refusal")),
        Answer::Read(_) | Answer::Wait(_) => panic!("a read"),
    }
}

/// The lines of `output` events told so far for terminal `id`, until `wanted` is among them.
fn output_until(
    cx: &mut TestAppContext,
    stream: &wormhole_core::ipc::bus::Subscription,
    id: i64,
    wanted: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    wait_until(cx, wanted, |_| {
        while let Next::Event(published) = stream.next(Duration::ZERO) {
            if let Body::Output { lines: told, .. } = &published.event.body {
                assert_eq!(published.event.terminal, Some(id), "an event of another terminal: {told:?}");
                assert_eq!(published.event.window, Some(1));
                lines.extend(told.iter().cloned());
            }
        }
        lines.iter().any(|line| line.contains(wanted))
    });
    lines
}

fn code(reply: &Reply) -> Option<ErrorCode> {
    reply.err().map(|error| error.code)
}

fn read_of(target: i64) -> Action {
    Action::Read(ReadSpec {
        target: Some(Target::Terminal(target)),
        range: Range::Screen,
        join: false,
        styles: false,
    })
}

fn keys_of(target: i64, keys: Vec<KeyItem>) -> Action {
    Action::SendKeys(SendKeysSpec { target: Target::Terminal(target), keys })
}

fn key(name: &str) -> KeyItem {
    KeyItem::Key { key: name.into() }
}

fn text(text: &str, paste: bool) -> KeyItem {
    KeyItem::Text { text: text.into(), paste }
}

fn pointer(row: u32, col: u32) -> Pointer {
    Pointer { row, col, button: Some(ButtonName::Left), count: 1, shift: false, alt: false, ctrl: false }
}

/// A caller outside every terminal, and one running under terminal `ix`'s shell.
struct Callers {
    outside: Peer,
    inside_first: Peer,
}

fn callers(cx: &mut TestAppContext, window: &OpenWindow) -> Callers {
    let shell = terminal(cx, window, 0).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs");
    Callers { outside: Peer::new(900_002, vec![1]), inside_first: Peer::new(900_001, vec![shell]) }
}

fn allow(cx: &mut TestAppContext, read: bool, write: bool) {
    cx.update(|cx| {
        update_settings(cx, |settings| {
            settings.control_read = read;
            settings.control_write = write;
        })
    });
}

/// What terminal `id` shows now, read as the socket thread reads it (with the switch on).
fn shown(cx: &mut TestAppContext, id: i64, callers: &Callers) -> Read {
    let reply = reply(ask(cx, read_of(id), None, &callers.outside));
    let result = reply.result.clone();
    serde_json::from_value(result.unwrap_or_else(|| panic!("{reply:?}"))).expect("a read")
}

fn show_contains(cx: &mut TestAppContext, id: i64, callers: &Callers, what: &str) {
    wait_until(cx, what, |cx| shown(cx, id, callers).lines.iter().any(|line| line.contains(what)));
}

/// Settle what is in motion until `done`, with the real clock (a terminal's output comes from its
/// reader thread).
fn wait_until(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    cx.run_until_parked();
    while !done(cx) {
        assert!(Instant::now() < until, "{what}");
        std::thread::sleep(Duration::from_millis(10));
        cx.run_until_parked();
    }
}

/// Print `bytes` in terminal `ix` as a program would: `cat` echoes the line back raw once it is
/// complete, so a sequence ending in Return reaches the engine.
fn print(cx: &mut TestAppContext, window: &OpenWindow, ix: usize, bytes: &[u8]) {
    let t = terminal(cx, window, ix);
    cx.update(|cx| t.update(cx, |t, _| t.write_input(bytes.to_vec())));
}

#[gpui_kit::test]
fn doctor_tells_the_flag_the_switches_and_which_terminal_the_caller_is_in(cx: &mut TestAppContext) {
    use wormhole_core::ipc::control::doctor::Facts;
    if isolate("doctor_tells_the_flag_the_switches_and_which_terminal_the_caller_is_in") {
        return;
    }
    let window = start(cx, 2);
    let who = callers(cx, &window);
    let facts = |cx: &mut TestAppContext, from: Option<&str>, peer: &Peer| -> Facts {
        let reply = reply(ask(cx, Action::Doctor, from, peer));
        serde_json::from_value(reply.result.clone().unwrap_or_else(|| panic!("{reply:?}"))).expect("facts")
    };
    allow(cx, false, false);
    let outside = facts(cx, None, &who.outside);
    assert_eq!(outside.protocol, wormhole_core::ipc::control::PROTOCOL_VERSION);
    assert_eq!((outside.control_read, outside.control_write), (false, false));
    assert!(outside.caller.is_none(), "not in any terminal");
    // Found by where it runs: the terminal whose shell it runs under, whatever it says.
    let inside = facts(cx, None, &who.inside_first).caller.expect("inside terminal 1");
    assert_eq!((inside.terminal, inside.verified), (1, true));
    let lying = facts(cx, Some("2"), &who.inside_first).caller.expect("still terminal 1");
    assert_eq!((lying.terminal, lying.verified), (1, true));
    // Only claimed: named, and told not to be taken for it.
    let claimed = facts(cx, Some("2"), &who.outside).caller.expect("claims terminal 2");
    assert_eq!((claimed.terminal, claimed.verified), (2, false));
    // `/bin/cat` is no shell with integration: no prompt mark has come.
    assert_eq!(inside.shell_integration, Some(false));
    allow(cx, true, true);
    let on = facts(cx, None, &who.outside);
    assert_eq!((on.control_read, on.control_write), (true, true));
    // Asked with the flag off, it still answers: that is when it is needed.
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert("automation".into(), false);
        })
    });
    assert!(!facts(cx, None, &who.outside).automation);
}

#[gpui_kit::test]
fn reading_needs_the_switch_unless_the_caller_runs_in_the_terminal(cx: &mut TestAppContext) {
    if isolate("reading_needs_the_switch_unless_the_caller_runs_in_the_terminal") {
        return;
    }
    let window = start(cx, 2);
    let who = callers(cx, &window);
    allow(cx, false, false);
    // Off: no read, whatever is claimed. A claim is no proof.
    assert_eq!(code(&reply(ask(cx, read_of(1), None, &who.outside))), Some(ErrorCode::NotAllowed));
    assert_eq!(code(&reply(ask(cx, read_of(1), Some("1"), &who.outside))), Some(ErrorCode::NotAllowed));
    // Running under terminal 1's shell: its own, without the switch, also by default (no target).
    assert!(matches!(ask(cx, read_of(1), None, &who.inside_first), Answer::Read(_)));
    let own = Action::Read(ReadSpec { target: None, range: Range::Screen, join: false, styles: false });
    assert!(matches!(ask(cx, own.clone(), Some("1"), &who.inside_first), Answer::Read(_)));
    // But not another terminal's.
    assert_eq!(code(&reply(ask(cx, read_of(2), None, &who.inside_first))), Some(ErrorCode::NotAllowed));
    // No target and no terminal of the caller's own: a mistake in the request.
    assert_eq!(code(&reply(ask(cx, own, None, &who.outside))), Some(ErrorCode::BadRequest));
    // On: any terminal; one that isn't there is not found.
    allow(cx, true, false);
    assert!(matches!(ask(cx, read_of(2), None, &who.outside), Answer::Read(_)));
    assert_eq!(code(&reply(ask(cx, read_of(9), None, &who.outside))), Some(ErrorCode::NotFound));
    // The blocks follow the same rule.
    let blocks = Action::Blocks(BlocksSpec { target: Some(Target::Terminal(2)), last: None, output: false });
    assert!(matches!(ask(cx, blocks.clone(), None, &who.outside), Answer::Read(_)));
    allow(cx, false, false);
    assert_eq!(code(&reply(ask(cx, blocks, None, &who.outside))), Some(ErrorCode::NotAllowed));
    // With the flag off, the new actions say so.
    allow(cx, true, true);
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert("automation".into(), false);
        })
    });
    let off = reply(ask(cx, read_of(1), None, &who.outside));
    assert_eq!(
        (code(&off), off.err().and_then(|e| e.reason)),
        (Some(ErrorCode::Unsupported), Some(Reason::FlagOff))
    );
    let off = reply(ask(cx, keys_of(1, vec![key("enter")]), None, &who.outside));
    assert_eq!(off.err().and_then(|e| e.reason), Some(Reason::FlagOff));
}

#[gpui_kit::test]
fn a_read_has_the_screen_the_cursor_the_modes_and_the_title(cx: &mut TestAppContext) {
    if isolate("a_read_has_the_screen_the_cursor_the_modes_and_the_title") {
        return;
    }
    let window = start(cx, 1);
    let who = callers(cx, &window);
    allow(cx, true, false);
    let first = terminal(cx, &window, 0);
    event(cx, &first, PtyEvent::Title("make test".into()));
    print(cx, &window, 0, b"hello world\r");
    show_contains(cx, 1, &who, "hello world");
    let screen = shown(cx, 1, &who);
    assert_eq!(screen.terminal, 1);
    assert_eq!(screen.title.as_deref(), Some("make test"));
    assert_eq!(screen.screen, "main");
    assert_eq!(screen.lines.len(), screen.size.rows);
    assert!(screen.cursor.visible && screen.cursor.row >= 1, "{:?}", screen.cursor);
    assert!(!screen.modes.app_cursor && screen.modes.mouse == "off" && screen.modes.kitty_keyboard == 0);
    // The program switches modes: application cursor keys, bracketed paste, mouse, the alt screen.
    print(cx, &window, 0, b"\x1b[?1h\x1b[?2004h\x1b[?1000h\x1b[?1006h\r");
    wait_until(cx, "the modes", |cx| shown(cx, 1, &who).modes.app_cursor);
    let modes = shown(cx, 1, &who).modes;
    assert!(modes.bracketed_paste && modes.mouse == "click" && modes.mouse_encoding == "sgr", "{modes:?}");
    print(cx, &window, 0, b"\x1b[?1049h\x1b[H\x1b[2Jfull screen\r");
    wait_until(cx, "the alternate screen", |cx| shown(cx, 1, &who).screen == "alt");
    assert!(shown(cx, 1, &who).lines.iter().any(|line| line.contains("full screen")));
    // The history is another range of the same screen.
    let history = Action::Read(ReadSpec {
        target: Some(Target::Terminal(1)),
        range: Range::Last { lines: 3 },
        join: false,
        styles: true,
    });
    let result: Read =
        serde_json::from_value(reply(ask(cx, history, None, &who.outside)).result.expect("ok"))
            .expect("read");
    assert_eq!((result.lines.len(), result.styles.as_ref().map(Vec::len)), (3, Some(3)));
}

#[gpui_kit::test]
fn blocks_tell_what_ran_and_how_it_ended(cx: &mut TestAppContext) {
    if isolate("blocks_tell_what_ran_and_how_it_ended") {
        return;
    }
    let window = start(cx, 1);
    let who = callers(cx, &window);
    allow(cx, true, false);
    let blocks =
        Action::Blocks(BlocksSpec { target: Some(Target::Terminal(1)), last: Some(5), output: true });
    let none: Blocks =
        serde_json::from_value(reply(ask(cx, blocks.clone(), None, &who.outside)).result.expect("ok"))
            .expect("blocks");
    assert!(!none.integration && none.blocks.is_empty(), "cat has no shell integration yet");
    print(cx, &window, 0, b"\x1b]133;A\x07$ \x1b]133;B\x07whoami\r\x1b]133;C\x07me\r\x1b]133;D;0\x07\r");
    wait_until(cx, "the block", |cx| {
        let found: Blocks =
            serde_json::from_value(reply(ask(cx, blocks.clone(), None, &who.outside)).result.expect("ok"))
                .expect("blocks");
        found.blocks.iter().any(|b| b.command == "whoami" && b.state == "finished")
    });
    let found: Blocks =
        serde_json::from_value(reply(ask(cx, blocks, None, &who.outside)).result.expect("ok"))
            .expect("blocks");
    let ran = found.blocks.iter().find(|b| b.command == "whoami").expect("the command");
    assert_eq!(ran.exit_code, Some(0));
    assert!(found.integration);
}

#[gpui_kit::test]
fn keys_reach_the_program_encoded_for_the_modes_it_has_set(cx: &mut TestAppContext) {
    if isolate("keys_reach_the_program_encoded_for_the_modes_it_has_set") {
        return;
    }
    let window = start(cx, 2);
    let who = callers(cx, &window);
    allow(cx, true, false);
    // Typing is behind its own switch.
    let sent = reply(ask(cx, keys_of(2, vec![text("abc", false)]), None, &who.outside));
    assert_eq!(code(&sent), Some(ErrorCode::NotAllowed));
    allow(cx, true, true);
    let sent = reply(ask(cx, keys_of(2, vec![text("abc", false), key("up")]), None, &who.outside));
    let sent: KeysSent = serde_json::from_value(sent.result.expect("ok")).expect("sent");
    assert_eq!((sent.tab, sent.keys), (2, 2));
    // The terminal echoes the letters and the arrow as the line discipline shows control characters.
    show_contains(cx, 2, &who, "abc^[[A");
    // The program asks for application cursor keys: the next arrow is `ESC O A`.
    print(cx, &window, 1, b"\x1b[?1h\x1b[?2004h\r");
    wait_until(cx, "application cursor keys", |cx| shown(cx, 2, &who).modes.app_cursor);
    reply(ask(cx, keys_of(2, vec![key("up"), text("pasted", true)]), None, &who.outside));
    show_contains(cx, 2, &who, "^[OA^[[200~pasted^[[201~");
    // A caller under terminal 1's shell can type into terminal 2, never into its own.
    let into_second = reply(ask(cx, keys_of(2, vec![key("enter")]), None, &who.inside_first));
    assert_eq!(code(&into_second), None, "{into_second:?}");
    let into_own = reply(ask(cx, keys_of(1, vec![key("enter")]), None, &who.inside_first));
    assert_eq!(code(&into_own), Some(ErrorCode::Refused));
    let claimed = reply(ask(cx, keys_of(1, vec![key("enter")]), Some("1"), &who.outside));
    assert_eq!(code(&claimed), Some(ErrorCode::Refused), "the claim alone refuses too");
    // Never into a terminal waiting for the user's answer.
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Allow?"));
    let waiting = reply(ask(cx, keys_of(2, vec![text("y", false)]), None, &who.outside));
    assert_eq!(code(&waiting), Some(ErrorCode::Refused));
    // What is wrong with the request is said.
    for bad in [vec![key("cmd-q")], vec![text("a\nb", false)], vec![]] {
        assert_eq!(code(&reply(ask(cx, keys_of(1, bad), None, &who.outside))), Some(ErrorCode::BadRequest));
    }
    assert_eq!(
        code(&reply(ask(cx, keys_of(1, vec![key("a"); 65]), None, &who.outside))),
        Some(ErrorCode::TooLarge)
    );
    assert_eq!(
        code(&reply(ask(cx, keys_of(9, vec![key("a")]), None, &who.outside))),
        Some(ErrorCode::NotFound)
    );
}

/// `less` rings the bell at its end, which the sidebar shows as "needs you"; nothing waits for an answer
/// there, and an agent driving the program must go on. An agent's question is another thing.
#[gpui_kit::test]
fn a_bell_does_not_shut_an_agent_out_of_a_terminal_but_a_question_does(cx: &mut TestAppContext) {
    if isolate("a_bell_does_not_shut_an_agent_out_of_a_terminal_but_a_question_does") {
        return;
    }
    let window = start(cx, 2);
    let who = callers(cx, &window);
    allow(cx, true, true);
    let status_of = |cx: &mut TestAppContext, tab: i64| {
        let reply =
            reply(ask(cx, Action::Status { target: Some(Target::Terminal(tab)) }, None, &who.outside));
        let info: wormhole_core::ipc::control::StatusInfo =
            serde_json::from_value(reply.result.expect("ok")).expect("status");
        info.terminal.status
    };
    // Terminal 1 (the second) is not the one in view, so a bell counts.
    let second = terminal(cx, &window, 1);
    event(cx, &second, PtyEvent::Bell);
    cx.run_until_parked();
    assert_eq!(status_of(cx, 2), TerminalState::NeedsYou, "the bell shows in the sidebar");
    let typed = reply(ask(cx, keys_of(2, vec![key("q")]), None, &who.outside));
    assert_eq!(code(&typed), None, "{typed:?}");
    let pasted = reply(ask(
        cx,
        Action::SendText { target: Target::Terminal(2), text: "q".into(), submit: false },
        None,
        &who.outside,
    ));
    assert_eq!(code(&pasted), None, "{pasted:?}");
    // An agent's question is a wait for an answer.
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Allow?"));
    assert_eq!(status_of(cx, 2), TerminalState::NeedsYou);
    let waiting = reply(ask(cx, keys_of(2, vec![key("q")]), None, &who.outside));
    assert_eq!(code(&waiting), Some(ErrorCode::Refused));
}

#[gpui_kit::test]
fn the_mouse_reaches_only_a_program_that_asked_for_it(cx: &mut TestAppContext) {
    if isolate("the_mouse_reaches_only_a_program_that_asked_for_it") {
        return;
    }
    let window = start(cx, 1);
    let who = callers(cx, &window);
    allow(cx, true, true);
    let mouse = |op: MouseOp| Action::Mouse(MouseSpec { target: Target::Terminal(1), op });
    let click = MouseOp::Click(pointer(1, 2));
    let refused = reply(ask(cx, mouse(click.clone()), None, &who.outside));
    assert_eq!(
        (code(&refused), refused.err().and_then(|e| e.reason)),
        (Some(ErrorCode::Refused), Some(Reason::NoMouse))
    );
    print(cx, &window, 0, b"\x1b[?1000h\x1b[?1006h\r");
    wait_until(cx, "mouse reporting", |cx| shown(cx, 1, &who).modes.mouse == "click");
    let sent = reply(ask(cx, mouse(click), None, &who.outside));
    assert_eq!(code(&sent), None, "{sent:?}");
    show_contains(cx, 1, &who, "^[[<0;3;2M^[[<0;3;2m");
    let wheel = MouseOp::Scroll(Wheel {
        row: 0,
        col: 0,
        direction: Direction::Down,
        lines: 2,
        shift: false,
        alt: false,
        ctrl: false,
    });
    assert_eq!(code(&reply(ask(cx, mouse(wheel), None, &who.outside))), None);
    show_contains(cx, 1, &who, "^[[<65;1;1M^[[<65;1;1M");
    // Outside the grid, or out of range, or into itself: refused with the reason.
    let far = reply(ask(cx, mouse(MouseOp::Press(pointer(10_000, 0))), None, &who.outside));
    assert_eq!(code(&far), Some(ErrorCode::BadRequest));
    let many = MouseOp::Click(Pointer { count: 9, ..pointer(0, 0) });
    assert_eq!(code(&reply(ask(cx, mouse(many), None, &who.outside))), Some(ErrorCode::BadRequest));
    let own = reply(ask(cx, mouse(MouseOp::Move(pointer(0, 0))), None, &who.inside_first));
    assert_eq!(code(&own), Some(ErrorCode::Refused));
    allow(cx, true, false);
    assert_eq!(
        code(&reply(ask(cx, mouse(MouseOp::Move(pointer(0, 0))), None, &who.outside))),
        Some(ErrorCode::NotAllowed)
    );
}

#[gpui_kit::test]
fn reads_and_keys_have_budgets_per_caller(cx: &mut TestAppContext) {
    if isolate("reads_and_keys_have_budgets_per_caller") {
        return;
    }
    let window = start(cx, 2);
    let who = callers(cx, &window);
    allow(cx, true, true);
    let budget = wormhole_core::ipc::control::Limited::SendKeys.budget();
    for n in 0..budget {
        let sent = reply(ask(cx, keys_of(2, vec![key("space")]), Some("1"), &who.outside));
        assert_eq!(code(&sent), None, "request {n}: {sent:?}");
    }
    let over = reply(ask(cx, keys_of(2, vec![key("space")]), Some("1"), &who.outside));
    assert_eq!(code(&over), Some(ErrorCode::RateLimited));
    // Another caller has a budget of its own, and a read is another kind.
    let other = reply(ask(cx, keys_of(2, vec![key("space")]), Some("2"), &who.outside));
    assert_eq!(code(&other), Some(ErrorCode::Refused), "tab 2 is the one asking: refused, not rate limited");
    assert_eq!(code(&reply(ask(cx, read_of(2), Some("1"), &who.outside))), None);
    let _ = Duration::ZERO;
}

#[gpui_kit::test]
fn what_a_terminal_prints_is_told_to_those_who_may_read_it_with_the_ids(cx: &mut TestAppContext) {
    if isolate("what_a_terminal_prints_is_told_to_those_who_may_read_it_with_the_ids") {
        return;
    }
    let window = start(cx, 2);
    cx.update(|cx| crate::automation::install(Bus::new(), cx));
    let who = callers(cx, &window);
    allow(cx, false, false);
    // Without the setting: not all terminals, not the other one's, only the caller's own, by name.
    assert_eq!(watch_output(cx, vec![], &who.outside).err(), Some(ErrorCode::NotAllowed));
    assert_eq!(watch_output(cx, vec![], &who.inside_first).err(), Some(ErrorCode::NotAllowed));
    assert_eq!(watch_output(cx, vec![2], &who.inside_first).err(), Some(ErrorCode::NotAllowed));
    let own = watch_output(cx, vec![1], &who.inside_first).expect("a terminal's own output");
    print(cx, &window, 0, b"one two\r");
    let told = output_until(cx, &own, 1, "one two");
    assert!(told.iter().any(|line| line == "one two"), "{told:?}");
    // With it, every terminal, and each event names its terminal.
    allow(cx, true, false);
    let all = watch_output(cx, vec![], &who.outside).expect("allowed");
    print(cx, &window, 1, b"from the second\r");
    let seen = {
        let mut lines = Vec::new();
        wait_until(cx, "the second terminal's output", |_| {
            while let Next::Event(published) = all.next(Duration::ZERO) {
                if let Body::Output { lines: told, .. } = &published.event.body {
                    assert_eq!((published.event.window, published.event.terminal), (Some(1), Some(2)));
                    lines.extend(told.iter().cloned());
                }
            }
            lines.iter().any(|line| line.contains("from the second"))
        });
        lines
    };
    assert!(!seen.is_empty());
    // The first terminal's caller heard nothing of the second.
    while let Next::Event(published) = own.next(Duration::ZERO) {
        assert_eq!(published.event.terminal, Some(1));
    }
    // The setting goes off: the stream that needed it ends, the terminal's own caller's does not.
    allow(cx, false, false);
    cx.update(|cx| crate::automation::sync_grants(cx, true));
    cx.update(|cx| crate::automation::sync_grants(cx, false));
    assert!(matches!(all.next(Duration::from_millis(200)), Next::Closed(CloseReason::NotAllowed)));
    print(cx, &window, 0, b"still mine\r");
    output_until(cx, &own, 1, "still mine");
}

#[gpui_kit::test]
fn a_full_screen_program_is_told_as_the_screen_changing(cx: &mut TestAppContext) {
    if isolate("a_full_screen_program_is_told_as_the_screen_changing") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| crate::automation::install(Bus::new(), cx));
    let who = callers(cx, &window);
    allow(cx, true, false);
    let stream = watch_output(cx, vec![], &who.outside).expect("allowed");
    print(cx, &window, 0, b"\x1b[?1049h\x1b[2Jvim is here\r");
    wait_until(cx, "the alternate screen", |cx| shown(cx, 1, &who).screen == "alt");
    let mut alt = false;
    wait_until(cx, "an event for the alternate screen", |_| {
        while let Next::Event(published) = stream.next(Duration::ZERO) {
            if let Body::Output { screen, lines, .. } = &published.event.body
                && screen == "alt"
            {
                assert!(lines.is_empty(), "the screen's text is `read`'s: {lines:?}");
                alt = true;
            }
        }
        alt
    });
}

/// Watching what a terminal prints costs the window nothing: the output reaches the bus's worker
/// without the workspace being notified, so nothing is drawn for it.
#[gpui_kit::test]
fn watching_output_does_not_wake_the_window(cx: &mut TestAppContext) {
    if isolate("watching_output_does_not_wake_the_window") {
        return;
    }
    use std::cell::Cell;
    use std::rc::Rc;
    let window = start(cx, 1);
    let first = terminal(cx, &window, 0);
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
    let without = notified.get();
    cx.update(|cx| crate::automation::install(Bus::new(), cx));
    allow(cx, true, false);
    let who = callers(cx, &window);
    let _stream = watch_output(cx, vec![], &who.outside).expect("allowed");
    round(cx);
    notified.set(0);
    round(cx);
    assert_eq!(
        notified.get(),
        without,
        "output events woke the workspace {} times more",
        notified.get().saturating_sub(without)
    );
}

/// A program that asked for the DEC locator instead of mouse reports gets the click as locator
/// reports, and a wheel turn (which the locator has no word for) is refused.
#[gpui_kit::test]
fn a_program_that_asked_for_the_locator_gets_the_click_as_locator_reports(cx: &mut TestAppContext) {
    if isolate("a_program_that_asked_for_the_locator_gets_the_click_as_locator_reports") {
        return;
    }
    let window = start(cx, 1);
    let who = callers(cx, &window);
    allow(cx, true, true);
    let mouse = |op: MouseOp| Action::Mouse(MouseSpec { target: Target::Terminal(1), op });
    // DECELR: enable the locator; DECSLE: report button down and button up.
    print(cx, &window, 0, b"\x1b[1'z\x1b[1;3'{\r");
    wait_until(cx, "the locator", |cx| shown(cx, 1, &who).modes.locator);
    let click = reply(ask(cx, mouse(MouseOp::Click(pointer(1, 2))), None, &who.outside));
    assert_eq!(code(&click), None, "{click:?}");
    // Down and up at row 2, column 3 (counted from 1): `CSI Pe ; Pb ; Pr ; Pc ; Pp & w`.
    show_contains(cx, 1, &who, "&w");
    let wheel = MouseOp::Scroll(Wheel {
        row: 0,
        col: 0,
        direction: Direction::Up,
        lines: 1,
        shift: false,
        alt: false,
        ctrl: false,
    });
    let refused = reply(ask(cx, mouse(wheel), None, &who.outside));
    assert_eq!(refused.err().and_then(|e| e.reason), Some(Reason::NoMouse));
}

/// A drag is a press, motion with the button held and a release: a program that tracks buttons (1002)
/// is told of the motion, which it would not be of a move with no button down.
#[gpui_kit::test]
fn a_drag_reaches_a_program_that_tracks_buttons(cx: &mut TestAppContext) {
    if isolate("a_drag_reaches_a_program_that_tracks_buttons") {
        return;
    }
    let window = start(cx, 1);
    let who = callers(cx, &window);
    allow(cx, true, true);
    let mouse = |op: MouseOp| Action::Mouse(MouseSpec { target: Target::Terminal(1), op });
    print(cx, &window, 0, b"\x1b[?1002h\x1b[?1006h\r");
    wait_until(cx, "button tracking", |cx| shown(cx, 1, &who).modes.mouse == "drag");
    let held = Pointer { button: Some(ButtonName::Left), ..pointer(1, 3) };
    for op in [
        MouseOp::Press(pointer(1, 2)),
        MouseOp::Move(Pointer { button: None, ..pointer(1, 5) }),
        MouseOp::Move(held),
        MouseOp::Release(pointer(1, 3)),
    ] {
        let sent = reply(ask(cx, mouse(op), None, &who.outside));
        assert!(matches!(code(&sent), None | Some(ErrorCode::Refused)), "{sent:?}");
    }
    // The move with no button is not told (the request is refused as nothing to send); the drag is.
    show_contains(cx, 1, &who, "^[[<0;3;2M^[[<32;4;2M^[[<0;4;2m");
}
