//! `wait` and `run`: a condition over a terminal, or a command typed at its prompt, answered on a stream
//! thread of the socket's when it is over (`wormhole_core::ipc::WaitJob`).
//!
//! This thread decides in a moment: the flag, the request (patterns compiled within limits), the
//! terminal, whether the caller may read it or type into it, the budget, and, for a state that
//! already holds, the answer itself. It then makes the subscription the waiter listens on, in the same
//! step as it reads the terminal, so no event between the two is missed, and hands the socket thread the
//! job. The UI thread never waits, copies a screen or matches a pattern for a client.

use gpui_kit::*;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::bus::{Grant, Watch};
use wormhole_core::ipc::control::{
    Error, ErrorCode, Limited, Met, Reason, Reply, RunSpec, SendTarget, Target, TerminalState, WaitPlan,
    WaitSpec, Waited, read_check, run_check, writes_off,
};
use wormhole_core::ipc::{Answer, Peer, WaitJob};

use super::input::target_facts;
use super::{Found, automation_off, budget, find, not_found};
use crate::app_state::{AppState, app};
use crate::window::{ReaderFacts, TabFacts};

/// Answer `wait`. Order of the checks: the flag, the request, the terminal, the right to read what it
/// looks at, the budget, a state that holds already, then the stream caps.
pub(super) fn wait(spec: &WaitSpec, from: Option<i64>, access: Access, peer: &Peer, cx: &mut App) -> Answer {
    match try_wait(spec, from, access, peer, cx) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn try_wait(
    spec: &WaitSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
    cx: &mut App,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(cx) {
        return Err(off);
    }
    let plan = spec.plan()?;
    let found = find(spec.target, peer, cx).ok_or_else(|| missing(spec.target))?;
    let facts = reader_of(&found, cx)?;
    let TabFacts { state, shell, .. } = state_of(&found, cx).ok_or_else(|| missing(spec.target))?;
    let self_read = shell.is_some_and(|shell| peer.runs_in(shell));
    if plan.needs_read() {
        read_check(access.read, self_read)?;
    }
    if plan.block_done && !facts.reader.has_blocks() {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "this shell has no integration, so it has no command blocks to wait for",
        )
        .because(Reason::NoIntegration));
    }
    // A terminal whose shell has ended has nothing more to wait for, unless that is what is asked.
    let wants_exit = plan.statuses.contains(&TerminalState::Exited);
    if state == TerminalState::Exited && !wants_exit {
        return Err(Error::new(
            ErrorCode::NotFound,
            format!("terminal {}'s shell has ended: nothing more will happen in it", facts.terminal),
        )
        .because(Reason::Exited));
    }
    charge(Limited::Wait, from, cx)?;
    if plan.statuses.contains(&state) {
        let held = Waited {
            terminal: facts.terminal,
            met: Met::Status,
            after_ms: 0,
            status: Some(state),
            matched: None,
            block: None,
            exit_code: None,
            revision: Some(facts.reader.revision()),
        };
        return Ok(Answer::Reply(Reply::ok(held)));
    }
    let grant = grant_for(&plan, access, peer, cx);
    let bus = crate::automation::bus(cx)
        .ok_or_else(|| Error::new(ErrorCode::Failed, "there is no event bus in this app"))?;
    let (subscription, _) = bus
        .wait(Watch::of_wait(facts.terminal, &plan), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    tracing::info!(?from, terminal = facts.terminal, "control: a wait began");
    let job = WaitJob::wait(subscription, facts.reader.downgrade(), facts.terminal, plan);
    Ok(Answer::Wait(job))
}

/// Answer `run`. Order of the checks: the flag, the switch, the request, the terminal and its shell
/// (not the caller's own, not waiting for the user, integration, at its prompt), the budgets, the stream
/// caps; only then is anything typed.
pub(super) fn run(spec: &RunSpec, from: Option<i64>, access: Access, peer: &Peer, cx: &mut App) -> Answer {
    match try_run(spec, from, access, peer, cx) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn try_run(
    spec: &RunSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
    cx: &mut App,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(cx) {
        return Err(off);
    }
    let enabled = access.write;
    if !enabled {
        return Err(writes_off());
    }
    let plan = spec.plan()?;
    let found = find(spec.target, peer, cx).ok_or_else(|| missing(spec.target))?;
    let facts = reader_of(&found, cx)?;
    let target: SendTarget = target_facts(&found, peer, cx).ok_or_else(|| missing(spec.target))?;
    run_check(enabled, from, target, facts.reader.at_prompt())?;
    let bus = crate::automation::bus(cx)
        .ok_or_else(|| Error::new(ErrorCode::Failed, "there is no event bus in this app"))?;
    // The shell still looks to be at its prompt between the command being typed and its mark, so another
    // run in the meantime would pass the checks above and clear this one's line.
    let claim = bus.claim_run(facts.terminal).ok_or_else(|| {
        Error::new(
            ErrorCode::Refused,
            format!("a run is under way in terminal {}: wait for it to end", facts.terminal),
        )
        .because(Reason::RunPending)
    })?;
    charge(Limited::Run, from, cx)?;
    charge(Limited::SendText, from, cx)?;
    let reads_all = access.read;
    let (subscription, _) = bus
        .wait(
            Watch::of_run(facts.terminal),
            Grant { reads_all, own: Vec::new(), readable: Vec::new(), holder: access.holder },
            from,
        )
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    let tab = found.tab;
    // The prompt the command is typed at: its block, whatever the shell marks around it by the time it
    // is heard.
    let first = facts
        .reader
        .current_prompt()
        .ok_or_else(|| Error::new(ErrorCode::Failed, format!("tab {tab}'s shell has marked no prompt")))?;
    let typed =
        AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_run(tab, &plan.command, cx));
    if typed != Some(true) {
        return Err(Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached")));
    }
    tracing::info!(tab, ?from, bytes = plan.command.len(), "control: a command was run at a prompt");
    let job =
        WaitJob::run(subscription, facts.reader.downgrade(), facts.terminal, first, plan.timeout, claim);
    Ok(Answer::Wait(job))
}

fn missing(target: Target) -> Error {
    not_found(target).error.unwrap_or_else(|| Error::new(ErrorCode::NotFound, "no such terminal"))
}

/// Count a request against its budget; the refusal as an error.
fn charge(kind: Limited, from: Option<i64>, cx: &mut App) -> Result<(), Error> {
    budget(kind, from, cx).map_err(|reply| {
        reply
            .error
            .unwrap_or_else(|| Error::new(ErrorCode::RateLimited, "too many requests of this kind lately"))
    })
}

/// The terminal's screen reader, while its shell runs.
fn reader_of(found: &Found, cx: &App) -> Result<ReaderFacts, Error> {
    let tab = found.tab;
    found
        .window
        .workspace
        .read_with(cx, |ws, cx| ws.control_reader(tab, cx))
        .ok()
        .flatten()
        .ok_or_else(|| Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached")))
}

/// The terminal's state and its shell's pid, as the sidebar counts them.
fn state_of(found: &Found, cx: &App) -> Option<TabFacts> {
    let tab = found.tab;
    found.window.workspace.read_with(cx, |ws, cx| ws.control_tab_facts(tab, cx)).ok().flatten()
}

/// What the waiter may read: every terminal while the setting allows it, and the ones the caller runs
/// in (found by its place in the process tree, only when the plan reads text at all).
fn grant_for(plan: &WaitPlan, access: Access, peer: &Peer, cx: &mut App) -> Grant {
    let reads_all = access.read;
    let own = if plan.needs_read() {
        app(cx)
            .windows
            .iter()
            .filter_map(|window| {
                window.workspace.read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx)).ok()
            })
            .flatten()
            .collect()
    } else {
        Vec::new()
    };
    Grant { reads_all, own, readable: Vec::new(), holder: access.holder }
}
