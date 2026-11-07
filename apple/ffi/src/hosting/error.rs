//! What can go wrong starting a terminal in a host or attaching to one.

use wormhole_core::ipc::hosts::UnavailableKind;

/// Why a host that runs cannot be shown (core's `UnavailableKind`); the interface words each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HostUnavailableKind {
    Refused,
    TooOld,
    TooNew,
    Busy,
    TimedOut,
    Unreachable,
    Failed,
}

impl From<UnavailableKind> for HostUnavailableKind {
    fn from(kind: UnavailableKind) -> Self {
        match kind {
            UnavailableKind::Refused => Self::Refused,
            UnavailableKind::TooOld => Self::TooOld,
            UnavailableKind::TooNew => Self::TooNew,
            UnavailableKind::Busy => Self::Busy,
            UnavailableKind::TimedOut => Self::TimedOut,
            UnavailableKind::Unreachable => Self::Unreachable,
            UnavailableKind::Failed => Self::Failed,
        }
    }
}

/// The cases Swift tells apart: `Gone` and `Unavailable` are an attach's two meanings in core
/// (`ipc::hosts::AttachFailure`): a host that is not there lets its terminal come back as a new shell;
/// one that runs and cannot be shown keeps its terminal's place and says why.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HostingError {
    /// The data folder is not the experiment's (`isolation`).
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// There is no session host program to start (`HostingProgram`).
    #[error("no session host program: {reason}")]
    NoProgram { reason: String },
    /// A new host or its program did not start: the terminal may run in the app's own process instead.
    #[error("could not start in a session host: {reason}")]
    Start { reason: String },
    /// The host to attach to is gone.
    #[error("the session host is gone")]
    Gone,
    /// The host runs and cannot be shown (refused, another protocol, busy, silent). `transient`: trying
    /// again may work (core's `UnavailableKind::is_transient`).
    #[error("the session host cannot be shown: {reason}")]
    Unavailable { reason: String, kind: HostUnavailableKind, transient: bool },
    /// The saved terminals could not be read or written.
    #[error("the saved terminals: {reason}")]
    Store { reason: String },
}

impl HostingError {
    pub(crate) fn store(why: impl std::fmt::Display) -> Self {
        Self::Store { reason: format!("{why:#}") }
    }
}
