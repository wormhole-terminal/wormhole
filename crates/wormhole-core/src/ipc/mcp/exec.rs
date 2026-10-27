//! Carrying a tool's request out: the same round trip over the control socket the command line makes, as
//! the same caller (the tab this process runs in, and its place in the process tree, which the app reads
//! from the socket itself).

use std::path::PathBuf;
use std::time::Duration;

use super::result::Outcome;
use crate::ipc::control::{Action, Request, show_automation};
use crate::ipc::exchange::{Caller, Cancel, answer_wait, exchange_cancellable};
use crate::platform::paths;

/// Where requests go, who they come from, and what a relative folder is relative to.
#[derive(Debug, Clone)]
pub(in crate::ipc) struct Endpoint {
    pub(super) caller: Caller,
    /// A tool's relative folder is under this one (the command line resolves its own the same way, against
    /// where it runs; the app cannot know).
    pub(super) cwd: PathBuf,
    pub(super) home: Option<PathBuf>,
    /// Whether the Automation feature flag is on, as the command line asks it (`wh help` hides the verbs the
    /// same way): read when the tools are listed, so turning it on shows them without a restart.
    pub(super) automation: fn() -> bool,
}

impl Endpoint {
    pub(super) fn from_env() -> Self {
        Self {
            caller: Caller::from_env(),
            cwd: std::env::current_dir().unwrap_or_default(),
            home: paths::home(),
            automation: show_automation,
        }
    }
}

/// `action`, sent and answered. The call ends early, with a result nobody will read, when `cancel` is
/// cancelled.
pub(super) fn run(at: &Endpoint, action: Action, cancel: &Cancel) -> Outcome {
    // A tool waits as long as the request says (a `wait` or a `run`), and the moment the app takes to decide.
    let request = Request::new(action, at.caller.from.clone());
    let wait: Duration = answer_wait(&request.action);
    match exchange_cancellable(&at.caller.socket, &request, wait, Some(cancel)) {
        Ok(reply) => Outcome::of_reply(&request.action, &reply, at.home.as_deref()),
        Err(failure) => Outcome::failed("unavailable", failure.words()),
    }
}
