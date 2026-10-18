//! One of every message, and every event kind, with fixed values. The golden vectors pin their
//! encoding, the property tests mutate them, and the fuzzer starts from them
//! (`examples/seeds.rs`). Fixtures, not API: a value here changes only together with a vector.

use crate::boot::{Hello, Incompatible, Welcome};
use crate::control::{Attach, ClipboardReply, Colors, ErrorMsg, Hook, Limits, Options, Resize, Spawn};
use crate::event::{Event, EventKind, Mark, MarkKind, ProgressState};
use crate::history::{FindDirection, FindMatch, FindResults, FindStart, Page, PageGet, find_flags};
use crate::message::Message;
use crate::stream::{Out, Repaint, RepaintKind, ResizeApplied};
use crate::types::{
    AppVersion, Counters, DetachReason, ExitInfo, Features, HostState, List, Role, Size, StreamMode,
};

const SIZE: Size = Size { cols: 120, rows: 40, cell_w: 134, cell_h: 272 };
const COUNTERS: Counters = Counters {
    epoch: 3,
    history_lines: 10_000,
    trimmed_total: 77,
    screen_rows: 40,
    alt_screen: false,
    revision: 9001,
};
const VERSION: AppVersion = AppVersion { major: 1, minor: 2, patch: 3 };

fn welcome() -> Welcome {
    Welcome {
        proto: 1,
        proto_minor: 0,
        state: HostState::Running,
        host_version: VERSION,
        pid: 4242,
        session_id: *b"abcdefghijkl",
        features: Features::HISTORY_PAGES.union(Features::FIND),
        started_at_ms: 1_760_000_000_000,
        first_block_id: 5,
        size: SIZE,
        counters: COUNTERS,
        exit: None,
        token: Some([7; 32]),
    }
}

/// The events, one per kind, as `(name, event)`.
pub fn events() -> Vec<(&'static str, Event<'static>)> {
    let at = |kind| Event { offset: 123_456, kind };
    vec![
        ("event_title", at(EventKind::Title("build: cargo test"))),
        ("event_reset_title", at(EventKind::ResetTitle)),
        ("event_bell", at(EventKind::Bell)),
        ("event_cwd", at(EventKind::Cwd("/Users/me/project"))),
        (
            "event_mark_command_end",
            at(EventKind::Mark(Mark {
                kind: MarkKind::CommandEnd,
                block: 12,
                exit_code: Some(-1),
                at_ms: 1_760_000_000_500,
            })),
        ),
        (
            "event_mark_prompt",
            at(EventKind::Mark(Mark { kind: MarkKind::PromptStart, block: 13, exit_code: None, at_ms: 0 })),
        ),
        ("event_notify", at(EventKind::Notify { source: 99, title: "Done", body: "all green" })),
        ("event_progress", at(EventKind::Progress { state: ProgressState::Set, percent: 42 })),
        ("event_program_status", at(EventKind::ProgramStatus("thinking"))),
        ("event_clipboard_store", at(EventKind::ClipboardStore { selection: b'c', data: b"aGk=" })),
        ("event_clipboard_load", at(EventKind::ClipboardLoad { ticket: 17, selection: b'c' })),
        ("event_window_op", at(EventKind::WindowOp { op: 8, a: 24, b: 80 })),
        ("event_exited", at(EventKind::Exited(ExitInfo { code: 0, signal: 9, at_ms: 1_760_000_001_000 }))),
        ("event_full_reset", at(EventKind::FullReset)),
        ("event_repaint_seeds", at(EventKind::RepaintSeeds { block: 12, link_seq: 3, image_handle: 5 })),
    ]
}

/// Every message once (some twice, where an option changes the layout).
pub fn all() -> Vec<(&'static str, Message<'static>)> {
    let mut v: Vec<(&'static str, Message<'static>)> = vec![
        (
            "hello_controller",
            Message::Hello(Hello {
                proto_min: 1,
                proto_max: 1,
                proto_minor: 0,
                role: Role::Controller,
                force: false,
                features: Features::HISTORY_PAGES.union(Features::FIND).union(Features::SYNC),
                client_version: VERSION,
                client_name: "wormhole-app",
                ticket: None,
            }),
        ),
        (
            "hello_admin_ticket",
            Message::Hello(Hello {
                proto_min: 1,
                proto_max: 3,
                proto_minor: 4,
                role: Role::Admin,
                force: true,
                features: Features::NONE,
                client_version: AppVersion::default(),
                client_name: "",
                ticket: Some(b"tkt"),
            }),
        ),
        ("welcome_running", Message::Welcome(welcome())),
        (
            "welcome_exited",
            Message::Welcome(Welcome {
                state: HostState::Exited,
                exit: Some(ExitInfo { code: 2, signal: 0, at_ms: 1_760_000_002_000 }),
                token: None,
                ..welcome()
            }),
        ),
        (
            "incompatible",
            Message::Incompatible(Incompatible { host_min: 4, host_max: 5, host_version: VERSION }),
        ),
        ("get_info", Message::GetInfo),
        ("info", Message::Info(welcome())),
        ("kill", Message::Kill),
        ("detach", Message::Detach),
        ("detached", Message::Detached(DetachReason::TakenOver)),
        ("ping", Message::Ping(99)),
        ("pong", Message::Pong { n: 99, pty_progress: 1_000_000 }),
        ("error", Message::Error(ErrorMsg { ref_type: 0x10, code: 3, text: "no such program" })),
        (
            "spawn",
            Message::Spawn(Spawn {
                size: SIZE,
                options: Options { scrollback_lines: 10_000, flags: 3 },
                program: "/bin/zsh",
                cwd: "/Users/me",
                args: List::Borrowed(&["-l", "-i"]),
                env: List::Borrowed(&[("TERM", "xterm-256color"), ("HOME", "/Users/me")]),
                colors: Some(Colors {
                    fg: [0xdd; 3],
                    bg: [0x11; 3],
                    cursor: Some([0xff, 0x80, 0]),
                    palette: List::Borrowed(&[[0, 0, 0], [0xcd, 0, 0]]),
                }),
            }),
        ),
        (
            "spawn_minimal",
            Message::Spawn(Spawn {
                size: SIZE,
                options: Options { scrollback_lines: 0, flags: 0 },
                program: "/bin/cat",
                cwd: "",
                args: List::Borrowed(&[]),
                env: List::Borrowed(&[]),
                colors: None,
            }),
        ),
        (
            "attach_raw",
            Message::Attach(Attach {
                stream: StreamMode::Raw,
                history_lines: 20_000,
                window: 1 << 20,
                resume_from: None,
            }),
        ),
        (
            "attach_resume",
            Message::Attach(Attach {
                stream: StreamMode::Sync,
                history_lines: 0,
                window: 4 << 20,
                resume_from: Some(1 << 40),
            }),
        ),
        ("resize", Message::Resize(Resize { req: 8, size: SIZE })),
        (
            "set_colors",
            Message::SetColors(Colors {
                fg: [1, 2, 3],
                bg: [4, 5, 6],
                cursor: None,
                palette: List::Borrowed(&[[9, 9, 9]]),
            }),
        ),
        ("set_limits", Message::SetLimits(Limits { history_bytes: 1 << 30, ring_bytes: 1 << 20 })),
        ("set_options", Message::SetOptions(Options { scrollback_lines: 2_000, flags: 1 })),
        ("close", Message::Close { grace_ms: 3000 }),
        ("clipboard_reply", Message::ClipboardReply(ClipboardReply { ticket: 17, data: b"hello" })),
        ("hook", Message::Hook(Hook { name: "needs_you", payload: br#"{"a":1}"# })),
        ("input", Message::Input(b"ls -la\r")),
        ("input_ack", Message::InputAck(7)),
        ("out", Message::Out(Out { offset: 4096, bytes: b"\x1b[31mred\x1b[0m\r\n" })),
        (
            "repaint_full",
            Message::Repaint(Repaint {
                epoch: 3,
                offset: 8192,
                kind: RepaintKind::Full,
                history_gap: false,
                first: true,
                last: true,
                bytes: b"\x1b[H\x1b[2Jscreen",
            }),
        ),
        (
            "repaint_rows_part",
            Message::Repaint(Repaint {
                epoch: 3,
                offset: 8192,
                kind: RepaintKind::Rows,
                history_gap: true,
                first: true,
                last: false,
                bytes: b"row",
            }),
        ),
        (
            "resize_applied",
            Message::ResizeApplied(ResizeApplied { offset: 9000, epoch: 4, req: 8, size: SIZE }),
        ),
        ("ack", Message::Ack(1 << 33)),
        ("counters", Message::Counters(COUNTERS)),
        (
            "backlog",
            Message::Backlog(List::Borrowed(&[
                Event { offset: 1, kind: EventKind::Bell },
                Event { offset: 2, kind: EventKind::Title("t") },
            ])),
        ),
        ("page_get", Message::PageGet(PageGet { epoch: 3, from: 1000, count: 200 })),
        ("page", Message::Page(Page { epoch: 3, from: 1000, count: 2, bytes: b"line one\r\nline two" })),
        (
            "find_start",
            Message::FindStart(FindStart {
                id: 5,
                flags: find_flags::REGEX | find_flags::CASE_SENSITIVE,
                direction: FindDirection::Backward,
                epoch: 3,
                anchor_line: 20_000,
                anchor_col: 4,
                pattern: "err(or)?",
            }),
        ),
        (
            "find_results",
            Message::FindResults(FindResults {
                id: 5,
                epoch: 3,
                more: true,
                matches: List::Borrowed(&[
                    FindMatch { line: 10, col: 2, cells: 5, text: "error" },
                    FindMatch { line: 11, col: 0, cells: 3, text: "err" },
                ]),
            }),
        ),
        ("find_cancel", Message::FindCancel { id: 5 }),
    ];
    v.extend(events().into_iter().map(|(name, e)| (name, Message::Event(e))));
    v
}
