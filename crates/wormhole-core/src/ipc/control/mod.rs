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
mod limits;
pub mod sessions;
mod welcome;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) use cli::exchange;
pub use cli::{COMMAND_NAME, Cli, Parsed, help, is_verb, parse, render, run};
pub use limits::{Limited, RateLimits};
pub use welcome::{
    AgentFact, AgentsFacts, FIELD_MAX_BYTES, HooksFact, LookFacts, NotifyFacts, PermissionFact, PrimaryFact,
    ThemeFact, WelcomeOp, WelcomeState,
};

use crate::notify::State;

/// The version of this request and reply format. Bump it when a change would make an older
/// binary misread a newer app's answer (or the reverse); adding an optional field is not that.
pub const PROTOCOL_VERSION: u32 = 1;

/// The most text one `send-text` may type (a prompt, not a file).
pub const SEND_TEXT_MAX_BYTES: usize = 16 * 1024;

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

/// A terminal's state in the reply, spelled as the sidebar speaks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalState {
    Idle,
    Working,
    NeedsYou,
    Finished,
    /// A command ended with a non-zero exit code, or an agent or program stopped on an error, and
    /// nobody has looked yet. A binary built before this state existed can't read a reply that
    /// has it; the protocol version stays, since moving it would turn away every script that
    /// still runs the older one.
    Failed,
    Exited,
}

impl From<State> for TerminalState {
    fn from(state: State) -> Self {
        match state {
            State::Idle => TerminalState::Idle,
            State::Working => TerminalState::Working,
            State::NeedsYou => TerminalState::NeedsYou,
            State::Done => TerminalState::Finished,
            State::Failed => TerminalState::Failed,
            State::Exited => TerminalState::Exited,
        }
    }
}

impl TerminalState {
    pub fn word(self) -> &'static str {
        match self {
            TerminalState::Idle => "idle",
            TerminalState::Working => "working",
            TerminalState::NeedsYou => "needs you",
            TerminalState::Finished => "finished",
            TerminalState::Failed => "failed",
            TerminalState::Exited => "exited",
        }
    }
}

/// `list`'s result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: i64,
    /// The window in front (the one `new-terminal` opens in without `--window`).
    pub front: bool,
    pub terminals: Vec<TerminalInfo>,
}

/// A sidebar terminal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub id: i64,
    pub name: String,
    pub status: TerminalState,
    /// The coding agent in its lead tab, by id (`claude`, `codex`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Its selected tab's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Changed files in its folder's repository, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<usize>,
    pub pinned: bool,
    /// The profile it is in, named only while there is more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// On screen in its window.
    pub selected: bool,
    pub tabs: Vec<TabInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: i64,
    pub name: String,
    pub status: TerminalState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The agent's last words, when it needs you.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The program the tab runs ("zsh", "claude").
    pub shell: String,
    /// The program in its foreground, when one is ("npm run dev").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Seconds in its status.
    pub since_seconds: u64,
    pub selected: bool,
    pub panes: Vec<PaneInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: i64,
}

/// `status`'s result for one terminal or tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusInfo {
    pub window: i64,
    pub terminal: TerminalInfo,
    /// The tab the status is about: the one asked for, else the terminal's selected tab.
    pub tab: i64,
}

/// `status`'s result without a target, outside any tab: the app as a whole.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub windows: usize,
    pub terminals: usize,
    pub needs_you: usize,
    pub working: usize,
    pub finished: usize,
    /// Added with the failed state: absent in an older app's reply.
    #[serde(default)]
    pub failed: usize,
    pub idle: usize,
    pub exited: usize,
}

impl Summary {
    /// Count `terminals` (each by its lead tab's state) into a summary over `windows` windows.
    pub fn of(windows: usize, terminals: impl IntoIterator<Item = TerminalState>) -> Self {
        let mut summary = Summary { windows, ..Summary::default() };
        for state in terminals {
            summary.terminals += 1;
            match state {
                TerminalState::NeedsYou => summary.needs_you += 1,
                TerminalState::Working => summary.working += 1,
                TerminalState::Finished => summary.finished += 1,
                TerminalState::Failed => summary.failed += 1,
                TerminalState::Idle => summary.idle += 1,
                TerminalState::Exited => summary.exited += 1,
            }
        }
        summary
    }
}

/// `new-terminal`'s result: where the terminal is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opened {
    pub window: i64,
    pub terminal: i64,
    pub tab: i64,
    pub pane: i64,
    pub folder: Option<String>,
    /// The worktree's branch and folder, when one was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

/// `new-terminal --worktree`'s result: the worktree is never made without the user's say, so the
/// dialog that asks is up in `window`; the terminal opens once they confirm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asked {
    pub asked: bool,
    pub window: i64,
    pub branch: String,
    pub folder: String,
}

/// `notify`'s and `send-text`'s result: which tab it reached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reached {
    pub tab: i64,
}

/// What the app knows of a `send-text` target when it decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendTarget {
    pub tab: i64,
    pub state: TerminalState,
}

/// Whether `text` may go into `target` for `from`, with typing `enabled` in Settings. Order:
/// the switch first (so a caller learns about the setting before anything about a terminal),
/// then the size, then the terminal: never the caller's own (an agent answering its own prompt
/// from a script), never one waiting for the user's answer (a permission prompt is exactly what
/// a hostile script would want to type `y` into), never a dead shell.
pub fn send_text_check(
    enabled: bool,
    text: &str,
    from: Option<i64>,
    target: SendTarget,
) -> Result<(), Error> {
    let refuse = |code, message: String| Err(Error { code, message });
    if !enabled {
        return Err(writes_off());
    }
    if text.len() > SEND_TEXT_MAX_BYTES {
        return refuse(
            ErrorCode::TooLarge,
            format!("send-text takes at most {} bytes, not {}", SEND_TEXT_MAX_BYTES, text.len()),
        );
    }
    if text.trim().is_empty() {
        return refuse(ErrorCode::BadRequest, "nothing to send".to_string());
    }
    if from == Some(target.tab) {
        return refuse(
            ErrorCode::Refused,
            format!("tab {} is the one asking: a terminal can't type into itself", target.tab),
        );
    }
    match target.state {
        TerminalState::NeedsYou => refuse(
            ErrorCode::Refused,
            format!(
                "tab {} is waiting for the user's answer; nothing is typed into it until they have",
                target.tab
            ),
        ),
        TerminalState::Exited => refuse(ErrorCode::Refused, format!("tab {}'s shell has ended", target.tab)),
        TerminalState::Idle | TerminalState::Working | TerminalState::Finished | TerminalState::Failed => {
            Ok(())
        }
    }
}

/// The answer to `send-text` and `new-terminal` while `control_write` is off.
pub fn writes_off() -> Error {
    Error {
        code: ErrorCode::NotAllowed,
        message: "typing into terminals and opening them from the command line is off: turn on \
                  Settings → Coding agents → Let the command line type and open terminals"
            .into(),
    }
}

/// `notify`'s words as the banner shows them: cleaned like a program's own notification (one
/// line, no control or invisible characters, cut with an ellipsis), or nothing when nothing is
/// left to say.
pub fn notice_text(title: &str, body: &str) -> Option<wormhole_term::Notification> {
    use wormhole_term::{BODY_MAX_CHARS, TITLE_MAX_CHARS, clean};
    let (title, body) = (clean(title, TITLE_MAX_CHARS), clean(body, BODY_MAX_CHARS));
    (!title.is_empty() || !body.is_empty()).then_some(wormhole_term::Notification { title, body })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        Action, Error, ErrorCode, PROTOCOL_VERSION, Reply, Request, SEND_TEXT_MAX_BYTES, SendTarget, Summary,
        Target, TerminalState, WelcomeOp, notice_text, send_text_check,
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

    fn check(enabled: bool, text: &str, from: Option<i64>, state: TerminalState) -> Result<(), ErrorCode> {
        send_text_check(enabled, text, from, SendTarget { tab: 5, state }).map_err(|Error { code, .. }| code)
    }

    #[test]
    fn text_goes_only_where_the_rules_allow() {
        // The switch first: a caller learns about Settings before anything about the terminal.
        assert_eq!(check(false, "hi", None, TerminalState::Idle), Err(ErrorCode::NotAllowed));
        assert_eq!(check(false, "hi", Some(5), TerminalState::NeedsYou), Err(ErrorCode::NotAllowed));
        assert_eq!(check(true, "hi", None, TerminalState::Idle), Ok(()));
        assert_eq!(check(true, "hi", Some(2), TerminalState::Working), Ok(()), "another agent at work");
        assert_eq!(check(true, "hi", None, TerminalState::Finished), Ok(()));
        assert_eq!(check(true, "hi", None, TerminalState::Failed), Ok(()), "back at its prompt");
        // Never the caller's own tab.
        assert_eq!(check(true, "y", Some(5), TerminalState::Idle), Err(ErrorCode::Refused));
        // Never a terminal waiting for the user: that is the permission prompt.
        assert_eq!(check(true, "y", Some(2), TerminalState::NeedsYou), Err(ErrorCode::Refused));
        assert_eq!(check(true, "y", None, TerminalState::NeedsYou), Err(ErrorCode::Refused));
        assert_eq!(check(true, "ls", None, TerminalState::Exited), Err(ErrorCode::Refused));
        // Caps.
        assert_eq!(check(true, "", None, TerminalState::Idle), Err(ErrorCode::BadRequest));
        assert_eq!(check(true, "  \n", None, TerminalState::Idle), Err(ErrorCode::BadRequest));
        let huge = "x".repeat(SEND_TEXT_MAX_BYTES + 1);
        assert_eq!(check(true, &huge, None, TerminalState::Idle), Err(ErrorCode::TooLarge));
        assert_eq!(check(true, &huge[1..], None, TerminalState::Idle), Ok(()), "at the cap is fine");
    }

    #[test]
    fn a_notices_words_are_cleaned_like_a_programs() {
        let notice =
            notice_text(" Deploy\u{202e} ", "finished\x07 in  3\u{200b}s\n").expect("something to say");
        assert_eq!((notice.title.as_str(), notice.body.as_str()), ("Deploy", "finished in 3s"));
        assert!(notice_text("\x07", "  ").is_none(), "nothing left to say");
        let long = notice_text("", &"a".repeat(1000)).expect("body");
        assert!(long.body.ends_with('…') && long.body.chars().count() == crate::terminal::BODY_MAX_CHARS + 1);
    }

    #[test]
    fn errors_map_to_exit_codes_and_states_to_words() {
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
        assert_eq!(TerminalState::from(crate::notify::State::Done), TerminalState::Finished);
        assert_eq!(TerminalState::from(crate::notify::State::Failed), TerminalState::Failed);
        assert_eq!(TerminalState::NeedsYou.word(), "needs you");
        assert_eq!(TerminalState::Failed.word(), "failed");
        assert_eq!(serde_json::to_string(&TerminalState::Failed).expect("serializes"), r#""failed""#);
        assert_eq!(serde_json::to_string(&TerminalState::NeedsYou).expect("serializes"), r#""needs_you""#);
        assert_eq!((Target::Pane(4).id(), Target::Pane(4).kind()), (4, "pane"));
    }

    /// An app that has no failed count yet answers without it; the command line reads that as none.
    #[test]
    fn a_summary_from_an_older_app_has_no_failed_count() {
        let older =
            r#"{"windows":1,"terminals":2,"needs_you":0,"working":1,"finished":1,"idle":0,"exited":0}"#;
        let summary: Summary = serde_json::from_str(older).expect("reads");
        assert_eq!(summary.failed, 0);
        let counted = Summary::of(1, [TerminalState::Failed, TerminalState::Failed, TerminalState::Idle]);
        assert_eq!((counted.failed, counted.idle, counted.terminals), (2, 1, 3));
    }
}
