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
mod core;
mod dispatch;
mod drive;
mod events;
mod gate;
mod messages;
mod processes;
mod tap;
mod terminals;
mod views;

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
        let root = DataRoot::new(dir);
        let profile = root.default_profile();
        let core = Core::new(workspace, usage, access, bus, publisher, (root, profile), listener);
        // Every other window's terminals are heard too, the ones that exist and the ones made later.
        for window in core.shares() {
            core.attach(&window);
        }
        if let Some((windows, _)) = core.shared.windows.get() {
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

    /// What the server asks Swift to do, since the last drain; re-arms the wake.
    pub fn drain_events(&self) -> Vec<HookEvent> {
        self.core.ui.drain()
    }

    /// The question a `HookEvent::CloseAsked` put to the person was answered (yes or no): another may be asked.
    pub fn close_question_over(&self) {
        self.core.close_question.store(false, Ordering::Release);
    }

    /// Take approval `id` back (`AccessDesk::revoke`, which this does) and end the streams it let in. Whether
    /// there was one.
    pub fn revoke_approval(&self, id: u64) -> bool {
        let revoked = self.core.desk.revoke(id);
        if revoked {
            self.core.bus.revoke_holder(gate::holder(false, id));
        }
        revoked
    }

    /// Take stored approval `id` (a database row of `AccessDesk::approvals`) back and end the streams it let in.
    /// Whether there was one.
    pub fn revoke_stored_approval(&self, id: u64) -> bool {
        let revoked = self.core.desk.revoke_stored(id);
        if revoked {
            self.core.bus.revoke_holder(gate::holder(true, id));
        }
        revoked
    }

    /// The settings changed: read them again, and end the streams that only the mode "Allowed while open" let
    /// read every terminal when it is no longer that.
    pub fn settings_changed(&self) {
        self.core.settings_changed();
        // The `probes` flag, the output-folds flag and "Never hide lines" are read again with them.
        crate::probes::reload(&self.core);
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
        // The first-run welcome is the GPUI app's own terminal program.
        ipc::Command::Welcome(_) => {
            let _ = writeln!(std::io::stderr(), "Wormhole: this app has no welcome terminal");
            Some(2)
        }
        // Most likely a command a newer build installed into an agent's settings: a launch would bring
        // the running app to the front every time it runs.
        ipc::Command::Unknown(command) => {
            let _ = writeln!(std::io::stderr(), "Wormhole: unknown command {command:?}");
            Some(2)
        }
        ipc::Command::App => None,
    }
}
