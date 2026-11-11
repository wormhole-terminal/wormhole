//! What can go wrong reading or writing a repository, as the panels say it. Separate from `FfiError`: the
//! git objects are not terminals and this keeps their cases to themselves.

use wormhole_core::git::GitError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GitBackendError {
    /// The process is not an isolated instance of the experiment (`isolation`).
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// The folder is in no repository: the tabs say "not a repository".
    #[error("not a git repository")]
    NotARepository,
    /// git, or the rules around it, said no: the message is in git's words (or core's).
    #[error("{message}")]
    Failed { message: String },
}

impl From<crate::FfiError> for GitBackendError {
    fn from(err: crate::FfiError) -> Self {
        Self::NotIsolated { reason: err.to_string() }
    }
}

impl GitBackendError {
    /// `err` from one of core's git functions: "not a repository" is its own case, anything else keeps
    /// git's words (a missing git, a dubious owner, a bad revision, a timeout).
    pub(crate) fn of(err: &anyhow::Error) -> Self {
        match GitError::of(err) {
            GitError::NotARepo => Self::NotARepository,
            other => Self::Failed { message: other.to_string() },
        }
    }

    pub(crate) fn failed(message: impl Into<String>) -> Self {
        Self::Failed { message: message.into() }
    }
}
