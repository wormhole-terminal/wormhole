//! What the server asks of the Swift side, and how it can fail to start.

/// Something the server cannot do by itself: it is on the screen, or it is the window's. Drain them with
/// `HookServer::drain_events` after `HookListener::wake`.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum HookEvent {
    /// A second launch, or `wh focus --activate`, asks the app to come to the front.
    Activate,
    /// `wh notify`: show a notification about `terminal`, in the caller's words (already cleaned).
    Notice { terminal: i64, title: String, body: String },
    /// `wh close`: ask the person whether to close `terminal`, as closing it by hand does, and call
    /// `HookServer::close_question_over` when it is answered. Nothing closes unless they say yes.
    CloseAsked { terminal: i64 },
    /// `wh new-terminal --worktree <branch>`: put the new-worktree question up, with `branch` filled in, for the
    /// repository `folder` is in. A worktree is never made on a script's say alone: it is made when the person
    /// confirms, and the terminal opens in it then.
    WorktreeAsked { folder: String, branch: String },
    /// The status line of `terminal`'s agent (`agent`, its command: `claude`) says which model it runs and how
    /// much of its context window is used (0 to 100). Only a coding agent that reports it has one.
    Gauge { terminal: i64, agent: String, model: Option<String>, context: Option<f32> },
    /// `wh new-window`: open a window for window id `window`, whose first terminal `terminal` is planned already
    /// (`Windows::plan`): make a `Workspace` in it with `in_window` and `restore()`. `command` is typed into the
    /// terminal once its shell is ready (the server does it); `activate` says to bring the window forward, else it
    /// opens behind the others and takes no keys.
    NewWindow { window: i64, terminal: i64, folder: Option<String>, activate: bool },
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
