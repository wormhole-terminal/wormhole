//! The gate every control request passes before a handler sees it: the GPUI app's `agent_access/gate.rs`
//! over core's decision (`ipc::access`: the mode in Settings, who the program behind the `wh` is, what the
//! user allowed it, which terminals it reaches), with the Swift side's `AccessDesk` holding the approvals
//! and the questions. Nothing a caller says counts: who asks is found from the system, and no request can
//! change what the desk holds.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use wormhole_core::config::AgentAccess;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::{
    Access, Allowance, Capability, Changed, Identity, Need, ProcessRef, Resolved, Verdict, decide, resolve,
};
use wormhole_core::ipc::control::event::Topic;
use wormhole_core::ipc::control::{Action, Error, ErrorCode, Reason, Reply, Request, Target};
use wormhole_core::ipc::{AskJob, Peer};
use wormhole_core::platform::process;
use wormhole_core::view::notice::Driver;

use super::core::Core;
use super::drive::{drive_of, driver_of};
use super::processes::System;

/// How long who is behind a process is remembered: `wh mcp` is one process that asks again and again.
const IDENTITY_TTL: Duration = Duration::from_secs(5);
const IDENTITIES_MAX: usize = 16;

/// Who was found behind a process lately: its pid and start time, when it was looked at, the answer.
#[derive(Default)]
pub(crate) struct Identities(Vec<(u32, SystemTime, Instant, Resolved)>);

/// A request that is let through, and what the handlers need to know of it.
pub(crate) struct Admit {
    pub access: Access,
    /// Who is asking, when the request drives a terminal and the chip is on to say so (`drive`).
    pub driver: Option<Driver>,
    /// The program, when the request opens terminals: what it opens is its own.
    pub opener: Option<Identity>,
    /// For a stream of what terminals print: the ones it may hear the text of (its scope). `None` when the
    /// access decides.
    pub readable: Option<Vec<i64>>,
}

pub(crate) enum Gate {
    Open(Admit),
    Reply(Reply),
    /// It waits for the user: the socket holds its connection until the question is answered.
    Ask(AskJob),
}

/// Whether `action` opens terminals (a terminal, a window, a split): those become the program's own.
pub(crate) fn opens(action: &Action) -> bool {
    matches!(action, Action::NewTerminal { .. } | Action::NewWindow(_) | Action::Split(_))
}

/// Judge `request` from `peer`. With the Automation feature off nothing changes from the release: typing
/// and opening go by `control_write`, and the actions of the feature answer `unsupported` themselves.
pub(crate) fn gate(core: &Core, request: &Request, peer: &Peer) -> Gate {
    let seat = core.seat();
    let desk = &seat.desk;
    let settings = seat.settings();
    if !settings.feature_enabled(FeatureFlag::Automation) {
        return Gate::Open(Admit {
            access: Access { read: false, write: settings.control_write, holder: None },
            driver: None,
            opener: None,
            readable: None,
        });
    }
    let mode = settings.agent_access;
    let need = Need::of(&request.action);
    let own = need == Need::Capability(Capability::Read) && reads_only_own(core, request, peer);
    match decide(mode, need, own, None, false) {
        Verdict::Allow(access) => {
            let driver = driver_when_shown(core, request, peer, access);
            // Allowed while open asks nothing, but what a request opens is still its program's own, for
            // the day the mode is Ask again.
            let opener = (mode == AgentAccess::Open && opens(&request.action))
                .then(|| identify(core, peer).ok().map(|resolved| resolved.identity))
                .flatten();
            return Gate::Open(Admit { access, driver, opener, readable: None });
        }
        Verdict::Refuse(error) => return Gate::Reply(error.into()),
        Verdict::Ask => {}
    }
    let resolved = match identify(core, peer) {
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
    let held = allowance(core, &resolved);
    let cooling = desk.with_state(|grants, _| grants.cooling(&resolved.identity, now));
    match decide(mode, need, false, held, cooling) {
        Verdict::Allow(access) => within_scope(core, request, peer, resolved, access),
        Verdict::Refuse(error) => Gate::Reply(error.into()),
        Verdict::Ask => ask_the_user(core, resolved, need, request.ask, None),
    }
}

/// The program is let in: does the request stay within the terminals it may use (the ones it opened, the
/// one it runs in, the ones the user opened up for it)? One that is not among them is asked about, one at a
/// time; a terminal hidden from agents is not a terminal here at all (it is not found, so there is nothing
/// to ask about, and the handler says what it says for an id that does not exist).
fn within_scope(core: &Core, request: &Request, peer: &Peer, resolved: Resolved, access: Access) -> Gate {
    let desk = core.desk();
    let own = core.terminals_under(peer);
    let touched = touched(core, request, peer);
    let now = Instant::now();
    let (out, cooling) = desk.with_state(|_, scopes| {
        let out = scopes.out_of_scope(&resolved.identity, resolved.process, &own, &touched);
        let cooling = out.first().map(|&(terminal, _)| scopes.cooling(&resolved.identity, terminal, now));
        (out, cooling)
    });
    if let Some(&(terminal, capability)) = out.first() {
        if cooling == Some(true) {
            return Gate::Reply(declined_recently(terminal).into());
        }
        return ask_the_user(core, resolved, Need::Capability(capability), request.ask, Some(terminal));
    }
    // A stream of output hears the terminals in scope, and, when it names none, those it opens later are not
    // among them (it would have to ask again).
    let hears_output = matches!(&request.action, Action::Events(_))
        && Need::of(&request.action) == Need::Capability(Capability::Read);
    let readable = hears_output.then(|| {
        let visible = core.visible_ids(peer);
        desk.with_state(|_, scopes| scopes.readable(&resolved.identity, resolved.process, &own, visible))
    });
    let opener = opens(&request.action).then(|| resolved.identity.clone());
    let driver = Some(driver_of(&resolved));
    Gate::Open(Admit { access, driver, opener, readable })
}

/// Who is acting, for the chip, when the request is let in without a look at who it is (everybody is let in, or
/// it needed no approval): only when the chip would be shown at all, since finding out costs a walk of the
/// process chain.
fn driver_when_shown(core: &Core, request: &Request, peer: &Peer, access: Access) -> Option<Driver> {
    if access == Access::default() || drive_of(&request.action, request.from_tab()).is_none() {
        return None;
    }
    if !core.settings().feature_enabled(FeatureFlag::Notices) {
        return None;
    }
    identify(core, peer).ok().map(|resolved| driver_of(&resolved))
}

fn declined_recently(terminal: i64) -> Error {
    Error::new(
        ErrorCode::NotAllowed,
        format!("the user said no about terminal {terminal} a moment ago; it is not asked again for two minutes: do not retry, ask the user"),
    )
    .because(Reason::DeclinedRecently)
}

/// Who is behind `peer`, from the system (and from the last few seconds' answer for the same process).
pub(crate) fn identify(core: &Core, peer: &Peer) -> Result<Resolved, Changed> {
    let head = peer.pid.and_then(|pid| process::start_time(pid).map(|started| (pid, started)));
    let now = Instant::now();
    let mut identities = locked(&core.identities);
    if let Some((pid, started)) = head
        && let Some((_, _, _, resolved)) = identities.0.iter().find(|(cached, at, seen, _)| {
            *cached == pid && *at == started && now.duration_since(*seen) < IDENTITY_TTL
        })
    {
        return Ok(resolved.clone());
    }
    let chain: Vec<u32> = peer.pid.into_iter().chain(peer.ancestors.iter().copied()).collect();
    let system = System::new(std::mem::take(&mut *locked(&core.signatures)), core.shell_pids());
    let resolved = resolve(&chain, &system);
    *locked(&core.signatures) = system.into_signatures();
    let resolved = resolved?;
    if let Some((pid, started)) = head {
        identities.0.retain(|(_, _, seen, _)| now.duration_since(*seen) < IDENTITY_TTL);
        if identities.0.len() >= IDENTITIES_MAX {
            identities.0.remove(0);
        }
        identities.0.push((pid, started, now, resolved.clone()));
    }
    Ok(resolved)
}

/// What covers `resolved`: an approval held in memory, else a stored one.
pub(crate) fn allowance(core: &Core, resolved: &Resolved) -> Option<Allowance> {
    let seat = core.seat();
    let desk = &seat.desk;
    let held = desk.with_state(|grants, _| {
        grants.find(&resolved.identity, resolved.process).map(|held| (held.caps, held.id))
    });
    if let Some((caps, id)) = held {
        return Some(Allowance { caps, holder: Some(seat.holder(false, id)) });
    }
    let caps = desk.stored_caps(&resolved.identity.key())?;
    Some(Allowance { caps, holder: Some(seat.holder(true, 0)) })
}

/// Queue the question for `resolved`. A caller that waits (`ask`) is held until it is answered; one that
/// cannot is refused at once, and the question stays, so that the next attempt finds the answer.
fn ask_the_user(core: &Core, resolved: Resolved, need: Need, waits: bool, about: Option<i64>) -> Gate {
    let desk = core.desk();
    let label = resolved.label.clone();
    let (waiter, job) = if waits {
        let (tx, job) = AskJob::channel();
        (Some(tx), Some(job))
    } else {
        (None, None)
    };
    if desk.ask(resolved, need, about, waiter).is_err() {
        return Gate::Reply(
            Error::new(ErrorCode::NotAllowed, "several questions are already waiting for the user")
                .because(Reason::TooManyQuestions)
                .into(),
        );
    }
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
fn reads_only_own(core: &Core, request: &Request, peer: &Peer) -> bool {
    let own = core.terminals_under(peer);
    let from = request.from_tab();
    // A read that names no terminal and has none of its own to default to reads nothing: its handler says
    // what is wrong with it, which is no business of the user's.
    let target = match &request.action {
        Action::Read(spec) => spec.target.or(from.map(Target::Tab)),
        Action::Blocks(spec) => spec.target.or(from.map(Target::Tab)),
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

/// What `request` needs of each terminal it names, for the ones that exist and that its caller may know of:
/// reading, typing or running, and what rename and focus need. Not `split` (opening is not scoped: what it
/// makes is the program's own), not `close` (every close is the user's to answer, on its own question).
fn touched(core: &Core, request: &Request, peer: &Peer) -> Vec<(i64, Capability)> {
    let from = request.from_tab().map(Target::Tab);
    let wanted: Vec<(Target, Capability)> = match &request.action {
        Action::Read(spec) => spec.target.or(from).map(|t| (t, Capability::Read)).into_iter().collect(),
        Action::Blocks(spec) => spec.target.or(from).map(|t| (t, Capability::Read)).into_iter().collect(),
        Action::Wait(spec) if spec.plan().is_ok_and(|plan| plan.needs_read()) => {
            vec![(spec.target, Capability::Read)]
        }
        Action::Events(spec) => spec
            .filter()
            .ok()
            .filter(|filter| filter.topics.contains(Topic::Output))
            .map(|filter| {
                filter.terminals.iter().map(|id| (Target::Terminal(*id), Capability::Read)).collect()
            })
            .unwrap_or_default(),
        Action::SendText { target, .. } => vec![(*target, Capability::Type)],
        Action::SendKeys(spec) => vec![(spec.target, Capability::Type)],
        Action::Mouse(spec) => vec![(spec.target, Capability::Type)],
        Action::Run(spec) => vec![(spec.target, Capability::Run)],
        Action::Focus(spec) => vec![(spec.target, Capability::Open)],
        Action::Rename(spec) => vec![(spec.target, Capability::Open)],
        _ => Vec::new(),
    };
    wanted
        .into_iter()
        .filter_map(|(target, capability)| Some((core.find(target, peer)?, capability)))
        .collect()
}

/// What a request that opened a terminal makes the program's own: the terminal its reply names.
pub(crate) fn own_opened(core: &Core, owner: Identity, reply: &Reply) {
    let opened = reply
        .ok
        .then_some(reply.result.as_ref())
        .flatten()
        .and_then(|result| result.get("terminal"))
        .and_then(serde_json::Value::as_i64);
    if let Some(terminal) = opened {
        core.desk().with_state(|_, scopes| scopes.own(terminal, owner));
    }
}

/// Forget what is kept about terminals that are closed, and what a session allowed for programs that ended
/// (every profile's desk).
pub(crate) fn sweep(core: &Core) {
    let open: Vec<i64> =
        core.shares().iter().flat_map(|shared| shared.peek_model(|model| model.ids())).collect();
    let now = Instant::now();
    for seat in core.seats() {
        let ended = seat.desk.with_state(|grants, scopes| {
            scopes.retain_open(&open);
            let running = |process: ProcessRef| process::start_time(process.pid) == Some(process.started);
            scopes.prune(now, running);
            grants.prune(now, running)
        });
        // What a session allowed for a program that ended ends its streams too.
        for id in ended {
            core.bus.revoke_holder(seat.holder(false, id));
        }
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
