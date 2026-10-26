//! Reading a terminal and driving a program in it from the control line, in a real window whose
//! terminals run `/bin/cat`: its line discipline echoes what is typed (keys as `^[[A`), which shows
//! the bytes `send-keys` and `mouse` wrote, and a line sent back to it prints raw, which is how a test
//! makes the engine see a program's mode switches. The harness is `test_harness`.

use std::time::{Duration, Instant};

use gpui_kit::TestAppContext;
use wormhole_core::ipc::control::mouse::{ButtonName, Direction, Pointer, Wheel};
use wormhole_core::ipc::control::read::{Blocks, Range, Read};
use wormhole_core::ipc::control::{
    Action, BlocksSpec, ErrorCode, KeyItem, KeysSent, MouseOp, MouseSpec, ReadSpec, Reason, Reply, Request,
    SendKeysSpec, Target,
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
        Answer::Stream { .. } => panic!("a stream"),
    }
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
    Pointer { row, col, button: ButtonName::Left, count: 1, shift: false, alt: false, ctrl: false }
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
