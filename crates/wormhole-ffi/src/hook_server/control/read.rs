//! `read` and `blocks`: what a terminal shows, handed to the socket thread to copy.
//!
//! This thread decides (the flag, the terminal, whether the caller may read it, the request's size, the
//! budget) in a moment and returns a [`ReadJob`] holding the terminal's screen reader; the socket thread
//! copies the rows under the screen's lock and writes the reply.

use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::control::{
    BlocksSpec, Error, ErrorCode, Limited, ReadSpec, Reply, Target, read_check,
};
use wormhole_core::ipc::{Answer, Peer, ReadJob};
use wormhole_term::ScreenReader;

use super::{automation_off, not_found};
use crate::hook_server::core::Core;

/// What a read of one terminal needs.
struct Reading {
    terminal: i64,
    reader: ScreenReader,
    title: Option<String>,
}

/// Answer `read`.
pub(super) fn read(core: &Core, spec: &ReadSpec, from: Option<i64>, access: Access, peer: &Peer) -> Answer {
    let job = prepare(core, spec.target, from, access, peer).and_then(|reading| {
        let capture = spec.capture_spec().map_err(Reply::from)?;
        core.budget(Limited::Read, from)?;
        Ok(ReadJob::lines(reading.reader, reading.terminal, reading.title, capture))
    });
    answer(job)
}

/// Answer `blocks`.
pub(super) fn blocks(
    core: &Core,
    spec: &BlocksSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Answer {
    let job = prepare(core, spec.target, from, access, peer).and_then(|reading| {
        let wanted = spec.blocks_spec().map_err(Reply::from)?;
        core.budget(Limited::Read, from)?;
        Ok(ReadJob::blocks(reading.reader, reading.terminal, wanted))
    });
    answer(job)
}

fn answer(job: Result<ReadJob, Reply>) -> Answer {
    match job {
        Ok(job) => Answer::Read(job),
        Err(reply) => Answer::Reply(reply),
    }
}

/// The flag, the target (the caller's own without one) and the right to read it: the switch, or the caller
/// running under the target's shell.
fn prepare(
    core: &Core,
    target: Option<Target>,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Result<Reading, Reply> {
    if let Some(off) = automation_off(core) {
        return Err(off.into());
    }
    let Some(target) = target.or(from.map(Target::Tab)) else {
        return Err(Reply::error(
            ErrorCode::BadRequest,
            "name a terminal (--terminal, --tab or --pane); inside one of Wormhole's terminals its own is the default",
        ));
    };
    let Some(tab) = core.find(target, peer) else { return Err(not_found(target)) };
    let terminal = core.terminal(tab);
    let reader = terminal.as_ref().and_then(|terminal| terminal.reader());
    let (Some(terminal), Some(reader)) = (terminal, reader) else {
        return Err(Reply::from(Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached"))));
    };
    // Where the caller runs, not what it says: `from` alone would let anyone claim the terminal.
    let self_read = terminal.shell_pid().is_some_and(|shell| peer.runs_in(shell));
    read_check(access.read, self_read).map_err(Reply::from)?;
    let title = core.shared.peek_model(|model| model.facts(tab).and_then(|facts| facts.title.clone()));
    Ok(Reading { terminal: tab, reader, title })
}
