//! `HostedTerminals`: starting terminals in session hosts, attaching to the ones the last run left, and
//! what quitting does with them.
//!
//! Threads: every method that starts or attaches blocks (the host's start and the handshake take a few
//! milliseconds, a few seconds when something is wrong: `wormhole_term::client`). The Swift side calls
//! them from the main actor when it must (a terminal's view needs its source at once) and from a task
//! when it can. Nothing here holds a lock across one of those waits.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use wormhole_core::config::Config;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc::host_binary::HostProgram;
use wormhole_core::ipc::hosts::{self, AttachFailure, HostMeta, LaunchHosts, Probed, Restore};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::Store;
use wormhole_term::GridSize;
use wormhole_term::client::{HostCommand, socket_of};

use super::registry::Registry;
use super::{HostSlot, HostingError, HostingProgram, LaunchPlan, OrphanHost, SavedTerminal, program};
use crate::aux_backends::{AuxError, resolve_data_dir};
use crate::events::TerminalListener;
use crate::terminal::Terminal;

/// How long one orphan gets to answer before it is left alone (`window/hosting.rs`).
const ORPHAN_ANSWER: Duration = Duration::from_millis(500);

/// A terminal's place and its program, as a start or an attach is asked for it.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HostedStart {
    /// The workspace's id for the terminal (`Opened { id }`): its shell's `WORMHOLE_TAB_ID`.
    pub tab: i64,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl HostedStart {
    fn size(&self) -> GridSize {
        GridSize {
            cols: self.cols,
            rows: self.rows,
            cell_width: self.cell_width,
            cell_height: self.cell_height,
        }
    }
}

#[derive(uniffi::Object)]
pub struct HostedTerminals {
    data_dir: PathBuf,
    hosts_dir: PathBuf,
    program: HostProgram,
    registry: Arc<Registry>,
    /// The live hosts found at launch, each taken once (`LaunchHosts::restore`).
    launch: Mutex<LaunchHosts>,
    /// The orphans of the plan, until `adoptable_orphans` asks them.
    orphans: Mutex<Vec<HostMeta>>,
    /// Every terminal this object made, to let go of them as the app quits.
    made: Mutex<Vec<Weak<Terminal>>>,
    /// The ids saved this run: what `finish_restore` keeps of the rows the last run left.
    saved_now: Mutex<HashSet<i64>>,
}

#[uniffi::export]
impl HostedTerminals {
    /// `data_dir`: a folder named `Wormhole-swift*` (tests), or `None` for core's own, which must be the
    /// experiment's. `host_binary`: an executable to start as the host instead of the one beside the app
    /// (tests and development; see `HostingProgram`). `window` is the id the terminals' rows are saved under.
    #[uniffi::constructor]
    pub fn new(
        data_dir: Option<String>,
        host_binary: Option<String>,
        window: i64,
    ) -> Result<Arc<Self>, HostingError> {
        let data_dir = resolve_data_dir(data_dir).map_err(|err| match err {
            AuxError::NotIsolated { reason } => HostingError::NotIsolated { reason },
            AuxError::Failed { reason } => HostingError::Store { reason },
        })?;
        Ok(Arc::new(Self {
            hosts_dir: hosts::hosts_dir(&data_dir),
            registry: Arc::new(Registry::open(&data_dir, window)?),
            program: program::locate(host_binary.as_deref()),
            data_dir,
            launch: Mutex::default(),
            orphans: Mutex::default(),
            made: Mutex::default(),
            saved_now: Mutex::default(),
        }))
    }

    /// Whether Settings has Persistent terminals on, by core's rules for feature flags (a release build
    /// reads the flag's release default, which is off, unless the flags are unlocked in `config.json`).
    pub fn enabled(&self) -> bool {
        let profile = DataRoot::new(&self.data_dir).default_profile();
        Config::load_for(&profile).settings.feature_enabled(FeatureFlag::PersistentTerminals)
    }

    /// Where the host program is, or why there is none: with none (or a disposable copy of the app) every
    /// terminal stays in the app's process, as the GPUI app does.
    pub fn program(&self) -> HostingProgram {
        HostingProgram::from(&self.program)
    }

    /// At launch, once: forget the files of hosts that are certainly gone, scan the folder, and match the
    /// live hosts against what every profile's saved terminals name. A saved terminal whose host runs is
    /// to be attached, one whose host is gone starts again, and a live host nobody names is an orphan.
    pub fn plan_at_launch(&self) -> Result<LaunchPlan, HostingError> {
        hosts::forget_gone_hosts(&self.hosts_dir);
        let live = hosts::scan(&self.hosts_dir).map(|scan| scan.live).unwrap_or_else(|err| {
            tracing::warn!("couldn't look for running session hosts: {err}");
            Vec::new()
        });
        let saved = self.registry.saved()?;
        let mut named: HashSet<String> = saved.iter().map(|s| s.host_id.clone()).collect();
        named.extend(self.named_elsewhere());
        let mut launch = LaunchHosts::plan(named.iter().map(String::as_str), live);
        let running = launch.live_ids();
        let (attach, replay): (Vec<SavedTerminal>, Vec<SavedTerminal>) =
            saved.into_iter().partition(|s| running.contains(&s.host_id));
        let orphans = launch.take_orphans();
        let plan = LaunchPlan {
            attach,
            replay,
            orphans: orphans
                .iter()
                .map(|m| OrphanHost { host_id: m.id.clone(), started_at: m.started_at })
                .collect(),
        };
        *locked(&self.launch) = launch;
        *locked(&self.orphans) = orphans;
        Ok(plan)
    }

    /// The orphans of the plan that take this app: their protocol fits and they answer a hello (one that
    /// refuses the app would only fail to attach and be adopted again at every launch). Blocks up to half a
    /// second per host that does not answer: off the main thread. Once: the next call is empty.
    pub fn adoptable_orphans(&self) -> Vec<OrphanHost> {
        let orphans = std::mem::take(&mut *locked(&self.orphans));
        hosts::adoptable(orphans, takes_the_app)
            .into_iter()
            .map(|m| OrphanHost { host_id: m.id, started_at: m.started_at })
            .collect()
    }

    /// Start `what` in a new session host. `Start` when the host or its program would not start (the caller
    /// may start it in the app's process instead and say so once); `NoProgram` when there is no host program.
    pub fn start(
        &self,
        what: HostedStart,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Terminal>, HostingError> {
        let HostProgram::Found(binary) = &self.program else {
            return Err(HostingError::NoProgram { reason: format!("{:?}", self.program()) });
        };
        let id = hosts::new_host_id().map_err(|e| HostingError::Start { reason: e.to_string() })?;
        let hosts_dir = hosts::ensure_hosts_dir(&self.data_dir)
            .map_err(|e| HostingError::Start { reason: e.to_string() })?;
        let options = Terminal::spawn_options(
            what.tab.to_string(),
            what.program.clone(),
            what.args.clone(),
            what.cwd.clone(),
        );
        let host = HostCommand { binary, hosts_dir: &hosts_dir, id: &id };
        let (pty, events) = wormhole_core::terminal::spawn_hosted(&host, what.size(), &options)
            .map_err(|e| HostingError::Start { reason: format!("{e:#}") })?;
        self.taken(&what, id, pty, events, &options, listener)
    }

    /// Attach `what.tab` to the running host `host_id` (a saved terminal or an adopted orphan), fed a repaint
    /// at once when `visible`. `Gone`: start a new terminal instead. `Unavailable`: the host runs and cannot be
    /// shown; the terminal keeps its place in the saved list, and `transient` says whether to try again.
    pub fn attach(
        &self,
        host_id: String,
        what: HostedStart,
        visible: bool,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Terminal>, HostingError> {
        let socket = match locked(&self.launch).restore(Some(&host_id)) {
            Restore::Attach(socket) => socket,
            // Not in the plan (a retry, or a host that came after): its own meta says where it listens.
            Restore::Replay => socket_of(&self.hosts_dir, &host_id).map_err(|_| HostingError::Gone)?,
        };
        let options = Terminal::spawn_options(
            what.tab.to_string(),
            what.program.clone(),
            what.args.clone(),
            what.cwd.clone(),
        );
        match wormhole_core::terminal::attach_hosted(&socket, &host_id, &options, visible) {
            Ok((pty, events)) => self.taken(&what, host_id, pty, events, &options, listener),
            Err(AttachFailure::Gone) => Err(HostingError::Gone),
            Err(AttachFailure::Unavailable(why)) => {
                // The terminal keeps the host's id: the next launch finds this row, not a stranger.
                self.remember(&what, &host_id);
                Err(HostingError::Unavailable { reason: why.detail, transient: why.kind.is_transient() })
            }
        }
    }

    /// After the saved terminals were attached or started again: the rows the last run left under other
    /// ids are forgotten (this run's terminals have their own).
    pub fn finish_restore(&self) -> Result<(), HostingError> {
        let mut keep: Vec<i64> = locked(&self.saved_now).iter().copied().collect();
        keep.extend(self.registry.unhosted_ids()?);
        self.registry.keep_only(&keep)
    }

    /// The app is quitting. `end_hosts` (Quit and End All Terminals, or the box left ticked): every terminal
    /// ends with its host and no row names a host. Otherwise each is let go of and its program runs on.
    pub fn finish(&self, end_hosts: bool) {
        let alive: Vec<Arc<Terminal>> = locked(&self.made).iter().filter_map(Weak::upgrade).collect();
        if end_hosts {
            if let Err(err) = self.registry.forget_all() {
                tracing::warn!("couldn't forget the saved terminals: {err}");
            }
            alive.iter().for_each(|terminal| terminal.close());
        } else {
            alive.iter().for_each(|terminal| {
                terminal.detach();
            });
        }
    }
}

impl HostedTerminals {
    /// The host ids that saved terminals of the other profiles name: a host of theirs is not an orphan.
    fn named_elsewhere(&self) -> Vec<String> {
        let root = DataRoot::new(&self.data_dir);
        let (list, _) = ProfileList::load_or_recover(&root);
        list.profiles()
            .iter()
            .filter(|p| &p.id != list.active_id())
            .flat_map(|p| Store::host_ids_read_only(&root.profile(&p.id)))
            .collect()
    }

    fn remember(&self, what: &HostedStart, host_id: &str) {
        let shell = Some(what.program.clone());
        match self.registry.remember(what.tab, host_id, what.cwd.clone(), shell) {
            Ok(()) => {
                locked(&self.saved_now).insert(what.tab);
            }
            Err(err) => tracing::warn!("couldn't save which host terminal {} runs in: {err}", what.tab),
        }
    }

    /// A running PTY in host `id`: make its object, save the row, count it for the quit.
    fn taken(
        &self,
        what: &HostedStart,
        id: String,
        pty: wormhole_term::Pty,
        events: wormhole_term::PtyEvents,
        options: &wormhole_core::terminal::SpawnOptions,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Terminal>, HostingError> {
        let registry = Arc::clone(&self.registry);
        let tab = what.tab;
        let slot = HostSlot::new(id.clone(), move || {
            if let Err(err) = registry.forget(tab) {
                tracing::warn!("couldn't forget terminal {tab}: {err}");
            }
        });
        let terminal = Terminal::from_parts(pty, events, options, listener, Some(slot))
            .map_err(|e| HostingError::Start { reason: e.to_string() })?;
        self.remember(what, &id);
        let mut made = locked(&self.made);
        made.retain(|weak| weak.strong_count() > 0);
        made.push(Arc::downgrade(&terminal));
        Ok(terminal)
    }
}

/// Whether the host takes this app: its protocol fits and it answers a hello.
fn takes_the_app(meta: &HostMeta) -> bool {
    hosts::unfit(meta.proto_min, meta.proto_max).is_none()
        && matches!(hosts::probe(meta, ORPHAN_ANSWER), Probed::Answered(_))
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
