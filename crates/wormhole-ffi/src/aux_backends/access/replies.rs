//! What a request that waited is told when it is not let in (`agent_access/ask.rs`): agents read these, so they say
//! not to retry.

use wormhole_core::ipc::control::{Error, ErrorCode, Reason, Reply};

pub(super) fn declined(label: &str) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to control Wormhole: do not retry; tell the user what you wanted to do and ask them"),
    )
    .because(Reason::Declined)
    .into()
}

pub(super) fn declined_terminal(label: &str, terminal: i64) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user did not allow {label} to use terminal {terminal}, which it did not open: do not retry; tell the user what you wanted to do there and ask them"),
    )
    .because(Reason::Declined)
    .into()
}

/// Access was switched off while the question was open (`agent_access/ask.rs`).
pub(super) fn access_off() -> Reply {
    Error::new(ErrorCode::NotAllowed, "agent access was switched off while the question was open")
        .because(Reason::AgentAccessOff)
        .into()
}

pub(super) fn timed_out(label: &str) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("nobody answered the question about {label} in Wormhole within a minute; it counts as no, and is not remembered"),
    )
    .because(Reason::AskTimedOut)
    .into()
}

pub(super) fn not_answered(label: &str) -> Reply {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the question about {label} went away without an answer (its window closed, or none could be shown); it counts as no for now, and is not remembered"),
    )
    .because(Reason::NotAnswered)
    .into()
}
