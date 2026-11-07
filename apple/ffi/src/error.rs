//! The errors that cross the seam: a few cases, never a panic.

/// What can go wrong starting or driving a terminal.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    /// The process is not an isolated instance of the experiment (`isolation`): it would have
    /// touched the real app's data.
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// The program could not be started.
    #[error("could not start the program: {reason}")]
    Start { reason: String },
    /// A theme could not be found or read, or a colour could not be parsed.
    #[error("theme: {reason}")]
    Theme { reason: String },
    /// The terminal was closed.
    #[error("the terminal is closed")]
    Closed,
}
