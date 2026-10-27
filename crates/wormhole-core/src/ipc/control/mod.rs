//! The control protocol: what `Wormhole list`, `status`, `notify`, `send-text` and `new-terminal`
//! send over the hook socket, and what the running app answers.
//!
//! A request is one JSON line (`Message::Control`), the answer one JSON line back on the same
//! connection. Both carry [`PROTOCOL_VERSION`]: the binary a script runs may be older or newer than
//! the app it talks to (an agent's shell keeps the binary it started with), so a request from
//! another version is answered with [`ErrorCode::Version`] rather than half understood.
//!
//! Who may do what is decided here, in one place the app and the tests share: reading (`list`,
//! `status`) and telling (`notify`) always work; typing into a terminal (`send-text`, `send-keys`,
//! `mouse`) and starting a program (`new-terminal`) are off until the user turns `control_write` on
//! in Settings, and never go into a terminal that is waiting for the user's answer or into the
//! caller's own ([`send_text_check`]); reading what a terminal shows (`read`, `blocks`, the `output`
//! events) is off until `control_read`, except that a terminal can always read itself
//! ([`read_check`]). Everything a caller sends is hostile input: capped in size here, rate
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

pub(in crate::ipc) mod cli;
mod codes;
pub mod doctor;
pub mod event;
mod facts;
pub mod keys;
mod limits;
pub mod mouse;
mod policy;
pub mod printable;
pub mod read;
pub mod sessions;
pub mod skill_cli;
pub mod structure;
pub mod wait;
mod welcome;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use cli::{COMMAND_NAME, Cli, Parsed, help, help_json, help_with, is_verb, parse, render, render_with};
pub(in crate::ipc) use cli::{
    events, json_env, print, print_err, resolve_folder, run_parts, show_automation,
};
pub use event::{Event, EventFilter, EventsSpec, Subscribed, Topic, Topics};
pub use facts::{
    Asked, KeysSent, Listing, Opened, PaneInfo, Reached, SplitInfo, StatusInfo, Summary, TabInfo,
    TerminalInfo, TerminalState, WindowInfo,
};
pub use keys::{KeyItem, SendKeysSpec};
pub use limits::{Limited, RateLimits};
pub use mouse::{MouseOp, MouseSpec};
pub use policy::{
    SEND_TEXT_MAX_BYTES, SendTarget, mouse_check, notice_text, read_check, reads_off, run_check,
    send_keys_check, send_text_check, writes_off,
};
pub use read::{BlocksSpec, REPLY_MAX_BYTES, ReadSpec};
pub use structure::{
    CloseAsked, CloseSpec, FocusSpec, Focused, NewWindowSpec, RenameSpec, Renamed, Side, SplitMade,
    SplitSpec, WindowOpened,
};
pub use wait::{Met, Ran, RunPlan, RunSpec, WaitPlan, WaitSpec, Waited, matched_text};
pub use welcome::{
    AgentFact, AgentsFacts, FIELD_MAX_BYTES, HooksFact, ImportDone, LookFacts, NotifyFacts, PermissionFact,
    PrimaryFact, ShownDone, ThemeFact, WelcomeOp, WelcomeState,
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
    /// Watch what happens: the reply says the state as of a point in time, and events follow on the
    /// same connection until either side closes it.
    Events(event::EventsSpec),
    /// What a terminal shows: its screen, history or one of its command blocks, as text.
    Read(read::ReadSpec),
    /// A terminal's command blocks: each command with how it ended, and its output if asked.
    Blocks(read::BlocksSpec),
    /// Key presses and text typed into a terminal, encoded for the program's keyboard modes.
    SendKeys(keys::SendKeysSpec),
    /// Pointer events for a program that asked for the mouse.
    Mouse(mouse::MouseSpec),
    /// A window with one new terminal, opened behind the other apps' windows unless asked to come forward.
    NewWindow(structure::NewWindowSpec),
    /// A new terminal beside a terminal, in the same split.
    Split(structure::SplitSpec),
    /// Select a terminal and give it its window's keyboard.
    Focus(structure::FocusSpec),
    /// Name a terminal (or call it after what runs in it again).
    Rename(structure::RenameSpec),
    /// Ask the user whether to close a terminal; it never closes on the caller's say.
    Close(structure::CloseSpec),
    /// What `wh doctor` needs of the app: version, protocol, the flag and switches, and the caller's terminal.
    Doctor,
    /// Block until a condition holds in a terminal: a state, text on its screen, quiet, a command done.
    Wait(wait::WaitSpec),
    /// Type a command at a shell's prompt, wait for it to finish and tell how it ended.
    Run(wait::RunSpec),
    /// The first-run welcome program asking the app that runs it (`token` is the one in its
    /// terminal's environment): state, choices, and the end.
    Welcome { token: String, op: WelcomeOp },
}

impl Action {
    /// What every action is called in a request's `action` field, for [`unreadable`]: a test keeps
    /// it in step with the enum.
    const KNOWN: [&str; 19] = [
        "list",
        "doctor",
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
        "welcome",
    ];

    /// What the action is called on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Action::List => "list",
            Action::Doctor => "doctor",
            Action::Status { .. } => "status",
            Action::Notify { .. } => "notify",
            Action::SendText { .. } => "send-text",
            Action::NewTerminal { .. } => "new-terminal",
            Action::Events(_) => "events",
            Action::Read(_) => "read",
            Action::Blocks(_) => "blocks",
            Action::SendKeys(_) => "send-keys",
            Action::Mouse(_) => "mouse",
            Action::NewWindow(_) => "new-window",
            Action::Split(_) => "split",
            Action::Focus(_) => "focus",
            Action::Rename(_) => "rename",
            Action::Close(_) => "close",
            Action::Wait(_) => "wait",
            Action::Run(_) => "run",
            Action::Welcome { .. } => "welcome",
        }
    }

    /// Whether the action types into a terminal, starts a program or changes the windows and terminals:
    /// off until `control_write`. (`Welcome` is none of these; its token is its switch.)
    pub fn writes(&self) -> bool {
        matches!(
            self,
            Action::SendText { .. }
                | Action::NewTerminal { .. }
                | Action::SendKeys(_)
                | Action::Mouse(_)
                | Action::NewWindow(_)
                | Action::Split(_)
                | Action::Focus(_)
                | Action::Rename(_)
                | Action::Close(_)
                | Action::Run(_)
        )
    }

    /// Whether the action tells what a terminal shows: off until `control_read`, for a terminal
    /// other than the one the caller runs in. (`Events` does only with the `output` topic.)
    pub fn reads(&self) -> bool {
        matches!(self, Action::Read(_) | Action::Blocks(_))
    }
}

/// The answer to a line that says it is a control request but is not one this build can read, or
/// `None` when the line is something else. An action this build doesn't know is `unsupported` (a
/// newer command line asking an older app gets that at once, not after its read timeout); one it
/// knows with fields it can't read, or no action at all, is `bad_request`.
pub fn unreadable(line: &str) -> Option<Reply> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("kind").and_then(Value::as_str) != Some("control") {
        return None;
    }
    let Some(name) = value.get("action").and_then(Value::as_str) else {
        return Some(Reply::error(ErrorCode::BadRequest, "the request names no action"));
    };
    // The name is the caller's: only a short plain word of it is repeated back.
    let plain = name.len() <= 32 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    let name = if plain { name } else { "?" };
    Some(if Action::KNOWN.contains(&name) {
        Reply::error(ErrorCode::BadRequest, format!("the `{name}` request could not be read"))
    } else {
        Error::new(ErrorCode::Unsupported, format!("this Wormhole doesn't know the action `{name}`")).into()
    })
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
        Error::new(code, message).into()
    }

    /// The error, when there is one.
    pub fn err(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}

/// Why a request was not carried out: a code, words for the person reading, and sometimes a
/// finer reason a script can act on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    /// What exactly, within the code, when a script could tell more than the code does. Absent from
    /// an older app's reply, and ignored by an older command line: the code stays what it acts on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// The command block the error is about, when it is about one: a `run` that ran out of time names the
    /// block still running, so the caller can `wait --after` or `read --block` it later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<u32>,
    /// For `rate_limited` when the app knows: the whole seconds after which the same request may be made
    /// again. Absent from an older app's reply, and ignored by an older command line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_s: Option<u16>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), reason: None, block: None, retry_after_s: None }
    }

    /// The same error about command block `block`.
    #[must_use]
    pub fn about_block(mut self, block: i64) -> Self {
        // (A shell would have to mark four billion prompts first; the field is small to keep `Reply` so.)
        self.block = Some(u32::try_from(block).unwrap_or(u32::MAX));
        self
    }

    /// The same error with the seconds to wait before asking again.
    #[must_use]
    pub fn retry_after(mut self, seconds: u64) -> Self {
        // (A `u16`, an hour and more: it keeps `Reply` under the size clippy asks `Err`s to stay in; the window is a minute.)
        self.retry_after_s = Some(u16::try_from(seconds).unwrap_or(u16::MAX));
        self
    }

    /// The same error with the finer `reason` named.
    #[must_use]
    pub fn because(mut self, reason: Reason) -> Self {
        self.reason = Some(reason);
        self
    }
}

impl From<Error> for Reply {
    fn from(error: Error) -> Self {
        Self { version: PROTOCOL_VERSION, ok: false, result: None, error: Some(error) }
    }
}

/// The finer reasons behind an [`ErrorCode`], spelled as stable snake_case words. A new one is an
/// addition: [`Reason::Other`] is what a build that doesn't know it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// A wait or a command took longer than it was given.
    Timeout,
    /// The shell has no integration, so its commands can't be told apart.
    NoIntegration,
    /// The shell is not at its prompt.
    NotAtPrompt,
    /// The program in the terminal did not ask for the mouse.
    NoMouse,
    /// The window has no room for another pane.
    NoRoom,
    /// A close question is already up in that window.
    QuestionOpen,
    /// The terminal's shell has ended.
    Exited,
    /// The feature is still behind its flag in this build.
    FlagOff,
    /// A `run` is under way in that terminal: its command was typed and has not ended.
    RunPending,
    /// The terminal is in a window of a profile that is not shown: it is left alone, as if absent.
    HiddenProfile,
    /// A reason a newer Wormhole gave.
    #[serde(other)]
    Other,
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
    /// A code a newer Wormhole gave: read as a refusal, so an older command line still says what
    /// the words say and exits 1.
    #[serde(other)]
    Other,
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
mod tests;
