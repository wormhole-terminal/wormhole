//! The gate every control request passes before a handler sees it.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::AgentAccess;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{
    Access, Allowance, CapSet, Capability, Changed, Need, Resolved, Scope, Verdict, decide, resolve,
};
use wormhole_core::ipc::control::doctor::{Access as Access2, Standing};
use wormhole_core::ipc::control::{Action, Error, ErrorCode, Reason, Reply, Request};
use wormhole_core::ipc::{AskJob, Peer};
use wormhole_core::view::notice::Driver;

use super::drive::{drive_of, driver_of};
use super::processes::System;
use super::{ask, state};
use crate::app_state::{app, app_mut};
use crate::settings::settings;

/// Stored approvals are told from the ones held in memory by the bit set in their holder number.
pub(super) const HOLDER_STORED: u64 = 1 << 62;

/// What becomes of a request.
pub(crate) enum Gate {
    /// Let it through, with what it may do beyond what needs no approval, and who is asking when the app
    /// had reason to find out (the marker on the terminal it acts on says so).
    Open(Access, Option<Driver>),
    /// Answer it with this and nothing more.
    Reply(Reply),
    /// It waits for the user: the socket holds its connection until the question is answered.
    Ask(AskJob),
}

/// Judge `request` from `peer`. With the Automation feature off nothing changes from the release: typing
/// and opening go by `control_write`, and the actions of the feature answer `unsupported` themselves.
pub(crate) fn gate(request: &Request, peer: &Peer, cx: &mut App) -> Gate {
    let settings = settings(cx);
    if !settings.feature_enabled(FeatureFlag::Automation) {
        return Gate::Open(Access { read: false, write: settings.control_write, holder: None }, None);
    }
    let mode = settings.agent_access;
    let need = Need::of(&request.action);
    let own = need == Need::Capability(Capability::Read) && reads_only_own(request, peer, cx);
    match decide(mode, need, own, None, false) {
        Verdict::Allow(access) => return Gate::Open(access, driver_when_shown(request, peer, access, cx)),
        Verdict::Refuse(error) => return Gate::Reply(error.into()),
        // Only in Ask mode, and nothing is held yet: who is it?
        Verdict::Ask => {}
    }
    let resolved = match identify(peer, cx) {
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
    let held = allowance(&resolved, now, cx);
    let cooling = state(cx).grants.cooling(&resolved.identity, now);
    let driver = driver_of(&resolved);
    match decide(mode, need, false, held, cooling) {
        Verdict::Allow(access) => Gate::Open(access, Some(driver)),
        Verdict::Refuse(error) => Gate::Reply(error.into()),
        Verdict::Ask => ask_the_user(resolved, need, request.ask, cx),
    }
}

/// Who is acting, for the marker, when the request is let in without a look at who it is (everybody is let in,
/// or it needed no approval): only when the chip would be shown at all, since finding out costs a walk of the
/// process chain.
fn driver_when_shown(request: &Request, peer: &Peer, access: Access, cx: &mut App) -> Option<Driver> {
    if access == Access::default() || drive_of(&request.action, request.from_tab()).is_none() {
        return None;
    }
    let profile = app(cx).active_profile().clone();
    if !crate::notice::enabled(&profile, cx) {
        return None;
    }
    identify(peer, cx).ok().map(|resolved| driver_of(&resolved))
}

/// Who is behind `peer`.
fn identify(peer: &Peer, cx: &mut App) -> Result<Resolved, Changed> {
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
    resolved
}

/// What covers `resolved`: an approval held in memory, else a stored one (noted as used, at most once a
/// minute).
fn allowance(resolved: &Resolved, now: Instant, cx: &mut App) -> Option<Allowance> {
    if let Some(held) = state(cx).grants.find(&resolved.identity, resolved.process) {
        return Some(Allowance { caps: held.caps, holder: Some(held.id) });
    }
    let profile = app(cx).active_profile().clone();
    let row = stored(resolved, cx)?;
    let due = state(cx).touched.get(&row.id).is_none_or(|at| now.duration_since(*at) >= TOUCH_EVERY);
    if due {
        state(cx).touched.insert(row.id, now);
        let location =
            resolved.program.as_ref().map(|p| p.path.to_string_lossy().into_owned()).unwrap_or_default();
        let cdhash = resolved.program.as_ref().and_then(|p| p.signature.cdhash.clone()).unwrap_or_default();
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
        if let Some(store) = app_mut(cx).store_of_mut(&profile)
            && let Err(err) = store.touch_agent_grant(row.id, seconds, &location, &cdhash)
        {
            tracing::warn!("couldn't note that an approval was used: {err:#}");
        }
    }
    let holder = HOLDER_STORED | u64::try_from(row.id).unwrap_or(0);
    Some(Allowance { caps: CapSet::from_bits(row.caps), holder: Some(holder) })
}

/// How often a stored approval's use is written.
const TOUCH_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Queue the question for `resolved`. A caller that waits (`ask`) is held until it is answered; one that
/// cannot is refused at once, and the question stays, so that the next attempt finds the answer.
fn ask_the_user(resolved: Resolved, need: Need, waits: bool, cx: &mut App) -> Gate {
    let label = resolved.label.clone();
    if app(cx).windows.iter().all(|window| window.hidden) {
        return Gate::Reply(approval_needed(&label, "Wormhole has no window to ask in").into());
    }
    let (waiter, job) = if waits {
        let (tx, job) = AskJob::channel();
        (Some(tx), Some(job))
    } else {
        (None, None)
    };
    if state(cx).queue.push(resolved, need, waiter).is_err() {
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

/// Whether `request` reads only terminals the caller runs inside of (by its place in the process tree,
/// never by what it says): that needs no approval.
fn reads_only_own(request: &Request, peer: &Peer, cx: &App) -> bool {
    let own: Vec<i64> = app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            window.workspace.read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx)).ok()
        })
        .flatten()
        .collect();
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

/// Whether `mode` lets programs in at all, for the tick: reads of every terminal end when it goes off.
pub(crate) fn lets_in(mode: AgentAccess) -> bool {
    mode != AgentAccess::Off
}

/// Where the caller stands, for `doctor`: the mode, who the app takes it for and what covers it. Reads, never
/// asks and never notes a use.
pub(crate) fn standing(peer: &Peer, cx: &mut App) -> Access2 {
    let mode = settings(cx).agent_access;
    let resolved = identify(peer, cx).ok();
    let who = resolved.as_ref().map_or_else(|| "Scripts".to_string(), |resolved| resolved.label.clone());
    let standing = match (mode, &resolved) {
        (AgentAccess::Off, _) => Standing::Off,
        (AgentAccess::Open, _) => Standing::Open,
        (AgentAccess::Ask, None) => Standing::NotAsked,
        (AgentAccess::Ask, Some(resolved)) => {
            let now = Instant::now();
            if let Some(held) = state(cx).grants.find(&resolved.identity, resolved.process) {
                match held.scope {
                    Scope::Session(_) => Standing::Session,
                    Scope::UntilQuit => Standing::UntilQuit,
                }
            } else if stored(resolved, cx).is_some() {
                Standing::Always
            } else if state(cx).grants.cooling(&resolved.identity, now) {
                Standing::DeclinedRecently
            } else {
                Standing::NotAsked
            }
        }
    };
    Access2 { mode, who, standing }
}

/// The stored approval that covers `resolved`.
fn stored(resolved: &Resolved, cx: &App) -> Option<wormhole_core::storage::store::AgentGrantRow> {
    let state = app(cx);
    state.store_of(state.active_profile())?.agent_grant(&resolved.identity.key()).ok().flatten()
}
