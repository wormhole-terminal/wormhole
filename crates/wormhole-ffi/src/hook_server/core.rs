//! What the server's threads share: the workspace and the terminals it holds, the desk that asks the
//! person, the bus the streams listen on, the budgets and the queue of what Swift is to do.

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use wormhole_core::config::features::FeatureFlag;
use wormhole_core::config::{AgentAccess, Config, Settings};
use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::{Limited, Listing, RateLimits, Reply, WindowInfo};
use wormhole_core::storage::paths::{DataRoot, ProfilePaths};

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

/// What a profile's requests are judged by: its settings and the approvals its desk holds. A profile is a trust
/// domain (`agent-permissions.md`, section 9), so a request is judged by the profile of the window it reaches,
/// never by the one that happens to be shown.
pub(crate) struct Seat {
    pub profile: ProfilePaths,
    pub desk: Arc<AccessDesk>,
    settings: Mutex<Option<(Instant, Arc<Settings>)>>,
    /// The mode last settled (`None` inside: Automation off), to act on a change of it once (`Core::settings_changed`).
    applied: Mutex<Option<Option<AgentAccess>>>,
}

impl Seat {
    fn new(desk: Arc<AccessDesk>) -> Self {
        // The mode at launch is what a later edit of `config.json` is measured against: looked at now, not at the
        // first request, and also while the Automation flag is off, so a widening made together with turning the
        // flag on is still asked about.
        desk.mode_in_force(Config::load_for(desk.profile()).settings.agent_access);
        Self { profile: desk.profile().clone(), desk, settings: Mutex::new(None), applied: Mutex::new(None) }
    }

    /// The profile's settings, as they were a moment ago at most.
    pub(crate) fn settings(&self) -> Arc<Settings> {
        let mut held = self.settings.lock().unwrap_or_else(PoisonError::into_inner);
        let read = match held.as_ref() {
            Some((read, settings)) if read.elapsed() < SETTINGS_TTL => settings.clone(),
            _ => {
                let settings = Arc::new(Config::load_for(&self.profile).settings);
                *held = Some((Instant::now(), settings.clone()));
                settings
            }
        };
        drop(held);
        // The mode in force, not the file's: a widening nobody in Wormhole made waits for the person's answer.
        if !read.feature_enabled(FeatureFlag::Automation) {
            return read;
        }
        match self.desk.mode_in_force(read.agent_access) {
            mode if mode == read.agent_access => read,
            mode => Arc::new(Settings { agent_access: mode, ..(*read).clone() }),
        }
    }

    /// What names an approval for the bus: whether it is stored, the profile (30 bits of its name's hash) and
    /// its id there, so two profiles' rows, and a row and a session approval, never share one.
    pub(crate) fn holder(&self, stored: bool, id: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.profile.id().hash(&mut hasher);
        let scope = hasher.finish() & 0x3FFF_FFFF;
        (u64::from(stored) << 63) | (scope << 32) | (id & 0xFFFF_FFFF)
    }
}

pub(crate) struct Core {
    /// Kept so that the workspace's pump thread lives as long as the server: it stops with the object.
    _workspace: Arc<Workspace>,
    pub shared: Arc<Shared>,
    pub usage: Arc<UsageService>,
    /// One per profile that has a desk; the first is the one the server started with (the fallback).
    seats: Mutex<Vec<Arc<Seat>>>,
    /// The seat of the request being answered: the dispatch thread answers one at a time, in order.
    reached: Mutex<Option<Arc<Seat>>>,
    pub bus: Arc<Bus>,
    /// One publisher per window (the first is the one the server started over); `attach` adds the rest.
    publishers: Mutex<Vec<Arc<Publisher>>>,
    /// The id of the window the server was started over, as `list` tells it.
    pub window: i64,
    pub ui: Ui,
    budgets: Mutex<RateLimits>,
    /// The data folder the server serves: where the Probes are.
    pub(crate) root: DataRoot,
    /// Who was found behind a process lately, and what was read of programs (`gate`).
    pub(super) identities: Mutex<super::gate::Identities>,
    pub(super) signatures: Mutex<Signatures>,
    /// A close question is up (`close`): one at a time, until Swift says it is over.
    pub(super) close_question: AtomicBool,
    /// The Probes that are on and the feed their hooks hear (`probes`).
    pub(crate) probes: crate::probes::Probes,
    /// The terminal welcomes that run (`welcome`).
    pub(super) welcome: Mutex<super::welcome::Welcomes>,
}

impl Core {
    pub(crate) fn new(
        workspace: Arc<Workspace>,
        usage: Arc<UsageService>,
        desk: Arc<AccessDesk>,
        bus: Arc<Bus>,
        publisher: Arc<Publisher>,
        root: DataRoot,
        listener: Arc<dyn HookListener>,
    ) -> Arc<Self> {
        let shared = workspace.shared().clone();
        let window = shared.window_id();
        Arc::new(Self {
            _workspace: workspace,
            shared,
            usage,
            seats: Mutex::new(vec![Arc::new(Seat::new(desk))]),
            reached: Mutex::new(None),
            bus,
            publishers: Mutex::new(vec![publisher]),
            window,
            ui: Ui::new(listener),
            budgets: Mutex::default(),
            root,
            identities: Mutex::default(),
            signatures: Mutex::default(),
            close_question: AtomicBool::new(false),
            probes: crate::probes::Probes::default(),
            welcome: Mutex::default(),
        })
    }

    /// Every window's workspace state, the first (the one the server was started over) among them: reading and
    /// typing, hooks and notices reach the terminals of all of them.
    pub(crate) fn shares(&self) -> Vec<Arc<Shared>> {
        let all = self.shared.windows().map(|(windows, _)| windows.shares()).unwrap_or_default();
        if all.is_empty() { vec![self.shared.clone()] } else { all }
    }

    /// Hear of window `shared`'s terminals too (its publisher, as the first window's): called for every window
    /// that exists when the server starts and for each one made after.
    pub(crate) fn attach(&self, shared: &Arc<Shared>) {
        if shared.tap.get().is_some() {
            return;
        }
        match Publisher::start(shared, self.bus.clone(), shared.window_id()) {
            Ok(publisher) if shared.tap.set(publisher.clone()).is_ok() => {
                self.publishers.lock().unwrap_or_else(PoisonError::into_inner).push(publisher);
            }
            Ok(_) => {}
            Err(err) => tracing::warn!("a window's events are not told: {err}"),
        }
    }

    fn publishers(&self) -> Vec<Arc<Publisher>> {
        let mut held = self.publishers.lock().unwrap_or_else(PoisonError::into_inner);
        held.retain(|publisher| publisher.is_alive());
        held.clone()
    }

    /// `list` as it reads now: every window, the server's own the front one.
    pub(crate) fn listing(&self) -> Listing {
        self.gather(Publisher::listing)
    }

    /// [`Self::listing`] and the baseline each window's next events are told against.
    pub(crate) fn listing_and_rebase(&self) -> Listing {
        self.gather(Publisher::listing_and_rebase)
    }

    fn gather(&self, read: fn(&Publisher) -> Listing) -> Listing {
        let windows =
            self.publishers().iter().flat_map(|publisher| read(publisher).windows).collect::<Vec<_>>();
        Listing {
            windows: windows.into_iter().map(|w| WindowInfo { front: w.id == self.window, ..w }).collect(),
        }
    }

    /// Look at every window again soon (`Publisher::kick`).
    pub(crate) fn kick(&self) {
        self.publishers().iter().for_each(|publisher| publisher.kick());
    }

    /// The state of window `id`, while it exists.
    pub(crate) fn share_of_window(&self, id: i64) -> Option<Arc<Shared>> {
        self.shares().into_iter().find(|shared| shared.window_id() == id)
    }

    /// The window state that holds terminal `id`.
    pub(crate) fn owner_of(&self, id: i64) -> Option<Arc<Shared>> {
        self.shares().into_iter().find(|shared| shared.peek_model(|model| model.ids().contains(&id)))
    }

    /// Another profile's desk: what its windows' requests are judged by from now on.
    pub(crate) fn add_desk(&self, desk: Arc<AccessDesk>) {
        let mut seats = self.seats.lock().unwrap_or_else(PoisonError::into_inner);
        seats.retain(|seat| seat.profile != *desk.profile());
        seats.push(Arc::new(Seat::new(desk)));
    }

    /// The seat of profile `id`, when it has one.
    pub(crate) fn seat_of_profile(&self, id: &str) -> Option<Arc<Seat>> {
        let seats = self.seats.lock().unwrap_or_else(PoisonError::into_inner);
        seats.iter().find(|seat| seat.profile.id().as_str() == id).cloned()
    }

    pub(crate) fn seats(&self) -> Vec<Arc<Seat>> {
        self.seats.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The seat window `shared` is judged by: its profile's, else (no desk made for it, or a window of no
    /// registry) the server's own.
    pub(crate) fn seat_of_window(&self, shared: &Shared) -> Arc<Seat> {
        let profile = shared.windows().map(|(windows, _)| windows.profile().id().clone());
        profile.and_then(|id| self.seat_of_profile(id.as_str())).unwrap_or_else(|| self.first_seat())
    }

    fn first_seat(&self) -> Arc<Seat> {
        self.seats.lock().unwrap_or_else(PoisonError::into_inner)[0].clone()
    }

    /// Judge what comes next by `seat` until [`Self::done`]: the seat of the window the request reaches.
    pub(crate) fn reaches(&self, seat: Arc<Seat>) {
        *self.reached.lock().unwrap_or_else(PoisonError::into_inner) = Some(seat);
    }

    pub(crate) fn done(&self) {
        *self.reached.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// The seat in force: the request's while one is answered, else the shown profile's.
    pub(crate) fn seat(&self) -> Arc<Seat> {
        let reached = self.reached.lock().unwrap_or_else(PoisonError::into_inner).clone();
        reached.unwrap_or_else(|| {
            let shown = crate::profile::active_in(&self.root);
            self.seat_of_profile(shown.id().as_str()).unwrap_or_else(|| self.first_seat())
        })
    }

    /// The settings in force (`seat`).
    pub(crate) fn settings(&self) -> Arc<Settings> {
        self.seat().settings()
    }

    /// The desk in force (`seat`).
    pub(crate) fn desk(&self) -> Arc<AccessDesk> {
        self.seat().desk.clone()
    }

    /// Profile `id`'s settings changed: read them again at the next request, and end the streams that only
    /// "Allowed while open" let read every terminal when the mode is something else now.
    pub(crate) fn settings_changed(&self, id: &str) {
        let Some(seat) = self.seat_of_profile(id) else { return };
        *seat.settings.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let settings = seat.settings();
        let reads_all =
            settings.feature_enabled(FeatureFlag::Automation) && settings.agent_access == AgentAccess::Open;
        if !reads_all {
            self.bus.revoke_reads();
        }
        // What the mode means for what waits and what is open (`agent_access/ask.rs` `settle_mode`), once per change.
        let mode = settings.feature_enabled(FeatureFlag::Automation).then_some(settings.agent_access);
        if seat.applied.lock().unwrap_or_else(PoisonError::into_inner).replace(mode) == Some(mode) {
            return;
        }
        match mode {
            Some(AgentAccess::Ask) => {}
            Some(AgentAccess::Open) => seat.desk.settle_questions(mode),
            Some(AgentAccess::Off) | None => {
                let (held, stored) = seat.desk.approval_ids();
                for id in held {
                    self.bus.revoke_holder(seat.holder(false, id));
                }
                for id in stored {
                    self.bus.revoke_holder(seat.holder(true, id));
                }
                seat.desk.settle_questions(mode);
            }
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
