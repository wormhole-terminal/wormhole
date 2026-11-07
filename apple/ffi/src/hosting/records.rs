//! The plain values of the launch plan and a terminal's hosting.

/// A terminal the last run saved with the session host that ran it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SavedTerminal {
    pub host_id: String,
    pub cwd: Option<String>,
    pub shell: Option<String>,
}

/// A live host no saved terminal of any profile names (what a crash of the saved state, or another
/// tool, left running). It is offered, never ended: what runs in it is the person's.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OrphanHost {
    pub host_id: String,
    /// Unix seconds, as the host's meta says.
    pub started_at: u64,
}

/// What the launch does with the hosts it found (core's `LaunchHosts`), in the sidebar's order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LaunchPlan {
    /// Saved terminals whose host runs: `HostedTerminals.attach`.
    pub attach: Vec<SavedTerminal>,
    /// Saved terminals whose host is gone: a new terminal in the same folder (their text is not replayed).
    pub replay: Vec<SavedTerminal>,
    /// Live hosts nobody names, oldest first (not yet asked whether they take this app).
    pub orphans: Vec<OrphanHost>,
}

/// Where a terminal's program runs, as the status bar says it (the app's `Hosting`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TerminalHosting {
    /// In the app's own process: it ends when the app quits.
    Local,
    /// In a session host, which outlives the app.
    Persistent { host_id: String },
    /// Let go of: the program runs on in its host for the next attach.
    Detached { host_id: String },
    /// Its host went away (crashed, killed, ended from the Session Hosts window) and took the program.
    Lost { host_id: String },
}
