//! What a settings call can refuse, in a few cases and never a panic. Kept apart from `FfiError` (the
//! terminal's) so the two areas grow without touching each other's file.

use crate::error::FfiError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SettingsError {
    /// The process is not an isolated instance of the experiment (`isolation`).
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// A key or a value core refuses; `reason` is core's sentence, fit to show beside the control.
    #[error("{reason}")]
    Invalid { reason: String },
    /// A profile that cannot be found or read.
    #[error("profile: {reason}")]
    Profile { reason: String },
    /// An agent's hooks that cannot be written or removed, or that have no home to go in.
    #[error("agents: {reason}")]
    Agents { reason: String },
}

impl From<FfiError> for SettingsError {
    fn from(err: FfiError) -> Self {
        match err {
            FfiError::NotIsolated { reason } => Self::NotIsolated { reason },
            other => Self::Invalid { reason: other.to_string() },
        }
    }
}
