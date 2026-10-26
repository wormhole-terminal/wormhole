//! The control protocol: what `Wormhole list`, `status`, `notify`, `send-text` and `new-terminal`
//! send over the hook socket, and what the running app answers.
//!
//! A request is one JSON line (`Message::Control`), the answer one JSON line back on the same
//! connection. Both carry [`PROTOCOL_VERSION`]: the binary a script runs may be older or newer than
//! the app it talks to (an agent's shell keeps the binary it started with), so a request from
//! another version is answered with [`ErrorCode::Version`] rather than half understood.
//!
//! Who may do what is decided here, in one place the app and the tests share: reading (`list`,
//! `status`) and telling (`notify`) always work; typing into a terminal (`send-text`) and starting
//! a program (`new-terminal`) are off until the user turns `control_write` on in Settings, and
//! never go into a terminal that is waiting for the user's answer or into the caller's own
//! ([`send_text_check`]). Everything a caller sends is hostile input: capped in size here, rate
//! limited in `limits`, and shown only through text cleaned like a program's own notification.
//!
//! `sessions` (`Wormhole sessions list | end | forget`) is not part of the protocol: it is for the session
//! hosts that keep terminals running behind the app, works from their files and sockets, and needs no app
//! (`sessions`).
//!
//! One more kind of request is not a script's: `welcome` is the first-run program (`Wormhole welcome`,
//! in a terminal the app started for it) talking to the app that runs it. A token the app put in that
//! terminal's environment stands in for `control_write` (`welcome`'s module notes say why that is
//! enough, and what it allows).

mod cli;
mod facts;
mod limits;
mod policy;
pub mod sessions;
mod welcome;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use cli::{COMMAND_NAME, Cli, Parsed, help, is_verb, parse, render};
pub(in crate::ipc) use cli::{print, print_err};
pub use facts::{
    Asked, Listing, Opened, PaneInfo, Reached, StatusInfo, Summary, TabInfo, TerminalInfo, TerminalState,
    WindowInfo,
};
pub use limits::{Limited, RateLimits};
pub use policy::{SEND_TEXT_MAX_BYTES, SendTarget, notice_text, send_text_check, writes_off};
pub use welcome::{
    AgentFact, AgentsFacts, FIELD_MAX_BYTES, HooksFact, LookFacts, NotifyFacts, PermissionFact, PrimaryFact,
    ThemeFact, WelcomeOp, WelcomeState,
};

/// The version of this request and reply format. Bump it when a change would make an older
/// binary misread a newer app's answer (or the reverse); adding an optional field is not that.
pub const PROTOCOL_VERSION: u32 = 1;

/// What a caller asks the app to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    /// The tab the caller runs in (`WORMHOLE_TAB_ID`), when it runs in one: who asked, which
    /// the app logs, uses as the default terminal for `status` and `notify`, and never lets
    /// `send-text` type into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(flatten)]
    pub action: Action,
}

impl Request {
    pub fn new(action: Action, from: Option<String>) -> Self {
        Self { version: PROTOCOL_VERSION, from, action }
    }

    /// The caller's tab as an id, when it runs in one.
    pub fn from_tab(&self) -> Option<i64> {
        self.from.as_deref().and_then(|tab| tab.trim().parse().ok())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Every window, terminal, tab and pane, with ids.
    List,
    /// One terminal's status (the caller's own without a target), or the app's summary.
    Status {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<Target>,
    },
    /// A notification about a terminal, in the caller's words (the caller's own without a target).
    Notify {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<Target>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        title: String,
        body: String,
    },
    /// Type into a terminal, as a paste; `submit` presses Return after it.
    SendText {
        target: Target,
        text: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        submit: bool,
    },
    /// A new terminal, in `folder` (the window's current folder without one), running `command`
    /// in its shell if given, in `window` (the front one without), in a new git worktree on
    /// `worktree` if given.
    NewTerminal {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        worktree: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window: Option<i64>,
    },
    /// The first-run welcome program asking the app that runs it (`token` is the one in its
    /// terminal's environment): state, choices, and the end.
    Welcome { token: String, op: WelcomeOp },
}

impl Action {
    /// What the action is called on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Action::List => "list",
            Action::Status { .. } => "status",
            Action::Notify { .. } => "notify",
            Action::SendText { .. } => "send-text",
            Action::NewTerminal { .. } => "new-terminal",
            Action::Welcome { .. } => "welcome",
        }
    }

    /// Whether the action types into a terminal or starts a program: off until `control_write`.
    /// (`Welcome` neither types nor starts a program; its token is its switch.)
    pub fn writes(&self) -> bool {
        matches!(self, Action::SendText { .. } | Action::NewTerminal { .. })
    }
}

/// What a request is about, by id. A pane is a tab until split panes land: a pane id names
/// its tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Target {
    /// A sidebar terminal (a session): its selected tab.
    Terminal(i64),
    Tab(i64),
    Pane(i64),
}

impl Target {
    pub fn id(self) -> i64 {
        match self {
            Target::Terminal(id) | Target::Tab(id) | Target::Pane(id) => id,
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            Target::Terminal(_) => "terminal",
            Target::Tab(_) => "tab",
            Target::Pane(_) => "pane",
        }
    }
}

/// The app's answer: `result` on success, `error` otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub version: u32,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

impl Reply {
    pub fn ok(result: impl Serialize) -> Self {
        let result = serde_json::to_value(result).unwrap_or(Value::Null);
        Self { version: PROTOCOL_VERSION, ok: true, result: Some(result), error: None }
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        let error = Error { code, message: message.into() };
        Self { version: PROTOCOL_VERSION, ok: false, result: None, error: Some(error) }
    }

    /// The error, when there is one.
    pub fn err(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request's `version` is not this build's.
    Version,
    /// Malformed, or missing something.
    BadRequest,
    /// No such terminal, tab, pane, window or folder.
    NotFound,
    /// Typing and starting programs are off in Settings (`control_write`).
    NotAllowed,
    /// Allowed in general, but not into this terminal right now.
    Refused,
    /// Too many requests of this kind from this caller lately.
    RateLimited,
    /// Over a size cap.
    TooLarge,
    /// This build doesn't do that.
    Unsupported,
    /// The app tried and could not.
    Failed,
}

impl ErrorCode {
    /// The command line's exit code for it: 2 for a request the caller got wrong, 1 for one the
    /// app would not or could not carry out.
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorCode::BadRequest | ErrorCode::Version => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        Action, ErrorCode, PROTOCOL_VERSION, Reply, Request, Summary, Target, TerminalState, WelcomeOp,
    };

    /// Compiles only while every action is listed: a new one must get a golden line too.
    fn name(action: &Action) -> &'static str {
        match action {
            Action::List => "list",
            Action::Status { .. } => "status",
            Action::Notify { .. } => "notify",
            Action::SendText { .. } => "send_text",
            Action::NewTerminal { .. } => "new_terminal",
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
                    Action::Notify {
                        target: Some(Target::Tab(9)),
                        title: "Deploy".into(),
                        body: "Done".into(),
                    },
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
            BTreeSet::from(["list", "status", "notify", "send_text", "new_terminal", "welcome"])
        );
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
}
