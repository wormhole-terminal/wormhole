use std::collections::BTreeSet;

use super::keys::{KeyItem, SendKeysSpec};
use super::mouse::{ButtonName, MouseOp, MouseSpec, Pointer};
use super::read::{BlocksSpec, Range, ReadSpec};
use super::structure::{CloseSpec, FocusSpec, NewWindowSpec, RenameSpec, Side, SplitSpec};
use super::wait::{RunSpec, WaitSpec};
use super::{
    Action, Error, ErrorCode, PROTOCOL_VERSION, Reason, Reply, Request, Summary, Target, TerminalState,
    WelcomeOp, event::EventsSpec, unreadable,
};

/// Compiles only while every action is listed: a new one must get a golden line too.
fn name(action: &Action) -> &'static str {
    match action {
        Action::List => "list",
        Action::Status { .. } => "status",
        Action::Notify { .. } => "notify",
        Action::SendText { .. } => "send_text",
        Action::NewTerminal { .. } => "new_terminal",
        Action::Events(_) => "events",
        Action::Read(_) => "read",
        Action::Blocks(_) => "blocks",
        Action::SendKeys(_) => "send_keys",
        Action::Mouse(_) => "mouse",
        Action::NewWindow(_) => "new_window",
        Action::Split(_) => "split",
        Action::Focus(_) => "focus",
        Action::Rename(_) => "rename",
        Action::Close(_) => "close",
        Action::Wait(_) => "wait",
        Action::Run(_) => "run",
        Action::Welcome { .. } => "welcome",
    }
}

/// Every action as it goes over the wire. A script's binary may be older or newer than the
/// app: a change here is a compatibility change (bump [`PROTOCOL_VERSION`] on purpose).
fn golden() -> Vec<(Request, &'static str)> {
    vec![
        (Request::new(Action::List, Some("7".into())), r#"{"version":1,"from":"7","action":"list"}"#),
        (Request::new(Action::Status { target: None }, None), r#"{"version":1,"action":"status"}"#),
        (
            Request::new(Action::Status { target: Some(Target::Terminal(3)) }, None),
            r#"{"version":1,"action":"status","target":{"kind":"terminal","id":3}}"#,
        ),
        (
            Request::new(
                Action::Notify { target: Some(Target::Tab(9)), title: "Deploy".into(), body: "Done".into() },
                Some("9".into()),
            ),
            r#"{"version":1,"from":"9","action":"notify","target":{"kind":"tab","id":9},"title":"Deploy","body":"Done"}"#,
        ),
        (
            Request::new(
                Action::SendText { target: Target::Pane(4), text: "run the tests".into(), submit: true },
                Some("2".into()),
            ),
            r#"{"version":1,"from":"2","action":"send_text","target":{"kind":"pane","id":4},"text":"run the tests","submit":true}"#,
        ),
        (
            Request::new(
                Action::NewTerminal {
                    folder: Some("/work/app".into()),
                    command: Some("claude".into()),
                    worktree: Some("fix/login".into()),
                    window: Some(1),
                },
                None,
            ),
            r#"{"version":1,"action":"new_terminal","folder":"/work/app","command":"claude","worktree":"fix/login","window":1}"#,
        ),
        (
            Request::new(
                Action::Events(EventsSpec {
                    topics: vec!["status".into(), "blocks".into()],
                    terminals: vec![3],
                    windows: vec![],
                    output: false,
                }),
                Some("7".into()),
            ),
            r#"{"version":1,"from":"7","action":"events","topics":["status","blocks"],"terminals":[3],"output":false}"#,
        ),
        (
            Request::new(
                Action::Read(ReadSpec {
                    target: Some(Target::Terminal(3)),
                    range: Range::Lines { from: -200, to: 39 },
                    join: true,
                    styles: false,
                }),
                None,
            ),
            r#"{"version":1,"action":"read","target":{"kind":"terminal","id":3},"range":{"kind":"lines","from":-200,"to":39},"join":true}"#,
        ),
        (
            Request::new(
                Action::Read(ReadSpec { target: None, range: Range::Screen, join: false, styles: true }),
                Some("7".into()),
            ),
            r#"{"version":1,"from":"7","action":"read","range":{"kind":"screen"},"styles":true}"#,
        ),
        (
            Request::new(
                Action::Blocks(BlocksSpec {
                    target: Some(Target::Terminal(3)),
                    last: Some(10),
                    output: true,
                }),
                None,
            ),
            r#"{"version":1,"action":"blocks","target":{"kind":"terminal","id":3},"last":10,"output":true}"#,
        ),
        (
            Request::new(
                Action::SendKeys(SendKeysSpec {
                    target: Target::Terminal(4),
                    keys: vec![
                        KeyItem::Key { key: "ctrl-c".into() },
                        KeyItem::Text { text: "ls -la".into(), paste: false },
                        KeyItem::Key { key: "enter".into() },
                    ],
                }),
                Some("2".into()),
            ),
            r#"{"version":1,"from":"2","action":"send_keys","target":{"kind":"terminal","id":4},"keys":[{"key":"ctrl-c"},{"text":"ls -la"},{"key":"enter"}]}"#,
        ),
        (
            Request::new(
                Action::Mouse(MouseSpec {
                    target: Target::Terminal(3),
                    op: MouseOp::Click(Pointer {
                        row: 4,
                        col: 10,
                        button: Some(ButtonName::Left),
                        count: 1,
                        shift: false,
                        alt: false,
                        ctrl: false,
                    }),
                }),
                None,
            ),
            r#"{"version":1,"action":"mouse","target":{"kind":"terminal","id":3},"op":{"kind":"click","row":4,"col":10,"button":"left","count":1}}"#,
        ),
        (
            Request::new(
                Action::NewWindow(NewWindowSpec {
                    folder: Some("/work/app".into()),
                    command: Some("claude".into()),
                    activate: false,
                }),
                None,
            ),
            r#"{"version":1,"action":"new_window","folder":"/work/app","command":"claude"}"#,
        ),
        (
            Request::new(Action::NewWindow(NewWindowSpec { activate: true, ..Default::default() }), None),
            r#"{"version":1,"action":"new_window","activate":true}"#,
        ),
        (
            Request::new(
                Action::Split(SplitSpec {
                    target: Target::Terminal(3),
                    direction: Side::Right,
                    size: Some(40),
                    folder: None,
                    command: Some("npm run dev".into()),
                    select: false,
                }),
                Some("7".into()),
            ),
            r#"{"version":1,"from":"7","action":"split","target":{"kind":"terminal","id":3},"direction":"right","size":40,"command":"npm run dev"}"#,
        ),
        (
            Request::new(Action::Focus(FocusSpec { target: Target::Terminal(3), activate: true }), None),
            r#"{"version":1,"action":"focus","target":{"kind":"terminal","id":3},"activate":true}"#,
        ),
        (
            Request::new(
                Action::Rename(RenameSpec { target: Target::Terminal(3), name: Some("api".into()) }),
                None,
            ),
            r#"{"version":1,"action":"rename","target":{"kind":"terminal","id":3},"name":"api"}"#,
        ),
        (
            Request::new(Action::Rename(RenameSpec { target: Target::Terminal(3), name: None }), None),
            r#"{"version":1,"action":"rename","target":{"kind":"terminal","id":3}}"#,
        ),
        (
            Request::new(Action::Close(CloseSpec { target: Target::Terminal(3) }), None),
            r#"{"version":1,"action":"close","target":{"kind":"terminal","id":3}}"#,
        ),
        (
            Request::new(
                Action::Wait(WaitSpec {
                    target: Target::Terminal(3),
                    status: vec![TerminalState::Idle, TerminalState::NeedsYou],
                    text: Some("\\$ $".into()),
                    gone: None,
                    stable_ms: Some(800),
                    block_done: true,
                    timeout_ms: Some(30000),
                }),
                Some("7".into()),
            ),
            r#"{"version":1,"from":"7","action":"wait","target":{"kind":"terminal","id":3},"status":["idle","needs_you"],"text":"\\$ $","stable_ms":800,"block_done":true,"timeout_ms":30000}"#,
        ),
        (
            Request::new(
                Action::Wait(WaitSpec {
                    target: Target::Terminal(3),
                    status: vec![],
                    text: None,
                    gone: Some("Loading".into()),
                    stable_ms: None,
                    block_done: false,
                    timeout_ms: None,
                }),
                None,
            ),
            r#"{"version":1,"action":"wait","target":{"kind":"terminal","id":3},"gone":"Loading"}"#,
        ),
        (
            Request::new(
                Action::Run(RunSpec {
                    target: Target::Terminal(3),
                    command: "cargo test".into(),
                    timeout_ms: Some(600000),
                }),
                Some("7".into()),
            ),
            r#"{"version":1,"from":"7","action":"run","target":{"kind":"terminal","id":3},"command":"cargo test","timeout_ms":600000}"#,
        ),
        (
            Request::new(
                Action::Welcome { token: "0123abcd".into(), op: WelcomeOp::State },
                Some("4".into()),
            ),
            r#"{"version":1,"from":"4","action":"welcome","token":"0123abcd","op":{"kind":"state"}}"#,
        ),
    ]
}

#[test]
fn every_action_has_a_golden_line() {
    let covered: BTreeSet<_> = golden().iter().map(|(request, _)| name(&request.action)).collect();
    assert_eq!(
        covered,
        BTreeSet::from([
            "list",
            "status",
            "notify",
            "send_text",
            "new_terminal",
            "events",
            "read",
            "blocks",
            "send_keys",
            "mouse",
            "new_window",
            "split",
            "focus",
            "rename",
            "close",
            "wait",
            "run",
            "welcome"
        ])
    );
}

#[test]
fn the_known_actions_are_the_ones_with_golden_lines() {
    let covered: BTreeSet<_> = golden().iter().map(|(request, _)| name(&request.action)).collect();
    assert_eq!(covered, Action::KNOWN.into_iter().collect::<BTreeSet<_>>());
}

#[test]
fn requests_serialize_to_the_golden_lines_and_back() {
    for (request, line) in golden() {
        assert_eq!(serde_json::to_string(&request).expect("serializes"), line);
        let parsed: Request = serde_json::from_str(line).expect("parses");
        assert_eq!(parsed, request);
    }
}

#[test]
fn replies_carry_a_result_or_an_error() {
    let ok = Reply::ok(Summary::of(1, [TerminalState::Working, TerminalState::NeedsYou]));
    assert_eq!(
        serde_json::to_string(&ok).expect("serializes"),
        r#"{"version":1,"ok":true,"result":{"windows":1,"terminals":2,"needs_you":1,"working":1,"finished":0,"failed":0,"idle":0,"exited":0}}"#
    );
    let error = Reply::error(ErrorCode::NotFound, "no tab 9");
    let line = serde_json::to_string(&error).expect("serializes");
    assert_eq!(line, r#"{"version":1,"ok":false,"error":{"code":"not_found","message":"no tab 9"}}"#);
    let back: Reply = serde_json::from_str(&line).expect("parses");
    assert_eq!(back.err().map(|e| e.code), Some(ErrorCode::NotFound));
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn a_requests_tab_is_an_id_or_nothing() {
    assert_eq!(Request::new(Action::List, Some(" 12 ".into())).from_tab(), Some(12));
    assert_eq!(Request::new(Action::List, Some("spoofed".into())).from_tab(), None);
    assert_eq!(Request::new(Action::List, None).from_tab(), None);
}

#[test]
fn only_typing_and_starting_programs_need_the_switch() {
    let keys = Action::SendKeys(SendKeysSpec { target: Target::Tab(1), keys: vec![] });
    let mouse = Action::Mouse(MouseSpec {
        target: Target::Tab(1),
        op: MouseOp::Press(Pointer {
            row: 0,
            col: 0,
            button: Some(ButtonName::Left),
            count: 1,
            shift: false,
            alt: false,
            ctrl: false,
        }),
    });
    assert!(keys.writes() && mouse.writes() && !keys.reads() && !mouse.reads());
    // `run` types a command; `wait` only waits (and reads text, which is `control_read`'s, decided per condition).
    let run = Action::Run(RunSpec { target: Target::Tab(1), command: "ls".into(), timeout_ms: None });
    assert!(run.writes() && !run.reads());
    let wait = Action::Wait(WaitSpec {
        target: Target::Tab(1),
        status: vec![],
        text: None,
        gone: None,
        stable_ms: None,
        block_done: true,
        timeout_ms: None,
    });
    assert!(!wait.writes() && !wait.reads());
    // Windows, splits, the selection, names and the close question are the switch's too.
    for action in [
        Action::NewWindow(NewWindowSpec::default()),
        Action::Split(SplitSpec {
            target: Target::Tab(1),
            direction: Side::Down,
            size: None,
            folder: None,
            command: None,
            select: false,
        }),
        Action::Focus(FocusSpec { target: Target::Tab(1), activate: false }),
        Action::Rename(RenameSpec { target: Target::Tab(1), name: None }),
        Action::Close(CloseSpec { target: Target::Tab(1) }),
    ] {
        assert!(action.writes() && !action.reads(), "{}", action.name());
    }
    let read = Action::Read(ReadSpec { target: None, range: Range::Screen, join: false, styles: false });
    assert!(read.reads() && !read.writes(), "reading is control_read's, not control_write's");
    assert!(Action::Blocks(BlocksSpec { target: None, last: None, output: false }).reads());
    assert!(!Action::List.reads() && !Action::Events(EventsSpec::default()).reads());
    assert!(!Action::List.writes());
    assert!(!Action::Status { target: None }.writes());
    assert!(!Action::Notify { target: None, title: String::new(), body: "x".into() }.writes());
    assert!(Action::SendText { target: Target::Tab(1), text: "x".into(), submit: false }.writes());
    assert!(Action::NewTerminal { folder: None, command: None, worktree: None, window: None }.writes());
    // The welcome's token is its switch, not Settings.
    let welcome = Action::Welcome { token: "t".into(), op: WelcomeOp::Finish { skipped: false } };
    assert!(!welcome.writes());
    assert_eq!(welcome.name(), "welcome");
}

#[test]
fn errors_map_to_exit_codes() {
    assert_eq!(ErrorCode::BadRequest.exit_code(), 2);
    assert_eq!(ErrorCode::Version.exit_code(), 2);
    for code in [
        ErrorCode::NotFound,
        ErrorCode::NotAllowed,
        ErrorCode::Refused,
        ErrorCode::RateLimited,
        ErrorCode::TooLarge,
        ErrorCode::Unsupported,
        ErrorCode::Failed,
    ] {
        assert_eq!(code.exit_code(), 1, "{code:?}");
    }
    assert_eq!((Target::Pane(4).id(), Target::Pane(4).kind()), (4, "pane"));
}

#[test]
fn a_reason_rides_beside_the_code_and_an_older_reader_ignores_it() {
    let error = Reply::from(Error::new(ErrorCode::Failed, "took too long").because(Reason::Timeout));
    let line = serde_json::to_string(&error).expect("serializes");
    assert_eq!(
        line,
        r#"{"version":1,"ok":false,"error":{"code":"failed","message":"took too long","reason":"timeout"}}"#
    );
    assert_eq!(serde_json::from_str::<Reply>(&line).expect("parses"), error);
    // Without one the line is what it always was.
    let plain = serde_json::to_string(&Reply::error(ErrorCode::Failed, "x")).expect("serializes");
    assert!(!plain.contains("reason"), "{plain}");
}

/// A newer app's reply may carry a code, a reason or a state this build has never heard of;
/// it still reads, as an unknown one, so the command line prints the words instead of failing.
#[test]
fn a_reply_from_a_newer_app_still_parses() {
    let line = r#"{"version":1,"ok":false,"error":{"code":"quota","message":"nope","reason":"later"}}"#;
    let reply: Reply = serde_json::from_str(line).expect("parses");
    let error = reply.err().expect("an error");
    assert_eq!((error.code, error.reason), (ErrorCode::Other, Some(Reason::Other)));
    assert_eq!(error.code.exit_code(), 1);
    let state: TerminalState = serde_json::from_str(r#""sleeping""#).expect("parses");
    assert_eq!(state, TerminalState::Other);
    assert_eq!(state.word(), "unknown");
}

#[test]
fn an_action_this_build_does_not_know_is_answered_unsupported() {
    let reply = unreadable(
        r#"{"kind":"control","version":1,"action":"teleport","target":{"kind":"terminal","id":3}}"#,
    )
    .expect("answered");
    let error = reply.err().expect("an error");
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(error.message.contains("`teleport`"), "{}", error.message);
    assert!(
        serde_json::from_str::<crate::ipc::Message>(r#"{"kind":"control","version":1,"action":"teleport"}"#)
            .is_err()
    );
}

#[test]
fn a_control_line_with_something_else_wrong_is_a_bad_request() {
    let code = |line: &str| unreadable(line).and_then(|reply| reply.error).map(|error| error.code);
    // A known action whose fields don't fit.
    assert_eq!(code(r#"{"kind":"control","version":1,"action":"send_text"}"#), Some(ErrorCode::BadRequest));
    // A welcome op this build doesn't know (a newer program asking an older app, as `import` was
    // to the apps before it): the action is known, so it is a bad request the program shows as a
    // note, not an unsupported action and not a hang.
    let newer = r#"{"kind":"control","version":1,"from":"4","action":"welcome","token":"t","op":{"kind":"import_everything"}}"#;
    assert_eq!(code(newer), Some(ErrorCode::BadRequest));
    assert!(serde_json::from_str::<crate::ipc::Message>(newer).is_err());
    assert_eq!(code(r#"{"kind":"control","version":1}"#), Some(ErrorCode::BadRequest));
    assert_eq!(code(r#"{"kind":"control","version":1,"action":7}"#), Some(ErrorCode::BadRequest));
    // Not a control line, or not a line at all: nothing to say.
    assert_eq!(code(r#"{"kind":"hook","action":"read"}"#), None);
    assert_eq!(code("{not json"), None);
}

/// The caller's words are repeated back only when they are a plain word.
#[test]
fn a_hostile_action_name_is_not_echoed() {
    let line = format!(r#"{{"kind":"control","version":1,"action":"{}\u001b[31m"}}"#, "x".repeat(40));
    let reply = unreadable(&line).expect("answered");
    let message = reply.err().expect("an error").message.clone();
    assert!(message.contains("`?`") && !message.contains('\u{1b}') && !message.contains('x'), "{message}");
}
