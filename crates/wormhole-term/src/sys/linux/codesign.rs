//! No code signatures: every peer of the same user is served, as an unsigned host does on macOS.

use crate::sys::socket::Stream;

use crate::sys::codesign::{Refusal, Signature, SignerKind};

/// A requirement a peer's signature must satisfy; there are none to write here.
pub struct Requirement(());

pub(crate) fn requirement(_text: &str, _identifier: &str, _team: &str) -> Option<Requirement> {
    None
}

pub(crate) fn self_satisfies(_requirement: &Requirement) -> bool {
    false
}

pub(crate) fn own_team_id() -> Option<String> {
    None
}

pub(crate) fn check_peer(_stream: &Stream, _requirement: &Requirement) -> Result<(), Refusal> {
    Ok(())
}

pub(crate) fn of_pid(_pid: u32) -> Signature {
    Signature::of(SignerKind::Unreadable)
}
