//! The error codes and reasons as data: every one with its name on the wire, the command line's exit
//! code and what a caller should do about it. `wh help --json` prints these, and the skill that
//! `wh skill` installs is checked against them, so the words here are the only place that says it.
//!
//! The lists are written out, and a match without a catch-all keeps each in step with its enum: a new
//! variant does not compile until it has an answer here, and a test checks that the list holds it.

use super::{ErrorCode, Reason};

impl ErrorCode {
    /// Every code this build gives, in the order the docs list them (not [`ErrorCode::Other`], which is
    /// only what an older build reads a newer one's as).
    pub const ALL: [ErrorCode; 9] = [
        ErrorCode::Version,
        ErrorCode::BadRequest,
        ErrorCode::NotFound,
        ErrorCode::NotAllowed,
        ErrorCode::Refused,
        ErrorCode::RateLimited,
        ErrorCode::TooLarge,
        ErrorCode::Unsupported,
        ErrorCode::Failed,
    ];

    /// The code as it is spelled in a reply's `error.code`.
    pub fn word(self) -> &'static str {
        match self {
            ErrorCode::Version => "version",
            ErrorCode::BadRequest => "bad_request",
            ErrorCode::NotFound => "not_found",
            ErrorCode::NotAllowed => "not_allowed",
            ErrorCode::Refused => "refused",
            ErrorCode::RateLimited => "rate_limited",
            ErrorCode::TooLarge => "too_large",
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::Failed => "failed",
            ErrorCode::Other => "other",
        }
    }

    /// What it means and what to do about it, for the caller that reads it.
    pub fn advice(self) -> &'static str {
        match self {
            ErrorCode::Version => {
                "The command line and the running Wormhole speak different protocol versions. Tell the user to update Wormhole (or restart it); do not retry."
            }
            ErrorCode::BadRequest => {
                "The arguments are wrong (a missing target, a bad range, a pattern that does not compile). The message says what; fix the call."
            }
            ErrorCode::NotFound => {
                "No such terminal, window or folder, or the terminal's shell has ended (reason exited). Read the ids again with `wh list --json`."
            }
            ErrorCode::NotAllowed => {
                "The user has not allowed this: agent access is off, a switch is off, or they answered Don't Allow (the reason says which). The message names it. Ask the user; never try to get around it."
            }
            ErrorCode::Refused => {
                "Allowed in general, not into this terminal now: your own terminal, one waiting for the user's answer, a shell that is not at its prompt, no room for a split. The reason says which; do not retry unchanged."
            }
            ErrorCode::RateLimited => {
                "Too many requests of this kind lately. `error.retry_after_s` says how many seconds to wait (the message says the same); wait that long, then ask less often."
            }
            ErrorCode::TooLarge => "Over a size cap (text, keys, lines). Send less.",
            ErrorCode::Unsupported => {
                "This Wormhole cannot do that now: the Automation flag is off (reason flag_off) or the shell has no integration (reason no_integration)."
            }
            ErrorCode::Failed => {
                "The app tried and could not, or a wait ran out (reason timeout, exit code 124). Read the message; a timeout can be waited for again."
            }
            ErrorCode::Other => "A code a newer Wormhole gave; read the message.",
        }
    }
}

impl Reason {
    /// Every reason this build gives (not [`Reason::Other`]).
    pub const ALL: [Reason; 18] = [
        Reason::Timeout,
        Reason::NoIntegration,
        Reason::NotAtPrompt,
        Reason::NoMouse,
        Reason::NoRoom,
        Reason::QuestionOpen,
        Reason::Exited,
        Reason::FlagOff,
        Reason::RunPending,
        Reason::HiddenProfile,
        Reason::ApprovalNeeded,
        Reason::Declined,
        Reason::DeclinedRecently,
        Reason::AskTimedOut,
        Reason::AgentAccessOff,
        Reason::IdentityChanged,
        Reason::TooManyQuestions,
        Reason::NotAnswered,
    ];

    /// The reason as it is spelled in a reply's `error.reason`.
    pub fn word(self) -> &'static str {
        match self {
            Reason::Timeout => "timeout",
            Reason::NoIntegration => "no_integration",
            Reason::NotAtPrompt => "not_at_prompt",
            Reason::NoMouse => "no_mouse",
            Reason::NoRoom => "no_room",
            Reason::QuestionOpen => "question_open",
            Reason::Exited => "exited",
            Reason::FlagOff => "flag_off",
            Reason::RunPending => "run_pending",
            Reason::HiddenProfile => "hidden_profile",
            Reason::ApprovalNeeded => "approval_needed",
            Reason::Declined => "declined",
            Reason::DeclinedRecently => "declined_recently",
            Reason::AskTimedOut => "ask_timed_out",
            Reason::AgentAccessOff => "agent_access_off",
            Reason::IdentityChanged => "identity_changed",
            Reason::TooManyQuestions => "too_many_questions",
            Reason::NotAnswered => "not_answered",
            Reason::Other => "other",
        }
    }

    /// What it means and what to do about it.
    pub fn advice(self) -> &'static str {
        match self {
            Reason::Timeout => {
                "A wait or a run took longer than it was given (exit 124). The command, if any, keeps running: follow it with `wait --block-done --after <block>` or `read --block`."
            }
            Reason::NoIntegration => {
                "The shell reports no command blocks (OSC 133), so `run` and `wait --block-done` cannot tell commands apart. Use `send-keys` or `send-text` and `wait --match` or `--stable` instead."
            }
            Reason::NotAtPrompt => {
                "The shell is busy or a program has the terminal. Wait for it (`wait --block-done`, `--status idle`) or interrupt it with `send-keys ctrl-c` if that is what the user wants."
            }
            Reason::NoMouse => "The program in the terminal did not ask for the mouse; use keys.",
            Reason::NoRoom => "The window has no room for another pane; open a new window instead.",
            Reason::QuestionOpen => "A close question is already up in that window; do not ask again.",
            Reason::Exited => "The terminal's shell has ended; nothing more will happen in it.",
            Reason::FlagOff => {
                "The Automation flag is off in this Wormhole. Ask the user to turn it on (Settings → Feature flags); `wh doctor` says how."
            }
            Reason::RunPending => {
                "A run is under way in that terminal; wait for it to end before running another."
            }
            Reason::HiddenProfile => {
                "The terminal is in a window of a profile that is not shown; leave it alone."
            }
            Reason::ApprovalNeeded => {
                "The user has not let this program control Wormhole, and Wormhole has asked them (or will, in its window). Do not retry in a loop: tell the user to answer the question in Wormhole, then try once more."
            }
            Reason::Declined => {
                "The user chose Don't Allow. Do not retry and do not look for another way in; tell the user what you wanted to do and ask them to allow it in Settings → Coding agents → Agent access."
            }
            Reason::DeclinedRecently => {
                "The user said no a moment ago, so Wormhole does not ask again for two minutes. Do not retry; ask the user."
            }
            Reason::AskTimedOut => {
                "Nobody answered the question in Wormhole within a minute. Tell the user it is waiting, and try once more only after they say they have looked."
            }
            Reason::AgentAccessOff => {
                "Agent access is switched off in Settings → Coding agents → Agent access. Ask the user to turn it on; never try to get around it."
            }
            Reason::IdentityChanged => {
                "The program asking changed while Wormhole looked at it (a process ended or was replaced). Try the request once more."
            }
            Reason::TooManyQuestions => {
                "Wormhole already has several questions waiting for the user. Wait for them to be answered, then try again."
            }
            Reason::NotAnswered => {
                "The question went away without an answer (its window closed, or no sheet could be shown). It counts as no for now and is not remembered: tell the user, and try once more after they say Wormhole's window is up."
            }
            Reason::Other => "A reason a newer Wormhole gave; read the message.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, Reason};

    /// The list holds every variant: serde's names for a list entry round-trip to itself, and the
    /// words are unique. (A variant added to the enum without being added to the list is caught by the
    /// test below, which reads the variants out of the source.)
    #[test]
    fn the_lists_are_spelled_as_the_wire_spells_them() {
        for code in ErrorCode::ALL {
            let wire = serde_json::to_value(code).expect("serializes");
            assert_eq!(wire, serde_json::json!(code.word()));
            assert!(!code.advice().is_empty());
        }
        for reason in Reason::ALL {
            let wire = serde_json::to_value(reason).expect("serializes");
            assert_eq!(wire, serde_json::json!(reason.word()));
            assert!(!reason.advice().is_empty());
        }
        let mut words: Vec<_> = ErrorCode::ALL.iter().map(|c| c.word()).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), ErrorCode::ALL.len());
    }

    /// Every variant of the enums, found in the source, is in the list (but `Other`).
    #[test]
    fn the_lists_hold_every_variant() {
        let source = include_str!("mod.rs");
        let variants = |name: &str| -> Vec<String> {
            let start = source.find(&format!("pub enum {name} {{")).expect("the enum is in mod.rs");
            let body = &source[start..];
            let body = &body[..body.find("\n}\n").expect("it ends")];
            body.lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//") && !line.starts_with('#') && line.ends_with(','))
                .map(|line| line.trim_end_matches(',').to_string())
                .collect()
        };
        let listed: Vec<String> = ErrorCode::ALL.iter().map(|c| format!("{c:?}")).collect();
        for variant in variants("ErrorCode") {
            assert!(variant == "Other" || listed.contains(&variant), "ErrorCode::{variant} is not in ALL");
        }
        let listed: Vec<String> = Reason::ALL.iter().map(|r| format!("{r:?}")).collect();
        for variant in variants("Reason") {
            assert!(variant == "Other" || listed.contains(&variant), "Reason::{variant} is not in ALL");
        }
    }
}
