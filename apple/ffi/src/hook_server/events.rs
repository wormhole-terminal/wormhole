//! What the server asks of the Swift side, and how it can fail to start.

/// Something the server cannot do by itself: it is on the screen, or it is the window's. Drain them with
/// `HookServer::drain_events` after `HookListener::wake`.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HookEvent {
    /// A second launch, or `wh focus --activate`, asks the app to come to the front.
    Activate,
    /// `wh notify`: show a notification about `terminal`, in the caller's words (already cleaned).
    Notice { terminal: i64, title: String, body: String },
    /// `wh close`: ask the person whether to close `terminal`, as closing it by hand does, and call
    /// `HookServer::close_question_over` when it is answered. Nothing closes unless they say yes.
    CloseAsked { terminal: i64 },
}

/// Why the server did not start.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HookError {
    /// The process (or the folder it was given) is not the experiment's own.
    #[error("not isolated: {reason}")]
    NotIsolated { reason: String },
    /// Another process holds this data folder's instance lock and serves its socket: it was asked to
    /// come to the front, and this one runs without hooks and without a control line.
    #[error("another Wormhole serves this data folder")]
    AlreadyServed,
    /// The lock or the socket could not be made.
    #[error("{reason}")]
    Failed { reason: String },
}
