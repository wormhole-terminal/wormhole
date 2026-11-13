//! The hook and control socket of the Swift app: what the GPUI app's `app/control/` and hook handling do,
//! over the `Workspace` and its terminals, behind one UniFFI object.
//!
//! [`HookServer::start`] takes the data folder's single-instance lock and serves the app's socket exactly as
//! core names and protects it (`ipc::acquire_in`: the path from the data folder, a private file, a peer check
//! on the user's uid, the kernel lock a second instance cannot take: it asks this one to come forward
//! instead). Then:
//!
//! - an agent's hook (`WORMHOLE_BIN hook …`) becomes a status event for its terminal (`messages`), so a real
//!   Claude Code or Codex session shows working or needing you in the sidebar; its usage reports go to the
//!   `UsageService`;
//! - the control line (`Wormhole list|status|notify|send-text|new-terminal|read|keys|split|focus|…`) is
//!   answered with core's rules and replies (`control`), after the gate (`gate`) that asks the person through
//!   the `AccessDesk` unless the mode is open;
//! - the event bus (`tap`) tells the streams and waits what happens to the terminals.
//!
//! What only Swift can do (come to the front, show a notification, ask whether to close) is queued as a
//! [`HookEvent`] behind one coalesced `wake()`, as the workspace's own events are.

mod control;
mod control_line;
mod core;
mod dispatch;
mod drive;
mod events;
mod gate;
mod messages;
mod processes;
mod reach;
mod tap;
mod terminals;
mod views;
mod welcome;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::ipc::{self, Instance, InstanceLock};
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::DataRoot;

pub(crate) use self::core::Core;
pub use self::events::{HookError, HookEvent};
use self::tap::Publisher;
pub(crate) use self::tap::Tap;
pub use self::welcome::{
    WelcomeDemoRow, WelcomeDemoState, WelcomeDock, WelcomePlan, welcome_demo_label, welcome_demo_next_ms,
    welcome_demo_rows, welcome_demo_title, welcome_dock,
};
use crate::aux_backends::{AccessDesk, AuxError};
use crate::files_panel::UsageService;
use crate::workspace::Workspace;

/// What Swift implements to hear that there is something to drain. Called from a Rust thread with no lock
/// held, at most once between two drains; it must return at once (note the wake and hop to the main queue)
/// and never call back into the server from inside.
#[uniffi::export(with_foreign)]
pub trait HookListener: Send + Sync {
    fn wake(&self);
}

/// The socket and the control line of one workspace.
#[derive(uniffi::Object)]
pub struct HookServer {
    pub(crate) core: Arc<Core>,
    socket: PathBuf,
    requests: smol::channel::Receiver<ipc::Incoming>,
    /// The instance lock, held while this server runs (the kernel lock goes with the process, crashes too).
    lock: Mutex<Option<InstanceLock>>,
}

#[uniffi::export]
impl HookServer {
    /// Become the running instance for the data folder and serve its socket. `data_dir` is the folder (named
    /// `Wormhole-swift*`: tests and previews pass their own); `None` is core's own, which must be the
    /// experiment's (`WORMHOLE_DATA_DIR`) and whose socket the terminals' `WORMHOLE_SOCKET` names.
    #[uniffi::constructor]
    pub fn start(
        workspace: Arc<Workspace>,
        usage: Arc<UsageService>,
        access: Arc<AccessDesk>,
        listener: Arc<dyn HookListener>,
        data_dir: Option<String>,
    ) -> Result<Arc<Self>, HookError> {
        let dir = crate::aux_backends::resolve_data_dir(data_dir.clone()).map_err(not_isolated)?;
        let socket = match data_dir {
            Some(_) => ipc::socket_path_in(&paths::temp(), &dir),
            None => ipc::socket_path(),
        };
        let (requests, bus, lock) = match ipc::acquire_in(&dir, &socket, true) {
            Ok(Instance::Primary { events, bus, lock }) => (events, bus, lock),
            Ok(Instance::Secondary) => return Err(HookError::AlreadyServed),
            Err(err) => return Err(HookError::Failed { reason: format!("the instance lock: {err}") }),
        };
        // A socket that could not be served is no hook socket: the lock is kept (core's rule), and this says so.
        if !socket.exists() {
            return Err(HookError::Failed { reason: format!("{} could not be served", socket.display()) });
        }
        let shared = workspace.shared();
        let publisher = Publisher::start(shared, bus.clone(), shared.window_id())
            .map_err(|err| HookError::Failed { reason: format!("the event thread: {err}") })?;
        if shared.tap.set(publisher.clone()).is_err() {
            return Err(HookError::Failed { reason: "this workspace already has a server".into() });
        }
        let core = Core::new(workspace, usage, access, bus, publisher, DataRoot::new(dir), listener);
        // Every other window's terminals are heard too, the ones that exist and the ones made later.
        for window in core.shares() {
            core.attach(&window);
        }
        if let Some((windows, _)) = core.shared.windows() {
            let weak = Arc::downgrade(&core);
            windows.on_register(move |window| {
                if let Some(core) = weak.upgrade() {
                    core.attach(window);
                }
            });
        }
        let server =
            Arc::new(Self { core, socket, requests: requests.clone(), lock: Mutex::new(Some(lock)) });
        dispatch::spawn(server.core.clone(), requests)
            .map_err(|err| HookError::Failed { reason: format!("the socket's thread: {err}") })?;
        crate::probes::reload(&server.core);
        Ok(server)
    }

    /// Where the app listens: what `WORMHOLE_SOCKET` holds in a terminal of this data folder.
    pub fn socket_path(&self) -> String {
        self.socket.to_string_lossy().into_owned()
    }

    /// The window in front is `window` now: `new-terminal` without `--window` opens there and `list` marks it.
    pub fn set_front_window(&self, window: i64) {
        self.core.set_front_window(window);
    }

    /// What the server asks Swift to do, since the last drain; re-arms the wake.
    pub fn drain_events(&self) -> Vec<HookEvent> {
        self.core.ui.drain()
    }

    /// The question a `HookEvent::CloseAsked` put to the person was answered (yes or no): another may be asked.
    pub fn close_question_over(&self) {
        self.core.close_question.store(false, Ordering::Release);
    }

    /// The new-worktree sheet came up (`up`) or went away, however it was opened: while it is up a command line's
    /// `--worktree` request is answered Failed (the GPUI app's "a worktree dialog is already up").
    pub fn worktree_sheet(&self, up: bool) {
        self.core.worktree_question.store(up, Ordering::Release);
    }

    /// Another profile's desk (a profile's windows were opened beside the first's): what its windows' requests are judged
    /// by, with its own mode, approvals and questions.
    pub fn add_desk(&self, desk: Arc<AccessDesk>) {
        self.core.add_desk(desk);
    }

    /// Take approval `id` of profile `profile`'s desk back (`AccessDesk::revoke`, which this does) and end the
    /// streams it let in. Whether there was one.
    pub fn revoke_approval(&self, profile: String, id: u64) -> bool {
        let Some(seat) = self.core.seat_of_profile(&profile) else { return false };
        let revoked = seat.desk.revoke(id);
        if revoked {
            self.core.bus.revoke_holder(seat.holder(false, id));
        }
        revoked
    }

    /// Take stored approval `id` (a database row of `AccessDesk::approvals`) of profile `profile` back and end the
    /// streams it let in. Whether there was one.
    pub fn revoke_stored_approval(&self, profile: String, id: u64) -> bool {
        let Some(seat) = self.core.seat_of_profile(&profile) else { return false };
        let revoked = seat.desk.revoke_stored(id);
        if revoked {
            self.core.bus.revoke_holder(seat.holder(true, id));
        }
        revoked
    }

    /// Switch one capability of approval `id` of profile `profile`'s desk (`stored`: a database row of
    /// `AccessDesk::approvals`) on or off, as the ticks of its row in Settings do. Taking one away ends the streams
    /// that were started under the approval, which it may have let in. Whether the approval changed.
    pub fn set_approval_capability(
        &self,
        profile: String,
        id: u64,
        stored: bool,
        capability: crate::aux_backends::AccessCapability,
        on: bool,
    ) -> bool {
        let Some(seat) = self.core.seat_of_profile(&profile) else { return false };
        let changed = seat.desk.set_capability(id, stored, capability, on);
        if changed && !on {
            self.core.bus.revoke_holder(seat.holder(stored, id));
        }
        changed
    }

    /// Profile `profile`'s settings changed: read them again, and end the streams that only the mode "Allowed
    /// while open" let read every terminal when it is no longer that.
    pub fn settings_changed(&self, profile: String) {
        self.core.settings_changed(&profile);
        // The `probes` flag, the output-folds flag and "Never hide lines" are read again with them.
        crate::probes::reload(&self.core);
    }

    /// macOS asked Wormhole to open these `file://` URLs (`application(_:open:)`): a terminal each in window `window`, a
    /// script run in its folder. How many terminals were made.
    pub fn open_from_system(&self, urls: Vec<String>, window: Option<i64>) -> u32 {
        control::open_from_system(&self.core, &urls, window)
    }

    /// Stop serving: the socket is removed, the lock given up, the streams closed. Explicit: a Swift `deinit`
    /// only frees, and the listener thread of the socket ends with the process.
    pub fn stop(&self) {
        self.requests.close();
        self.core.bus.close_all(wormhole_core::ipc::control::event::CloseReason::Quitting);
        remove(&self.socket);
        self.lock.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}

fn remove(socket: &Path) {
    let _ = std::fs::remove_file(socket);
}

fn not_isolated(error: AuxError) -> HookError {
    match error {
        AuxError::NotIsolated { reason } => HookError::NotIsolated { reason },
        AuxError::Failed { reason } => HookError::Failed { reason },
    }
}

/// What a `Wormhole` command line asked of the app's own executable (`hook`, `statusline`, a control verb):
/// done, with its exit code; `None` for a launch. The Swift app's `main` asks first, before anything opens:
/// the executable is `WORMHOLE_BIN` in every terminal, and the agents' hook scripts run `$WORMHOLE_BIN hook …`
/// on it. `arguments` are `CommandLine.arguments`, the program first.
#[uniffi::export]
pub fn cli_exit_code(arguments: Vec<String>) -> Option<i32> {
    use std::io::Write as _;
    match ipc::command(&arguments) {
        ipc::Command::Hook(rest) => {
            ipc::run_cli(rest);
            Some(0)
        }
        ipc::Command::StatusLine => {
            ipc::run_status_line_cli();
            Some(0)
        }
        ipc::Command::Control(rest) => Some(ipc::run_control_cli(rest)),
        // The first run as a program in a terminal: this executable is what the welcome terminal starts.
        ipc::Command::Welcome(rest) => Some(wormhole_core::onboarding::welcome::run(rest)),
        // Most likely a command a newer build installed into an agent's settings: a launch would bring
        // the running app to the front every time it runs.
        ipc::Command::Unknown(command) => {
            let _ = writeln!(std::io::stderr(), "Wormhole: unknown command {command:?}");
            Some(2)
        }
        ipc::Command::App => None,
    }
}
