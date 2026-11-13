//! What the server asks of the Swift side, and how it can fail to start.

/// Something the server cannot do by itself: it is on the screen, or it is the window's. Drain them with
/// `HookServer::drain_events` after `HookListener::wake`.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum HookEvent {
    /// A second launch, or `wh focus --activate`, asks the app to come to the front.
    Activate,
    /// `wh focus --activate`: bring the window that holds `terminal` to the front (`Activate` brings the app).
    Front { terminal: i64 },
    /// `wh close`: ask the person whether to close `terminal`, as closing it by hand does, and call
    /// `HookServer::close_question_over` when it is answered. Nothing closes unless they say yes.
    CloseAsked { terminal: i64 },
    /// `wh new-terminal --worktree <branch>`: put the new-worktree question up, with `branch` filled in, for the
    /// repository `folder` is in. A worktree is never made on a script's say alone: it is made when the person
    /// confirms, and the terminal opens in it then, typing `command` once its shell is ready when the request gave one.
    WorktreeAsked { folder: String, branch: String, command: Option<String> },
    /// The status line of `terminal`'s agent (`agent`, its command: `claude`) says which model it runs and how
    /// much of its context window is used (0 to 100). Only a coding agent that reports it has one.
    Gauge { terminal: i64, agent: String, model: Option<String>, context: Option<f32> },
    /// `wh new-window`: open a window for window id `window`, whose first terminal `terminal` is planned already
    /// (`Windows::plan`): make a `Workspace` in it with `in_window` and `restore()`. `command` is typed into the
    /// terminal once its shell is ready (the server does it); `activate` says to bring the window forward, else it
    /// opens behind the others and takes no keys.
    NewWindow { window: i64, terminal: i64, folder: Option<String>, activate: bool },
    /// A Probe's hook asks for a notification (the Probe's name is the subtitle, `"<name> (extension)"`): show it
    /// by the notification rules. `probe` groups them, one thread per Probe.
    ProbeNotice { probe: String, subtitle: String, title: String, body: String },
    /// A Probe's hook offers a file in `terminal`'s folder, in the words of the chip. Nothing opens until the
    /// person says so: then `Terminal::resolve_path(text)` finds it.
    ProbeOffer { terminal: i64, words: String, text: String },
    /// A hook failed too often and was paused until the Probes are loaded again; `words` say so.
    ProbePaused { words: String },
    /// A program is driving `terminal` (`typing`, else only reading): say it on the chip of the window that shows
    /// it. `agent` is the coding agent's name as the sentence has it ("Claude Code"); none is a script.
    Driven { terminal: i64, typing: bool, agent: Option<String> },
    /// The rules, a status item, a tinted block or a fold changed: take a frame of every terminal and read
    /// `HookServer::probe_status_items` again.
    ProbesChanged,
    /// The welcome program's sidebar step: put the example band in the sidebar of the window that holds `terminal`
    /// (`show`), or take it away. The band is never a terminal: not listed, not saved, not numbered, not selectable.
    WelcomeDemo { terminal: i64, show: bool },
    /// The welcome in `terminal` is over: the example band goes and a start animation still playing is cut short. `note`: a
    /// first run that was not skipped ended on the primer (no coding agent found), so the sidebar's "no agents" note comes up.
    WelcomeEnded { terminal: i64, note: bool },
    /// A press turned a banner on, or said Allow, while macOS has not been asked: ask it, then tell
    /// `HookServer::set_notification_permission`.
    WelcomeAskPermission,
    /// The welcome's "How much to show" walk opens (`open`) or closes the right panel of the window that holds `terminal`.
    WelcomePanel { terminal: i64, open: bool },
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
