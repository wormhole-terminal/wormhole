//! The first run as a program in a terminal (`Wormhole welcome`, core's `onboarding::welcome`), the Swift
//! app's side: what the GPUI app keeps in `onboarding/welcome/`. The program's choices come over the
//! control socket as `Action::Welcome`; core's `Registry` decides who may ask (a token only that terminal has,
//! while its welcome runs) and these files do what passes: the look, what reaches the person, an agent's hooks,
//! the example band, the import and the end of the first run. Budgets and the token rule are core's.
//!
//! Swift starts a welcome terminal with [`HookServer::begin_welcome`] (it hands back the token for that
//! terminal's environment alone) and hears what only it can do as `HookEvent`s: the example band, the end.

mod band;
mod facts;
mod ops;

pub use band::{
    WelcomeDemoRow, WelcomeDemoState, welcome_demo_label, welcome_demo_next_ms, welcome_demo_rows,
    welcome_demo_title,
};

use std::sync::{Arc, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use wormhole_core::ipc::control::{AgentsFacts, ErrorCode, Limited, PermissionFact, Reply, WelcomeOp};
use wormhole_core::onboarding::intro::Handover;
use wormhole_core::onboarding::welcome::{Registry, facts as scan, new_token};

use super::HookServer;
use super::core::Core;
use super::events::HookEvent;
use crate::settings::FontFamilyResolver;

/// Whether the start animation plays around this welcome, and since when.
#[derive(Clone, Copy)]
enum Play {
    /// None plays: the program has nothing to wait for.
    Never,
    /// About to: a request that comes in before it starts waits for the whole run.
    Pending,
    Since(Instant),
}

/// A theme an import saved in the profile's themes folder, which the look page lists beside the ones it offers.
#[derive(Clone)]
struct ImportedTheme {
    name: String,
    /// What `settings.theme` is set to for it: the theme file's path.
    value: String,
    light: bool,
}

/// One welcome terminal's side in the app.
struct Run {
    tab: i64,
    handover: Handover,
    first_run: bool,
    play: Play,
    reduce_motion: bool,
    /// The agents found; `None` until the search (off the dispatch thread) is done.
    agents: Option<AgentsFacts>,
    /// What went wrong with the last Install, until the next one that works.
    problem: Option<String>,
    imported: Option<ImportedTheme>,
    import_home: Option<String>,
    fonts: Arc<dyn FontFamilyResolver>,
}

/// Every welcome the app has running, and the doors to them.
pub(crate) struct Welcomes {
    gates: Registry,
    runs: Vec<Run>,
    /// What macOS says of banners, as Swift last told it.
    permission: PermissionFact,
}

impl Default for Welcomes {
    fn default() -> Self {
        Self { gates: Registry::default(), runs: Vec::new(), permission: PermissionFact::Checking }
    }
}

impl Core {
    fn welcomes(&self) -> MutexGuard<'_, Welcomes> {
        self.welcome.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The welcome request of `from`, answered (after the gate: these are not behind `control_write`, the token
    /// is the key).
    pub(crate) fn welcome(self: &Arc<Self>, token: &str, op: WelcomeOp, from: Option<i64>) -> Reply {
        if let Err(error) = self.welcomes().gates.authorize(from, token, &op) {
            return error.into();
        }
        let Some(tab) = from else { return Reply::error(ErrorCode::Refused, "outside any terminal") };
        let kind = match &op {
            WelcomeOp::State => None,
            WelcomeOp::SetLook { .. } => Some(Limited::WelcomeLook),
            WelcomeOp::SetNotify { .. } => Some(Limited::WelcomeNotify),
            WelcomeOp::SetShown { .. } => Some(Limited::WelcomeShown),
            WelcomeOp::InstallHooks { .. } => Some(Limited::WelcomeInstall),
            WelcomeOp::Demo { .. } => Some(Limited::WelcomeDemo),
            WelcomeOp::Import { .. } => Some(Limited::WelcomeImport),
            WelcomeOp::Finish { .. } => Some(Limited::WelcomeFinish),
        };
        if let Some(kind) = kind
            && let Err(reply) = self.budget(kind, from)
        {
            return reply;
        }
        tracing::debug!(tab, op = op.name(), "welcome request");
        ops::answer(self, tab, op)
    }
}

/// What a welcome is to be.
#[derive(uniffi::Record)]
pub struct WelcomePlan {
    /// The hand-over of the start animation: `cursor`, `portal` or `logo`.
    pub handover: String,
    /// The first run, which finishing is remembered of, or a replay.
    pub first_run: bool,
    /// The start animation is about to play around the window: the program waits for the ship to dock.
    pub animates: bool,
    pub reduce_motion: bool,
    /// Where an `import` looks for the other terminal's files (`None`: this Mac's own home).
    pub import_home: Option<String>,
}

#[uniffi::export]
impl HookServer {
    /// A welcome starts in terminal `tab` (`handover`: `cursor`, `portal` or `logo`): its door and its run, and the
    /// token for that terminal's environment alone (`Terminal.start_for_welcome`). `animates` says the start
    /// animation is about to play around the window (the program waits for the ship to dock); `import_home`
    /// is where an `import` looks for the other terminal's files (`None`: this Mac's own home). `None` when
    /// no random token could be had: the terminal is then a plain one.
    pub fn begin_welcome(
        &self,
        tab: i64,
        plan: WelcomePlan,
        fonts: Arc<dyn FontFamilyResolver>,
    ) -> Option<String> {
        let token = new_token()?;
        let handover = Handover::parse(&plan.handover).unwrap_or_default();
        let play = if plan.animates { Play::Pending } else { Play::Never };
        {
            let mut welcomes = self.core.welcomes();
            welcomes.gates.open(tab, token.clone());
            welcomes.runs.retain(|run| run.tab != tab);
            welcomes.runs.push(Run {
                tab,
                handover,
                first_run: plan.first_run,
                play,
                reduce_motion: plan.reduce_motion,
                agents: None,
                problem: None,
                imported: None,
                import_home: plan.import_home,
                fonts,
            });
        }
        // A few hundred `stat`s and small reads: off the dispatch thread, so the page never waits for them.
        let core = self.core.clone();
        let scanning = std::thread::Builder::new().name("welcome-agents".into()).spawn(move || {
            let agents = scan::scan_agents(&wormhole_core::agents::integrations::home());
            if let Some(run) = core.welcomes().run_mut(tab) {
                run.agents = Some(agents);
            }
        });
        if scanning.is_err() {
            tracing::warn!("no thread to look for the coding agents for the welcome");
        }
        Some(token)
    }

    /// The start animation really started (`playing`) or is over, cut short or not: the program's wait counts
    /// from here.
    pub fn welcome_playing(&self, tab: i64, playing: bool) {
        if let Some(run) = self.core.welcomes().run_mut(tab) {
            run.play = if playing { Play::Since(Instant::now()) } else { Play::Never };
        }
    }

    /// What macOS says of banners now (`checking`, `not_asked`, `allowed`, `denied`, `outside`): what the
    /// notifications page shows. `HookEvent::WelcomeAskPermission` asked for the prompt; this is its answer.
    pub fn set_notification_permission(&self, state: String) {
        let fact = match state.as_str() {
            "not_asked" => PermissionFact::NotAsked,
            "allowed" => PermissionFact::Allowed,
            "denied" => PermissionFact::Denied,
            "outside" => PermissionFact::Outside,
            _ => PermissionFact::Checking,
        };
        self.core.welcomes().permission = fact;
    }

    /// The terminal of a welcome is gone (closed): its token opens nothing. Whether a first run was still
    /// running in it, which counts as skipped (the first run is remembered).
    pub fn welcome_gone(&self, tab: i64) -> bool {
        let mut welcomes = self.core.welcomes();
        let unfinished =
            welcomes.run_mut(tab).is_some_and(|run| run.first_run) && welcomes.gates.running(tab);
        welcomes.gates.forget(tab);
        welcomes.runs.retain(|run| run.tab != tab);
        drop(welcomes);
        if unfinished {
            ops::remember_first_run(&self.core);
        }
        unfinished
    }

    /// Whether the terminal welcome is on (the feature flag `terminal_welcome`).
    pub fn welcome_available(&self) -> bool {
        self.core.settings().feature_enabled(wormhole_core::config::features::FeatureFlag::TerminalWelcome)
    }
}

impl Welcomes {
    fn run_mut(&mut self, tab: i64) -> Option<&mut Run> {
        self.runs.iter_mut().find(|run| run.tab == tab)
    }

    fn run(&self, tab: i64) -> Option<&Run> {
        self.runs.iter().find(|run| run.tab == tab)
    }
}

/// How long the program waits before it draws: the rest of the run's wait for the ship to dock, if the run
/// is still playing.
fn wait(run: &Run) -> Duration {
    let playing = match run.play {
        Play::Never => None,
        Play::Pending => Some(Duration::ZERO),
        Play::Since(at) => Some(at.elapsed()),
    };
    run.handover.wait(playing, run.reduce_motion)
}
