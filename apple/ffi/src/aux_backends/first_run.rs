//! The first run's backend (`onboarding/`, core `onboarding` and `agents::integrations`): the folders the first
//! step offers, the coding agents found and their hooks, the look the second step sets, and the flag that the
//! intro was seen. Nothing is installed, trusted or written until a method that asks for it is called (the
//! person pressed that button); a replay of the intro calls none of the writers.

use std::path::PathBuf;
use std::sync::Arc;

use wormhole_core::agents::integrations::{self, State};
use wormhole_core::agents::{AGENTS, Agent, installed};
use wormhole_core::config::Config;
use wormhole_core::ipc::access::agent_word;
use wormhole_core::onboarding::{self, ONBOARDED_KEY, PlaceKind};
use wormhole_core::platform::paths;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::Store;

use super::common::{AuxError, resolve_data_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PlaceKindRecord {
    Recent,
    Home,
    Desktop,
    Documents,
    Developer,
}

/// A folder the first step offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PlaceRecord {
    pub kind: PlaceKindRecord,
    pub name: String,
    pub path: String,
    /// Where it is, with the home folder as `~`.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HookState {
    NotInstalled,
    Partial,
    Installed,
    /// The agent has no hooks to write.
    Unsupported,
}

/// An agent found on `PATH`, as the third step lists it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FoundAgentRecord {
    pub id: String,
    pub name: String,
    pub hooks: HookState,
    /// The file the hooks go in (`~/.claude/settings.json`).
    pub path: Option<String>,
}

/// What `scan_agents` found: the agents there and how many others the registry knows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentScanRecord {
    pub found: Vec<FoundAgentRecord>,
    pub others: u32,
}

/// What the Look and Agents steps change (`OnboardingPreferences`).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct LookRecord {
    /// A `#rrggbb` the person picked; `None` keeps the theme's own primary.
    pub primary: Option<String>,
    pub font_family: String,
    pub font_size: f64,
    pub ligatures: bool,
    pub notifications: bool,
    pub notify_command_done: bool,
}

#[derive(uniffi::Object)]
pub struct FirstRun {
    data_dir: PathBuf,
    /// Where agents keep their hook files (`WORMHOLE_AGENT_HOME`, else the home folder).
    agent_home: PathBuf,
}

#[uniffi::export]
impl FirstRun {
    /// `agent_home`: where agents' config lives (a scratch folder in tests); `None` is core's rule.
    #[uniffi::constructor]
    pub fn new(agent_home: Option<String>, data_dir: Option<String>) -> Result<Arc<Self>, AuxError> {
        Ok(Arc::new(Self {
            data_dir: resolve_data_dir(data_dir)?,
            agent_home: agent_home.map_or_else(integrations::home, PathBuf::from),
        }))
    }

    /// The folders the first step lists: the recent roots that exist, then Home, Desktop, Documents and
    /// Developer when there is one, under `home` (the person's, when `None`).
    pub fn places(&self, home: Option<String>, recents: Vec<String>) -> Vec<PlaceRecord> {
        let home = home.map(PathBuf::from).or_else(paths::home).unwrap_or_default();
        let recents: Vec<PathBuf> = recents.into_iter().map(PathBuf::from).collect();
        onboarding::places(&home, &recents, |path| path.is_dir())
            .into_iter()
            .map(|place| PlaceRecord {
                kind: match place.kind {
                    PlaceKind::Recent => PlaceKindRecord::Recent,
                    PlaceKind::Home => PlaceKindRecord::Home,
                    PlaceKind::Desktop => PlaceKindRecord::Desktop,
                    PlaceKind::Documents => PlaceKindRecord::Documents,
                    PlaceKind::Developer => PlaceKindRecord::Developer,
                },
                name: place.name,
                path: place.path.display().to_string(),
                detail: place.detail,
            })
            .collect()
    }

    /// Every agent's command looked up on `PATH` and its hooks read: a few hundred `stat`s, so off the main
    /// thread.
    pub fn scan_agents(&self) -> AgentScanRecord {
        let mut scan = AgentScanRecord { found: Vec::new(), others: 0 };
        for agent in AGENTS {
            if installed(agent) {
                scan.found.push(self.row_of(agent));
            } else {
                scan.others += 1;
            }
        }
        scan
    }

    /// Writes the hooks of agent `id` (an `id` of a scan) and reads them back, so the row says what is there.
    pub fn install_hooks(&self, id: String) -> Result<FoundAgentRecord, AuxError> {
        let agent = AGENTS.iter().find(|a| agent_word(a.id) == id).ok_or_else(|| AuxError::failed("no such agent"))?;
        let spec = integrations::spec(agent.id).ok_or_else(|| AuxError::failed("this agent has no hooks"))?;
        spec.install(&self.agent_home)
            .map_err(|err| AuxError::failed(format!("Couldn't write the hooks: {err}")))?;
        Ok(self.row_of(agent))
    }

    /// Writes what the Look and Agents steps chose into the Default profile's `config.json`, through core's
    /// settings (clamped and sanitised as every write is).
    pub fn write_look(&self, look: LookRecord) {
        let profile = DataRoot::new(&self.data_dir).default_profile();
        Config::load_for(&profile).update(|settings| {
            settings.choose_primary(look.primary.unwrap_or_default());
            settings.font_family = look.font_family;
            settings.font_size = look.font_size as f32;
            settings.font_ligatures = look.ligatures;
            settings.notifications = look.notifications;
            settings.notify_command_done = look.notify_command_done;
        });
    }

    /// Whether the intro should show: a first launch that nothing was saved from and that never finished or
    /// skipped it (`saved_windows`: how many windows a saved session would restore).
    pub fn should_show(&self, saved_windows: u32) -> bool {
        onboarding::should_show(self.flag().as_deref(), usize::try_from(saved_windows).unwrap_or(usize::MAX))
    }

    /// The first run finished or was skipped: remember it. A replay does not call this.
    pub fn finish(&self) -> Result<(), AuxError> {
        let store = Store::open_at(&self.data_dir).map_err(AuxError::failed)?;
        store.set_meta(ONBOARDED_KEY, "1").map_err(AuxError::failed)
    }
}

impl FirstRun {
    fn flag(&self) -> Option<String> {
        Store::open_at(&self.data_dir).ok()?.meta(ONBOARDED_KEY).ok().flatten()
    }

    fn row_of(&self, agent: &Agent) -> FoundAgentRecord {
        let spec = integrations::spec(agent.id);
        let hooks = match spec.map(|spec| spec.state(&self.agent_home)) {
            None => HookState::Unsupported,
            Some(Ok(State::Installed)) => HookState::Installed,
            Some(Ok(State::Partial)) => HookState::Partial,
            // A file that did not read is shown as not installed, as the GPUI step does (`state: None`).
            Some(Ok(State::NotInstalled) | Err(_)) => HookState::NotInstalled,
        };
        FoundAgentRecord {
            id: agent_word(agent.id),
            name: agent.name.to_string(),
            hooks,
            path: spec.map(|spec| spec.display_path(&self.agent_home)),
        }
    }
}
