//! `HostedTerminals`: starting terminals in session hosts, attaching to the ones the last run left, and
//! what quitting does with them.
//!
//! Threads: every method that starts or attaches blocks (the host's start and the handshake take a few
//! milliseconds, a few seconds when something is wrong: `wormhole_term::client`). The Swift side calls
//! them from the main actor when it must (a terminal's view needs its source at once) and from a task
//! when it can. Nothing here holds a lock across one of those waits.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, Weak};
use std::time::Duration;

use wormhole_core::config::Config;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc::host_binary::HostProgram;
use wormhole_core::ipc::hosts::{self, AttachFailure, HostMeta, LaunchHosts, Probed, Restore};
use wormhole_core::storage::paths::{DataRoot, ProfileId, ProfilePaths};
use wormhole_core::storage::store::Store;
use wormhole_term::GridSize;
use wormhole_term::client::AttachError;
use wormhole_term::client::{HostCommand, socket_of};

use super::checkpoints::{self, Checkpoints};
use super::quit::{Fate, QuitGate};
use super::registry::Registry;
use super::{HostSlot, HostingError, HostingProgram, LaunchPlan, OrphanHost, SavedTerminal, program};
use crate::aux_backends::{AuxError, resolve_data_dir};
use crate::events::TerminalListener;
use crate::profile;
use crate::terminal::Terminal;

/// How long one orphan gets to answer before it is left alone (`window/hosting.rs`).
/// How long each host gets to end when all of them are ended on the way out (`window/windows.rs`).
const END_WAIT: Duration = Duration::from_secs(5);
const ORPHAN_ANSWER: Duration = Duration::from_millis(500);

/// A terminal's place and its program, as a start or an attach is asked for it.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HostedStart {
    /// The workspace's id for the terminal (`Opened { id }`): its shell's `WORMHOLE_TAB_ID`.
    pub tab: i64,
    /// The shell it was opened with (`Workspace::launch_facts`: a `terminal::shells` key); `None` lets the project,
    /// `default_shell` and the login shell decide (`launch`).
    pub shell: Option<String>,
    /// A program to run instead of what the terminal would (`--shell`, the welcome's tour, tests), with its arguments,
    /// unless `shell` names one; `None`: the shell `launch` decides.
    pub program: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
    pub cell_width: f32,
    pub cell_height: f32,
    /// A terminal whose host is gone (`SavedTerminal.running` false), started again: the host it ran in, whose checkpoint
    /// (after a restart of the Mac) is drawn above the new shell, and its saved text, drawn when there is none.
    /// Only a start reads them; a terminal in the app's own process starts above `saved_text` too.
    pub old_host: Option<String>,
    pub saved_text: Option<String>,
    /// The profile whose settings and environment it starts with (`None`: the one Wormhole is in). A desk starts its
    /// terminals in its own profile.
    pub profile: Option<String>,
}

impl HostedStart {
    pub(crate) fn size(&self) -> GridSize {
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
    pub(super) hosts_dir: PathBuf,
    /// The folder of the profile these terminals belong to: the settings and the saved text this object reads are its.
    profile: RwLock<ProfilePaths>,
    program: HostProgram,
    pub(super) registry: Arc<Registry>,
    pub(super) checkpoints: Arc<Checkpoints>,
    /// The terminals whose rows say their text is not to be kept: no checkpoint is asked for them either.
    forgetting: Mutex<HashSet<i64>>,
    /// The live hosts found at launch, each taken once (`LaunchHosts::restore`).
    launch: Mutex<LaunchHosts>,
    /// The orphans of the plan, until `adoptable_orphans` asks them.
    orphans: Mutex<Vec<HostMeta>>,
    /// Every terminal this object made (by the workspace's id for it), to save their text and to let go of them as the
    /// app quits.
    pub(super) made: Mutex<Vec<(i64, Weak<Terminal>)>>,
    /// The ids saved this run: what `finish_restore` keeps of the rows the last run left.
    saved_now: Mutex<HashSet<i64>>,
    /// Whether the app quit, and the starts and attaches still under way then.
    pub(super) quit: QuitGate,
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
        // A folder given is a profile of its own (tests); core's own is the profile Wormhole is in.
        let given = data_dir.is_some();
        let data_dir = resolve_data_dir(data_dir).map_err(|err| match err {
            AuxError::NotIsolated { reason } => HostingError::NotIsolated { reason },
            AuxError::Failed { reason } => HostingError::Store { reason },
        })?;
        let root = DataRoot::new(&data_dir);
        let profile = if given { root.default_profile() } else { profile::active_in(&root) };
        Ok(Arc::new(Self {
            hosts_dir: hosts::hosts_dir(&data_dir),
            registry: Arc::new(Registry::open(profile.dir(), window)?),
            profile: RwLock::new(profile),
            checkpoints: Arc::new(Checkpoints::new(&data_dir)),
            forgetting: Mutex::default(),
            program: program::locate(host_binary.as_deref()),
            data_dir,
            launch: Mutex::default(),
            orphans: Mutex::default(),
            made: Mutex::default(),
            saved_now: Mutex::default(),
            quit: QuitGate::default(),
        }))
    }

    /// The window moved to profile `profile` ("Move this window to…"): the rows of its terminals, their text and the
    /// settings this object reads are that profile's from now on, so a terminal started after the move is saved
    /// where the window is.
    pub fn follow_profile(&self, profile: String) -> Result<(), HostingError> {
        let id = ProfileId::parse(&profile).map_err(|err| HostingError::Store { reason: err.to_string() })?;
        let paths = DataRoot::new(&self.data_dir).profile(&id);
        self.registry.follow(paths.dir())?;
        *self.profile.write().unwrap_or_else(PoisonError::into_inner) = paths;
        Ok(())
    }

    /// Whether Settings has Persistent terminals on, by core's rules for feature flags (a release build
    /// reads the flag's release default, which is off, unless the flags are unlocked in `config.json`).
    pub fn enabled(&self) -> bool {
        Config::load_for(&self.profile()).settings.feature_enabled(FeatureFlag::PersistentTerminals)
    }

    /// Where the host program is, or why there is none: with none (or a disposable copy of the app) every
    /// terminal stays in the app's process, as the GPUI app does.
    pub fn program(&self) -> HostingProgram {
        HostingProgram::from(&self.program)
    }

    /// At launch, once: forget the files of hosts that are certainly gone, scan the folder, and match the
    /// live hosts against what every profile's saved terminals name. A saved terminal whose host runs is
    /// to be attached, one whose host is gone starts again (`SavedTerminal.running` says which), and a live host
    /// nobody names is an orphan.
    pub fn plan_at_launch(&self) -> Result<LaunchPlan, HostingError> {
        hosts::forget_gone_hosts(&self.hosts_dir);
        let live = hosts::scan(&self.hosts_dir).map(|scan| scan.live).unwrap_or_else(|err| {
            tracing::warn!("couldn't look for running session hosts: {err}");
            Vec::new()
        });
        let saved = self.registry.saved()?;
        let mut named: HashSet<String> = saved.iter().map(|s| s.host_id.clone()).collect();
        named.extend(self.registry.hosts_of_other_windows()?);
        named.extend(self.named_elsewhere());
        let mut launch = LaunchHosts::plan(named.iter().map(String::as_str), live);
        let running = launch.live_ids();
        let saved: Vec<SavedTerminal> = saved
            .into_iter()
            .map(|s| {
                let running = running.contains(&s.host_id);
                // The text only matters to a terminal whose host is gone.
                SavedTerminal { running, text: s.text.filter(|_| !running), ..s }
            })
            .collect();
        locked(&self.forgetting).extend(saved.iter().filter(|s| s.forget_text).map(|s| s.terminal));
        self.checkpoints.expect_restores(saved.iter().filter(|s| !s.running).map(|s| s.host_id.clone()));
        self.sweep_checkpoints(&running);
        let orphans = launch.take_orphans();
        let plan = LaunchPlan {
            saved,
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

    /// A terminal of another window's desk that moved into this window: this desk lets go of it as the app quits.
    pub fn adopt(&self, tab: i64, terminal: Arc<Terminal>) {
        let mut made = locked(&self.made);
        made.retain(|(_, weak)| weak.strong_count() > 0);
        made.push((tab, Arc::downgrade(&terminal)));
    }

    /// Start `what` in a new session host. `Start` when the host or its program would not start (the caller
    /// may start it in the app's process instead and say so once); `NoProgram` when there is no host program.
    pub fn start(
        &self,
        what: HostedStart,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Terminal>, HostingError> {
        let _under_way = self.quit.begin();
        let HostProgram::Found(binary) = &self.program else {
            return Err(HostingError::NoProgram { reason: format!("{:?}", self.program()) });
        };
        let id = hosts::new_host_id().map_err(|e| HostingError::Start { reason: e.to_string() })?;
        let hosts_dir = hosts::ensure_hosts_dir(&self.data_dir)
            .map_err(|e| HostingError::Start { reason: e.to_string() })?;
        let mut options = self.options(&what);
        let restored_from = self.seed(&what, &mut options);
        let host = HostCommand { binary, hosts_dir: &hosts_dir, id: &id };
        let (pty, events) = wormhole_core::terminal::spawn_hosted(&host, what.size(), &options)
            .map_err(|e| HostingError::Start { reason: format!("{e:#}") })?;
        self.taken(&what, id, pty, events, &options, restored_from, listener)
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
        let _under_way = self.quit.begin();
        let socket = match locked(&self.launch).restore(Some(&host_id)) {
            Restore::Attach(socket) => socket,
            // Not in the plan (a retry, or a host that came after): its own meta says where it listens.
            Restore::Replay => socket_of(&self.hosts_dir, &host_id).map_err(|err| {
                let failure =
                    hosts::failure_of(&self.hosts_dir, &host_id, &AttachError::Unreachable(err.to_string()));
                self.failed(&what, &host_id, failure)
            })?,
        };
        let options = self.options(&what);
        match wormhole_core::terminal::attach_hosted(&socket, &host_id, &options, visible) {
            Ok((pty, events)) => self.taken(&what, host_id, pty, events, &options, None, listener),
            Err(failure) => Err(self.failed(&what, &host_id, failure)),
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
        // First: a start or an attach that ends after this lets go of its host, or ends it; ending waits for them.
        self.quit.quit(end_hosts);
        let alive: Vec<Arc<Terminal>> =
            locked(&self.made).iter().filter_map(|(_, weak)| weak.upgrade()).collect();
        if end_hosts {
            if let Err(err) = self.registry.forget_all() {
                tracing::warn!("couldn't forget the saved terminals: {err}");
            }
            alive.iter().for_each(|terminal| terminal.close());
            // Every live host of the data folder, not only the ones this run made: another profile's that
            // was never opened, an orphan not adopted, one left running behind a closed terminal.
            hosts::end_all_live(&self.hosts_dir, END_WAIT);
        } else {
            alive.iter().for_each(|terminal| {
                terminal.detach();
            });
        }
    }
}

impl HostedTerminals {
    pub(super) fn profile(&self) -> ProfilePaths {
        self.profile.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

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

    /// What `what` starts with in this desk's profile, without the saved text: `seed` decides what it starts above.
    fn options(&self, what: &HostedStart) -> wormhole_core::terminal::SpawnOptions {
        let what = HostedStart { saved_text: None, ..what.clone() };
        crate::launch::options(&DataRoot::new(&self.data_dir), &self.profile(), &what)
    }

    fn remember(&self, what: &HostedStart, host_id: &str) {
        match self.registry.remember(what.tab, host_id, what.cwd.clone(), what.shell.clone()) {
            Ok(()) => {
                locked(&self.saved_now).insert(what.tab);
            }
            Err(err) => tracing::warn!("couldn't save which host terminal {} runs in: {err}", what.tab),
        }
    }

    /// What an attach that did not work means: gone (start a new terminal) or a host that runs and cannot be shown.
    fn failed(&self, what: &HostedStart, host_id: &str, failure: AttachFailure) -> HostingError {
        match failure {
            AttachFailure::Gone => HostingError::Gone,
            AttachFailure::Unavailable(why) => {
                // The terminal keeps the host's id: the next launch finds this row, not a stranger.
                self.remember(what, host_id);
                HostingError::Unavailable {
                    reason: why.detail,
                    kind: why.kind.into(),
                    transient: why.kind.is_transient(),
                }
            }
        }
    }

    /// A terminal started again above what it showed: the checkpoint of the host it ran in when that reads (after a
    /// restart of the Mac), else its saved text. The host it was restored from, when a checkpoint was drawn.
    fn seed(
        &self,
        what: &HostedStart,
        options: &mut wormhole_core::terminal::SpawnOptions,
    ) -> Option<String> {
        let old = what.old_host.as_deref()?;
        if let Some(key) = self.checkpoint_key(what.tab, old)
            && self.checkpoints.seed(old, &key, options)
        {
            return Some(old.to_owned());
        }
        if let Some(text) = what.saved_text.as_deref() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs());
            options.restore = Some(wormhole_core::terminal::restored::seed_from_text(text, now))
                .filter(|seed| !seed.is_empty());
        }
        None
    }

    /// The key host `host` seals its checkpoint with, when this terminal keeps checkpoints at all.
    fn checkpoint_key(&self, tab: i64, host: &str) -> Option<wormhole_term::checkpoint::Key> {
        if locked(&self.forgetting).contains(&tab) || !checkpoints::on(&self.profile()) {
            return None;
        }
        self.registry.checkpoint_key(host)
    }

    /// The checkpoints of old hosts at launch (off the main thread): `running` are spared.
    fn sweep_checkpoints(&self, running: &HashSet<String>) {
        let any_on = checkpoints::on(&self.profile()) || self.other_profiles_keep_checkpoints();
        let (checkpoints, running) = (Arc::clone(&self.checkpoints), running.clone());
        let sweep = std::thread::Builder::new()
            .name("wormhole-ffi-checkpoint-sweep".into())
            .spawn(move || checkpoints.sweep(running, any_on));
        drop(sweep);
    }

    fn other_profiles_keep_checkpoints(&self) -> bool {
        let root = DataRoot::new(&self.data_dir);
        let (list, _) = ProfileList::load_or_recover(&root);
        list.profiles().iter().any(|p| checkpoints::on(&root.profile(&p.id)))
    }

    /// A running PTY in host `id`: make its object, save the row, ask for checkpoints, count it for the quit.
    #[allow(clippy::too_many_arguments)]
    fn taken(
        &self,
        what: &HostedStart,
        id: String,
        pty: wormhole_term::Pty,
        events: wormhole_term::PtyEvents,
        options: &wormhole_core::terminal::SpawnOptions,
        restored_from: Option<String>,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Terminal>, HostingError> {
        let registry = Arc::clone(&self.registry);
        let checkpoints = Arc::clone(&self.checkpoints);
        let tab = what.tab;
        let closing_hosts: Vec<String> =
            [Some(id.clone()), restored_from.clone()].into_iter().flatten().collect();
        let slot = HostSlot::new(id.clone(), move || {
            if let Err(err) = registry.forget(tab) {
                tracing::warn!("couldn't forget terminal {tab}: {err}");
            }
            checkpoints.forget_later(closing_hosts.clone());
        });
        let terminal = Terminal::from_parts(
            pty,
            events,
            options,
            &self.profile().id().to_string(),
            listener,
            Some(slot),
        )
        .map_err(|e| HostingError::Start { reason: e.to_string() })?;
        self.remember(what, &id);
        if let Some(key) = self.checkpoint_key(tab, &id) {
            self.checkpoints.start(&terminal, &id, key, restored_from);
        }
        {
            let mut made = locked(&self.made);
            made.retain(|(_, weak)| weak.strong_count() > 0);
            made.push((tab, Arc::downgrade(&terminal)));
        }
        // Made after the app quit (counted in `made` first, so `finish` sees it when it quit after this): nobody takes it.
        match self.quit.fate() {
            Fate::Runs => {}
            Fate::LetGo => {
                terminal.detach();
            }
            Fate::Ends => terminal.close(),
        }
        Ok(terminal)
    }
}

/// Whether the host takes this app: its protocol fits and it answers a hello.
fn takes_the_app(meta: &HostMeta) -> bool {
    hosts::unfit(meta.proto_min, meta.proto_max).is_none()
        && matches!(hosts::probe(meta, ORPHAN_ANSWER), Probed::Answered(_))
}

pub(super) fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
