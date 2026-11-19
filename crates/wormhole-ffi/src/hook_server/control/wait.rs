//! `wait` and `run`: a condition over a terminal, or a command typed at its prompt, answered on a thread of
//! the socket's when it is over (`wormhole_core::ipc::WaitJob`).
//!
//! This thread decides in a moment: the flag, the request (patterns compiled within limits), the terminal,
//! whether the caller may read it or type into it, the budget, and, for a state that already holds, the
//! answer itself. It then makes the subscription the waiter listens on, in the same step as it reads the
//! terminal, so no event between the two is missed, and hands the socket thread the job.

use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::bus::{Grant, Watch};
use wormhole_core::ipc::control::{
    Error, ErrorCode, Limited, Met, Reason, Reply, RunSpec, SendTarget, Target, TerminalState, WaitPlan,
    WaitSpec, Waited, read_check, run_check, writes_off,
};
use wormhole_core::ipc::{Answer, Peer, WaitJob};
use wormhole_core::workspace::StatusEvent;
use wormhole_term::ScreenReader;

use super::input::target_facts;
use super::{automation_off, not_found};
use crate::hook_server::core::Core;
use crate::hook_server::terminals::TabFacts;

/// ⌃U: the shell clears the line it has so far, as Put Command at Prompt does before it types.
const KILL_LINE: u8 = 0x15;
/// The pause between the pasted command and the Return that runs it.
const SUBMIT_DELAY: std::time::Duration = std::time::Duration::from_millis(60);

/// Answer `wait`. Order of the checks: the flag, the request, the terminal, the right to read what it looks
/// at, the budget, a state that holds already, then the stream caps.
pub(super) fn wait(core: &Core, spec: &WaitSpec, from: Option<i64>, access: Access, peer: &Peer) -> Answer {
    match try_wait(core, spec, from, access, peer) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn try_wait(
    core: &Core,
    spec: &WaitSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(core) {
        return Err(off);
    }
    let plan = spec.plan()?;
    let tab = core.find(spec.target, peer).ok_or_else(|| missing(spec.target))?;
    let reader = reader_of(core, tab)?;
    let TabFacts { state, shell, .. } = core.tab_facts(tab).ok_or_else(|| missing(spec.target))?;
    let self_read = shell.is_some_and(|shell| peer.runs_in(shell));
    if plan.needs_read() {
        read_check(access.read, self_read)?;
    }
    if plan.block_done && !reader.has_blocks() {
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
            format!("terminal {tab}'s shell has ended: nothing more will happen in it"),
        )
        .because(Reason::Exited));
    }
    charge(core, Limited::Wait, from)?;
    if plan.statuses.contains(&state) {
        let held = Waited {
            terminal: tab,
            met: Met::Status,
            after_ms: 0,
            status: Some(state),
            matched: None,
            block: None,
            exit_code: None,
            revision: Some(reader.revision()),
        };
        return Ok(Answer::Reply(Reply::ok(held)));
    }
    let grant = grant_for(core, &plan, access, peer);
    let (subscription, _) = core
        .bus
        .wait(Watch::of_wait(tab, &plan), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    // The state may have changed between the look above and the subscription: the next diff tells it.
    core.kick();
    tracing::info!(?from, terminal = tab, "control: a wait began");
    Ok(Answer::Wait(WaitJob::wait(subscription, reader.downgrade(), tab, plan)))
}

/// Answer `run`. Order of the checks: the flag, the switch, the request, the terminal and its shell (not the
/// caller's own, not waiting for the user, integration, at its prompt), the budgets, the stream caps; only
/// then is anything typed.
pub(super) fn run(core: &Core, spec: &RunSpec, from: Option<i64>, access: Access, peer: &Peer) -> Answer {
    match try_run(core, spec, from, access, peer) {
        Ok(answer) => answer,
        Err(error) => Answer::Reply(error.into()),
    }
}

fn try_run(
    core: &Core,
    spec: &RunSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Result<Answer, Error> {
    if let Some(off) = automation_off(core) {
        return Err(off);
    }
    let enabled = access.write;
    if !enabled {
        return Err(writes_off());
    }
    let plan = spec.plan()?;
    let tab = core.find(spec.target, peer).ok_or_else(|| missing(spec.target))?;
    let reader = reader_of(core, tab)?;
    let target: SendTarget = target_facts(core, tab, peer).ok_or_else(|| missing(spec.target))?;
    run_check(enabled, from, target, reader.at_prompt())?;
    // The shell still looks to be at its prompt between the command being typed and its mark, so another
    // run in the meantime would pass the checks above and clear this one's line.
    let claim = core.bus.claim_run(tab).ok_or_else(|| {
        Error::new(ErrorCode::Refused, format!("a run is under way in terminal {tab}: wait for it to end"))
            .because(Reason::RunPending)
    })?;
    charge(core, Limited::Run, from)?;
    charge(core, Limited::SendText, from)?;
    let grant =
        Grant { reads_all: access.read, own: Vec::new(), readable: Vec::new(), holder: access.holder };
    let (subscription, _) = core
        .bus
        .wait(Watch::of_run(tab), grant, from)
        .map_err(|refused| Error::new(ErrorCode::RateLimited, refused.to_string()))?;
    // The prompt the command is typed at: its block, whatever the shell marks around it by the time it is heard.
    let first = reader
        .current_prompt()
        .ok_or_else(|| Error::new(ErrorCode::Failed, format!("tab {tab}'s shell has marked no prompt")))?;
    let terminal = core
        .terminal(tab)
        .ok_or_else(|| Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached")))?;
    terminal.write(vec![KILL_LINE]);
    terminal.paste(plan.command.clone());
    terminal.control_typed();
    let submit = terminal.clone();
    let spawned = std::thread::Builder::new().name("wormhole-hook-submit".into()).spawn(move || {
        std::thread::sleep(SUBMIT_DELAY);
        submit.write(b"\r".to_vec());
    });
    if let Err(err) = spawned {
        tracing::warn!("couldn't press Return after a command: {err}");
    }
    core.shared.feed_quiet(tab, StatusEvent::Typed);
    tracing::info!(tab, ?from, bytes = plan.command.len(), "control: a command was run at a prompt");
    Ok(Answer::Wait(WaitJob::run(subscription, reader.downgrade(), tab, first, plan.timeout, claim)))
}

fn missing(target: Target) -> Error {
    not_found(target).error.unwrap_or_else(|| Error::new(ErrorCode::NotFound, "no such terminal"))
}

/// Count a request against its budget; the refusal as an error.
fn charge(core: &Core, kind: Limited, from: Option<i64>) -> Result<(), Error> {
    core.budget(kind, from).map_err(|reply| {
        reply
            .error
            .unwrap_or_else(|| Error::new(ErrorCode::RateLimited, "too many requests of this kind lately"))
    })
}

/// The terminal's screen reader, while its shell runs.
fn reader_of(core: &Core, tab: i64) -> Result<ScreenReader, Error> {
    core.terminal(tab)
        .and_then(|terminal| terminal.reader())
        .ok_or_else(|| Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached")))
}

/// What the waiter may read: every terminal while the setting allows it, and the ones the caller runs in
/// (found by its place in the process tree, only when the plan reads text at all).
fn grant_for(core: &Core, plan: &WaitPlan, access: Access, peer: &Peer) -> Grant {
    let own = if plan.needs_read() { core.terminals_under(peer) } else { Vec::new() };
    Grant { reads_all: access.read, own, readable: Vec::new(), holder: access.holder }
}
