//! The Program Status Protocol (OSC 7501, revision 0.4): a program tells the terminal what it is
//! doing (idle, working, blocked on the user, done, failed) and why, and the terminal keeps a
//! current picture of every program running in it. The protocol shares state and leaves how it is
//! shown to the terminal.
//!
//! - `parse` is the grammar and its limits: one OSC body in, a [`Report`] (or the feature-detection
//!   query) out, or nothing when the spec says to discard it. Everything is checked before the
//!   store is touched, so a discarded report changes nothing.
//! - `store` is [`ProgramStatus`]: the records by id, how they are replaced, cleared, evicted and
//!   dropped when a prompt or the program's exit says they are over, and the [`Summary`] the UI
//!   shows of them.
//!
//! Everything in a report is untrusted text from a program that already controls the screen. It
//! is validated here (UTF-8, no control characters, sizes) and cleaned again for display
//! ([`Summary::message`]); nothing a report holds is ever written back, and the one reply is
//! [`QUERY_REPLY`].

mod parse;
mod store;
#[cfg(test)]
mod tests;

pub use parse::{Parsed, SEQUENCE_MAX, parse};
pub use store::{ProgramStatus, RECORDS_MAX, Summary};

use crate::agents::{AGENTS, AgentId};

/// The terminal's answer to `OSC 7501 ; ? ST`. A plain `?`: an absent `states` or `kinds` list
/// means exactly the default ones, which are all supported, so there is nothing to list.
pub const QUERY_REPLY: &[u8] = b"\x1b]7501;?\x1b\\";

/// What a program says it is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    /// At rest, waiting for the user's next instruction.
    Idle,
    /// Running. May carry progress.
    Working,
    /// Finished a piece of work that the user has not seen yet.
    Done,
    /// Cannot continue until the user does something ([`Kind`] says what).
    Blocked,
    /// Failed and stopped.
    Error,
}

impl State {
    /// How much a state needs the user, for choosing which of several records to show: what needs
    /// them first (a question, then a failure, then a result to look at) outranks work that is
    /// merely going on, and that outranks being at rest.
    fn priority(self) -> u8 {
        match self {
            State::Blocked => 4,
            State::Error => 3,
            State::Done => 2,
            State::Working => 1,
            State::Idle => 0,
        }
    }

    /// Done and error outlive the program and its prompt; the terminal decides when the user has
    /// seen them. The rest is only true while the program runs.
    pub(crate) fn outlives_program(self) -> bool {
        matches!(self, State::Done | State::Error)
    }
}

/// What a blocked program waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Approval to do something.
    Permission,
    /// An answer the user must type.
    Question,
    /// A login, token or credential.
    Auth,
}

/// What the terminal stores for one id: exactly the keys of the last report that used it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub state: State,
    /// Only with [`State::Blocked`].
    pub kind: Option<Kind>,
    /// 0 to 100; only with working or blocked. Absent means indeterminate.
    pub progress: Option<u8>,
    /// The program's own identifier ("cargo", "claude-code"). Read through
    /// [`ProgramStatus::summary`], which inherits it from the nearest ancestor.
    pub app: Option<String>,
    /// A short label (decoded, validated, not yet cleaned for display).
    pub title: Option<String>,
    /// One line on what it does, waits for or finished (decoded, validated, not yet cleaned).
    pub msg: Option<String>,
}

/// One accepted report. Ids are paths (`build/test` is a child of `build`; no id is the root).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Report {
    /// `state=clear`: removes the record at `id` and every record beneath it; with no id, every
    /// record there is.
    Clear { id: Option<String> },
    /// Any other state: the record at `id` becomes exactly this.
    Set { id: Option<String>, record: Record },
}

/// The coding agent a report's `app` names, where that is obvious: an agent's command or
/// executable name (the registry's, so `claude`, `codex`, `cursor-agent`, …) or one of the few
/// names its products go by. `app` is the program's own claim, so this only picks an icon and the
/// wording; nothing trusts it.
pub fn agent_for_app(app: &str) -> Option<AgentId> {
    /// Names that are not the command.
    const ALIASES: &[(&str, AgentId)] = &[
        ("claude-code", AgentId::Claude),
        ("codex-cli", AgentId::Codex),
        ("gemini-cli", AgentId::Gemini),
        ("github-copilot", AgentId::Copilot),
        ("cursor", AgentId::Cursor),
        ("qwen-code", AgentId::Qwen),
        ("kiro", AgentId::Kiro),
        ("cline-cli", AgentId::Cline),
        ("pi-coding-agent", AgentId::Pi),
    ];
    let app = app.to_ascii_lowercase();
    AGENTS
        .iter()
        .find(|agent| agent.command == app || agent.executables.contains(&app.as_str()))
        .map(|agent| agent.id)
        .or_else(|| ALIASES.iter().find(|(name, _)| *name == app).map(|(_, id)| *id))
}
