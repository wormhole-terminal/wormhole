//! What the server's threads share: the workspace and the terminals it holds, the desk that asks the
//! person, the bus the streams listen on, the budgets and the queue of what Swift is to do.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use wormhole_core::config::features::FeatureFlag;
use wormhole_core::config::{AgentAccess, Config, Settings};
use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::{Limited, RateLimits, Reply};
use wormhole_core::storage::paths::ProfilePaths;

use super::HookListener;
use super::events::HookEvent;
use super::processes::Signatures;
use super::tap::Publisher;
use crate::aux_backends::AccessDesk;
use crate::files_panel::UsageService;
use crate::workspace::{Shared, Workspace};

/// How long the settings read for a request are reused: a file read per request would be wasteful, and a
/// switch the person turns off must take effect within a moment.
const SETTINGS_TTL: Duration = Duration::from_secs(1);

pub(crate) struct Core {
    /// Kept so that the workspace's pump thread lives as long as the server: it stops with the object.
    _workspace: Arc<Workspace>,
    pub shared: Arc<Shared>,
    pub usage: Arc<UsageService>,
    pub desk: Arc<AccessDesk>,
    pub bus: Arc<Bus>,
    pub publisher: Arc<Publisher>,
    /// The window's id as `list` tells it: the Swift app has one window per workspace.
    pub window: i64,
    pub ui: Ui,
    budgets: Mutex<RateLimits>,
    profile: ProfilePaths,
    settings: Mutex<Option<(Instant, Arc<Settings>)>>,
    /// Who was found behind a process lately, and what was read of programs (`gate`).
    pub(super) identities: Mutex<super::gate::Identities>,
    pub(super) signatures: Mutex<Signatures>,
    /// A close question is up (`close`): one at a time, until Swift says it is over.
    pub(super) close_question: AtomicBool,
}

impl Core {
    pub(crate) fn new(
        workspace: Arc<Workspace>,
        usage: Arc<UsageService>,
        desk: Arc<AccessDesk>,
        bus: Arc<Bus>,
        publisher: Arc<Publisher>,
        profile: ProfilePaths,
        listener: Arc<dyn HookListener>,
    ) -> Arc<Self> {
        let shared = workspace.shared().clone();
        Arc::new(Self {
            _workspace: workspace,
            shared,
            usage,
            desk,
            bus,
            publisher,
            window: 1,
            ui: Ui::new(listener),
            budgets: Mutex::default(),
            profile,
            settings: Mutex::new(None),
            identities: Mutex::default(),
            signatures: Mutex::default(),
            close_question: AtomicBool::new(false),
        })
    }

    /// The profile's settings, as they were a moment ago at most.
    pub(crate) fn settings(&self) -> Arc<Settings> {
        let mut held = self.settings.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((read, settings)) = held.as_ref()
            && read.elapsed() < SETTINGS_TTL
        {
            return settings.clone();
        }
        let settings = Arc::new(Config::load_for(&self.profile).settings);
        *held = Some((Instant::now(), settings.clone()));
        settings
    }

    /// The settings changed: read them again at the next request, and end the streams that only "Allowed while
    /// open" let read every terminal when the mode is something else now.
    pub(crate) fn settings_changed(&self) {
        *self.settings.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let settings = self.settings();
        let reads_all =
            settings.feature_enabled(FeatureFlag::Automation) && settings.agent_access == AgentAccess::Open;
        if !reads_all {
            self.bus.revoke_reads();
        }
    }

    /// Count a `kind` request of `from` against its budget, or the reply that says when to try again.
    pub(crate) fn budget(&self, kind: Limited, from: Option<i64>) -> Result<(), Reply> {
        let now = Instant::now();
        let mut budgets = self.budgets.lock().unwrap_or_else(PoisonError::into_inner);
        if budgets.allow(kind, from, now) {
            return Ok(());
        }
        let wait = budgets.retry_after(kind, from, now).as_secs().max(1);
        Err(wormhole_core::ipc::control::Error::new(
            wormhole_core::ipc::control::ErrorCode::RateLimited,
            format!("too many requests of this kind lately; try again in {wait}s"),
        )
        .retry_after(wait)
        .into())
    }
}

/// What the server asks Swift to do: queued, with one wake until drained.
pub(crate) struct Ui {
    pending: Mutex<Vec<HookEvent>>,
    /// The latest gauge told for each terminal: a status line repeats itself many times a minute.
    gauges: Mutex<std::collections::HashMap<i64, HookEvent>>,
    woken: AtomicBool,
    listener: Arc<dyn HookListener>,
}

impl Ui {
    fn new(listener: Arc<dyn HookListener>) -> Self {
        Self { pending: Mutex::default(), gauges: Mutex::default(), woken: AtomicBool::new(false), listener }
    }

    pub(crate) fn push(&self, event: HookEvent) {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner).push(event);
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }

    /// Queue `gauge` (a `HookEvent::Gauge`) unless it says what the last one for its terminal did.
    pub(crate) fn push_gauge(&self, gauge: HookEvent) {
        let HookEvent::Gauge { terminal, .. } = &gauge else { return };
        let terminal = *terminal;
        let mut told = self.gauges.lock().unwrap_or_else(PoisonError::into_inner);
        if told.get(&terminal) == Some(&gauge) {
            return;
        }
        told.insert(terminal, gauge.clone());
        drop(told);
        {
            let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
            // Only the newest word of a terminal counts.
            pending
                .retain(|event| !matches!(event, HookEvent::Gauge { terminal: old, .. } if *old == terminal));
        }
        self.push(gauge);
    }

    pub(crate) fn drain(&self) -> Vec<HookEvent> {
        self.woken.store(false, Ordering::Release);
        std::mem::take(&mut *self.pending.lock().unwrap_or_else(PoisonError::into_inner))
    }
}
