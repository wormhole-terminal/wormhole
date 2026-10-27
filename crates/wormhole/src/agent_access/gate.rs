//! The gate every control request passes before a handler sees it.

use std::time::{Duration, Instant, SystemTime};

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{
    Access, Allowance, CapSet, Capability, Changed, Identity, Need, Resolved, Scope, Verdict, decide, resolve,
};
use wormhole_core::ipc::control::doctor::{Access as DoctorAccess, Standing};
use wormhole_core::ipc::control::{Action, Error, ErrorCode, Reason, Reply, Request};
use wormhole_core::ipc::{AskJob, Peer};
use wormhole_core::platform::process;
use wormhole_core::view::notice::Driver;

use super::change::reconcile;
use super::drive::{drive_of, driver_of};
use super::processes::System;
use super::profile::{Reach, holder, reach, reach_front};
use super::scopes::{opens, own_terminals, touched, visible_terminals};
use super::{ask, state};
use crate::app_state::{app, app_mut};
use crate::settings::settings_in;

/// A request that is let through, and what the app does besides carrying it out.
pub(crate) struct Admit {
    /// What it may do beyond what needs no approval.
    pub access: Access,
    /// Who is asking, when the app had reason to find out (the marker on the terminal it acts on says so).
    pub driver: Option<Driver>,
    /// The program, when the request opens terminals: what it opens is its own (`scopes`).
    pub opener: Option<Identity>,
    /// For a stream of what terminals print: the terminals it may hear the text of, which are the ones in the
    /// program's scope. `None` when the access decides (Allowed while open, a request with no such stream).
    pub readable: Option<Vec<i64>>,
}

impl Admit {
    fn of(access: Access) -> Self {
        Self { access, driver: None, opener: None, readable: None }
    }
}

/// What becomes of a request.
pub(crate) enum Gate {
    /// Let it through.
    Open(Admit),
    /// Answer it with this and nothing more.
    Reply(Reply),
    /// It waits for the user: the socket holds its connection until the question is answered.
    Ask(AskJob),
}

/// How long who is behind a process is remembered: `wh mcp` is one process that asks again and again.
const IDENTITY_TTL: Duration = Duration::from_secs(5);
const IDENTITIES_MAX: usize = 16;

/// Judge `request` from `peer`, by the profile of the window it reaches. With the Automation feature off
/// nothing changes from the release: typing and opening go by `control_write`, and the actions of the feature
/// answer `unsupported` themselves.
pub(crate) fn gate(request: &Request, peer: &Peer, cx: &mut App) -> Gate {
    gate_with(request, peer, &identify, cx)
}

/// [`gate`] with the finding out of who asks given (a test's).
pub(super) fn gate_with(
    request: &Request,
    peer: &Peer,
    who: &dyn Fn(&Peer, &mut App) -> Result<Resolved, Changed>,
    cx: &mut App,
) -> Gate {
    let reach = reach(request, cx);
    let settings = settings_in(&reach.profile, cx);
    if !settings.feature_enabled(FeatureFlag::Automation) {
        return Gate::Open(Admit::of(Access { read: false, write: settings.control_write, holder: None }));
    }
    let mode = reconcile(&reach.profile, cx);
    let need = Need::of(&request.action);
    let own = need == Need::Capability(Capability::Read) && reads_only_own(request, peer, cx);
    match decide(mode, need, own, None, false) {
        Verdict::Allow(access) => {
            let driver = driver_when_shown(request, peer, access, &reach, who, cx);
            // Allowed while open asks nothing, but what a request opens is still its program's own, for the
            // day the mode is Ask again.
            let opener = (mode == AgentAccess::Open && opens(&request.action))
                .then(|| who(peer, cx).ok().map(|resolved| resolved.identity))
                .flatten();
            return Gate::Open(Admit { access, driver, opener, readable: None });
        }
        Verdict::Refuse(error) => return Gate::Reply(error.into()),
        // Only in Ask mode, and nothing is held yet: who is it?
        Verdict::Ask => {}
    }
    let resolved = match who(peer, cx) {
        Ok(resolved) => resolved,
        Err(Changed) => {
            return Gate::Reply(
                Error::new(
                    ErrorCode::Failed,
                    "the program asking changed while Wormhole looked at it: try the request once more",
                )
                .because(Reason::IdentityChanged)
                .into(),
            );
        }
    };
    let now = Instant::now();
    let held = allowance(&resolved, &reach, now, cx);
    let cooling = state(cx).grants_of(reach.scope).cooling(&resolved.identity, now);
    let driver = driver_of(&resolved);
    match decide(mode, need, false, held, cooling) {
        Verdict::Allow(access) => within_scope(request, peer, resolved, access, driver, &reach, cx),
        Verdict::Refuse(error) => Gate::Reply(error.into()),
        Verdict::Ask => ask_the_user(resolved, need, request.ask, None, &reach, cx),
    }
}

/// The program is let in: does the request stay within the terminals it may use (the ones it opened, the one it
/// runs in, the ones the user opened up for it)? A terminal that is not among them is asked about, one at a
/// time, and a terminal hidden from agents is not a terminal here at all (it is not found, so there is nothing
/// to ask about, and the handler says what it says for an id that does not exist).
fn within_scope(
    request: &Request,
    peer: &Peer,
    resolved: Resolved,
    access: Access,
    driver: Driver,
    reach: &Reach,
    cx: &mut App,
) -> Gate {
    let own = own_terminals(peer, cx);
    let touched = touched(request, peer, cx);
    #[cfg(test)]
    let touched = if state(cx).unscoped { Vec::new() } else { touched };
    let out = state(cx).scopes.out_of_scope(&resolved.identity, resolved.process, &own, &touched);
    if let Some(&(terminal, capability)) = out.first() {
        if state(cx).scopes.cooling(&resolved.identity, terminal, Instant::now()) {
            return Gate::Reply(declined_recently(terminal).into());
        }
        return ask_the_user(resolved, Need::Capability(capability), request.ask, Some(terminal), reach, cx);
    }
    // A stream of output hears the terminals in scope, and, when it names none, those it opens later are not
    // among them (it would have to ask again).
    let hears_output = matches!(&request.action, Action::Events(_))
        && Need::of(&request.action) == Need::Capability(Capability::Read);
    let readable = hears_output.then(|| {
        let visible = visible_terminals(peer, cx);
        state(cx).scopes.readable(&resolved.identity, resolved.process, &own, visible)
    });
    let opener = opens(&request.action).then(|| resolved.identity.clone());
    Gate::Open(Admit { access, driver: Some(driver), opener, readable })
}

fn declined_recently(terminal: i64) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user said no about terminal {terminal} a moment ago; it is not asked again for two minutes: do not retry, ask the user"),
    )
    .because(Reason::DeclinedRecently)
}

/// Who is behind `peer`, from the system (and from the last few seconds' answer for the same process).
fn identify(peer: &Peer, cx: &mut App) -> Result<Resolved, Changed> {
    let head = peer.pid.and_then(|pid| process::start_time(pid).map(|started| (pid, started)));
    let now = Instant::now();
    if let Some((pid, started)) = head
        && let Some((_, _, _, resolved)) = state(cx).identities.iter().find(|(cached, at, seen, _)| {
            *cached == pid && *at == started && now.duration_since(*seen) < IDENTITY_TTL
        })
    {
        return Ok(resolved.clone());
    }
    let chain: Vec<u32> = peer.pid.into_iter().chain(peer.ancestors.iter().copied()).collect();
    let shells: Vec<u32> = app(cx)
        .windows
        .iter()
        .filter_map(|window| window.workspace.read_with(cx, |ws, cx| ws.control_shell_pids(cx)).ok())
        .flatten()
        .collect();
    let system = System::new(std::mem::take(&mut state(cx).signatures), shells);
    let resolved = resolve(&chain, &system);
    state(cx).signatures = system.into_signatures();
    let resolved = resolved?;
    if let Some((pid, started)) = head {
        let identities = &mut state(cx).identities;
        identities.retain(|(_, _, seen, _)| now.duration_since(*seen) < IDENTITY_TTL);
        if identities.len() >= IDENTITIES_MAX {
            identities.remove(0);
        }
        identities.push((pid, started, now, resolved.clone()));
    }
    Ok(resolved)
}

/// What covers `resolved`: an approval held in memory, else a stored one (its use counted, and written at most
/// once a minute).
fn allowance(resolved: &Resolved, reach: &Reach, now: Instant, cx: &mut App) -> Option<Allowance> {
    let scope = reach.scope;
    if let Some(held) = state(cx).grants_of(scope).find(&resolved.identity, resolved.process) {
        return Some(Allowance { caps: held.caps, holder: Some(holder(false, scope, held.id)) });
    }
    let row = stored(resolved, &reach.profile, cx)?;
    *state(cx).uses.entry((scope, row.id)).or_default() += 1;
    let due = state(cx).touched.get(&(scope, row.id)).is_none_or(|at| now.duration_since(*at) >= TOUCH_EVERY);
    if due {
        state(cx).touched.insert((scope, row.id), now);
        let uses = state(cx).uses.remove(&(scope, row.id)).unwrap_or(0);
        let location =
            resolved.program.as_ref().map(|p| p.path.to_string_lossy().into_owned()).unwrap_or_default();
        let cdhash = resolved.program.as_ref().and_then(|p| p.signature.cdhash.clone()).unwrap_or_default();
        let seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
        if let Some(store) = app_mut(cx).store_of_mut(&reach.profile)
            && let Err(err) = store.touch_agent_grant(row.id, seconds, &location, &cdhash, uses)
        {
            tracing::warn!("couldn't note that an approval was used: {err:#}");
        }
    }
    let holder = holder(true, scope, u64::try_from(row.id).unwrap_or(0));
    Some(Allowance { caps: CapSet::from_bits(row.caps), holder: Some(holder) })
}

/// How often a stored approval's use is written.
const TOUCH_EVERY: Duration = Duration::from_secs(60);

/// The stored approval of `profile` that covers `resolved`.
fn stored(
    resolved: &Resolved,
    profile: &wormhole_core::storage::paths::ProfileId,
    cx: &App,
) -> Option<wormhole_core::storage::store::AgentGrantRow> {
    app(cx).store_of(profile)?.agent_grant(&resolved.identity.key()).ok().flatten()
}

/// Queue the question for `resolved`. A caller that waits (`ask`) is held until it is answered; one that
/// cannot is refused at once, and the question stays, so that the next attempt finds the answer.
fn ask_the_user(
    resolved: Resolved,
    need: Need,
    waits: bool,
    about: Option<i64>,
    reach: &Reach,
    cx: &mut App,
) -> Gate {
    let label = resolved.label.clone();
    if !reach.shown || app(cx).windows.iter().all(|window| window.hidden) {
        return Gate::Reply(
            approval_needed(&label, "Wormhole has no window of that profile to ask in").into(),
        );
    }
    let (waiter, job) = if waits {
        let (tx, job) = AskJob::channel();
        (Some(tx), Some(job))
    } else {
        (None, None)
    };
    if state(cx).queue.push(resolved, need, reach.scope, about, waiter).is_err() {
        return Gate::Reply(
            Error::new(ErrorCode::NotAllowed, "several questions are already waiting for the user")
                .because(Reason::TooManyQuestions)
                .into(),
        );
    }
    // Not inside the request's own handling: the sheet is put on a window, which may be busy.
    cx.defer(ask::pump);
    match job {
        Some(job) => Gate::Ask(job),
        None => Gate::Reply(
            approval_needed(
                &label,
                "the user has been asked in Wormhole's window; tell them, then try again",
            )
            .into(),
        ),
    }
}

fn approval_needed(label: &str, why: &str) -> Error {
    Error::new(ErrorCode::NotAllowed, format!("{label} has not been allowed to control Wormhole yet: {why}"))
        .because(Reason::ApprovalNeeded)
}

/// Who is acting, for the marker, when the request is let in without a look at who it is (everybody is let in,
/// or it needed no approval): only when the chip would be shown at all, since finding out costs a walk of the
/// process chain.
fn driver_when_shown(
    request: &Request,
    peer: &Peer,
    access: Access,
    reach: &Reach,
    who: &dyn Fn(&Peer, &mut App) -> Result<Resolved, Changed>,
    cx: &mut App,
) -> Option<Driver> {
    if access == Access::default() || drive_of(&request.action, request.from_tab()).is_none() {
        return None;
    }
    if !crate::notice::enabled(&reach.profile, cx) {
        return None;
    }
    who(peer, cx).ok().map(|resolved| driver_of(&resolved))
}

/// Whether `request` reads only terminals the caller runs inside of (by its place in the process tree,
/// never by what it says): that needs no approval.
fn reads_only_own(request: &Request, peer: &Peer, cx: &App) -> bool {
    let own = own_terminals(peer, cx);
    let from = request.from_tab();
    // A read that names no terminal and has none of its own to default to reads nothing: its handler says
    // what is wrong with it, which is no business of the user's.
    let target = match &request.action {
        Action::Read(spec) => spec.target.or(from.map(wormhole_core::ipc::control::Target::Tab)),
        Action::Blocks(spec) => spec.target.or(from.map(wormhole_core::ipc::control::Target::Tab)),
        Action::Wait(spec) => Some(spec.target),
        Action::Events(spec) => {
            return spec.filter().is_ok_and(|filter| {
                !filter.terminals.is_empty() && filter.terminals.iter().all(|id| own.contains(id))
            });
        }
        _ => return false,
    };
    target.is_none_or(|target| own.contains(&target.id()))
}

/// Where the caller stands, for `doctor`: the mode, who the app takes it for and what covers it, in the profile
/// of the window a request with no target acts in. Reads, never asks and never notes a use.
pub(crate) fn standing(peer: &Peer, cx: &mut App) -> DoctorAccess {
    let reach = reach_front(cx);
    let mode = reconcile(&reach.profile, cx);
    let resolved = identify(peer, cx).ok();
    let who = resolved.as_ref().map_or_else(|| "Scripts".to_string(), |resolved| resolved.label.clone());
    let standing = match (mode, &resolved) {
        (AgentAccess::Off, _) => Standing::Off,
        (AgentAccess::Open, _) => Standing::Open,
        (AgentAccess::Ask, None) => Standing::NotAsked,
        (AgentAccess::Ask, Some(resolved)) => {
            let now = Instant::now();
            if let Some(held) = state(cx).grants_of(reach.scope).find(&resolved.identity, resolved.process) {
                match held.scope {
                    Scope::Session(_) => Standing::Session,
                    Scope::UntilQuit => Standing::UntilQuit,
                }
            } else if stored(resolved, &reach.profile, cx).is_some() {
                Standing::Always
            } else if state(cx).grants_of(reach.scope).cooling(&resolved.identity, now) {
                Standing::DeclinedRecently
            } else {
                Standing::NotAsked
            }
        }
    };
    DoctorAccess { mode, who, standing }
}
