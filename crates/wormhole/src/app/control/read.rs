//! `read` and `blocks`: what a terminal shows, handed to the socket thread to copy.
//!
//! This thread decides (the flag, the terminal, whether the caller may read it, the request's size,
//! the budget) in a moment and returns a [`ReadJob`] holding the terminal's screen reader; the socket
//! thread copies the rows under the screen's lock and writes the reply. The UI thread never copies
//! terminal text for a client.

use gpui_kit::*;
use wormhole_core::ipc::control::{
    BlocksSpec, Error, ErrorCode, Limited, ReadSpec, Reply, Target, read_check,
};
use wormhole_core::ipc::{Answer, Peer, ReadJob};

use super::{automation_off, budget, find, not_found};
use crate::settings::settings;
use crate::window::ReaderFacts;

/// Answer `read`.
pub(super) fn read(spec: &ReadSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Answer {
    let job = prepare(spec.target, from, peer, cx).and_then(|facts| {
        let capture = spec.capture_spec().map_err(Reply::from)?;
        budget(Limited::Read, from, cx)?;
        let ReaderFacts { terminal, reader, title, .. } = facts;
        Ok(ReadJob::lines(reader, terminal, title, capture))
    });
    answer(job)
}

/// Answer `blocks`.
pub(super) fn blocks(spec: &BlocksSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Answer {
    let job = prepare(spec.target, from, peer, cx).and_then(|facts| {
        let wanted = spec.blocks_spec().map_err(Reply::from)?;
        budget(Limited::Read, from, cx)?;
        Ok(ReadJob::blocks(facts.reader, facts.terminal, wanted))
    });
    answer(job)
}

fn answer(job: Result<ReadJob, Reply>) -> Answer {
    match job {
        Ok(job) => Answer::Read(job),
        Err(reply) => Answer::Reply(reply),
    }
}

/// The flag, the target (the caller's own without one) and the right to read it: the switch, or the
/// caller running under the target's shell.
fn prepare(
    target: Option<Target>,
    from: Option<i64>,
    peer: &Peer,
    cx: &mut App,
) -> Result<ReaderFacts, Reply> {
    if let Some(off) = automation_off(cx) {
        return Err(off.into());
    }
    let Some(target) = target.or(from.map(Target::Tab)) else {
        return Err(Reply::error(
            ErrorCode::BadRequest,
            "name a terminal (--terminal, --tab or --pane); inside one of Wormhole's terminals its own is the default",
        ));
    };
    let Some(found) = find(target, cx) else { return Err(not_found(target)) };
    let tab = found.tab;
    let facts = found.window.workspace.read_with(cx, |ws, cx| ws.control_reader(tab, cx)).ok().flatten();
    let Some(facts) = facts else {
        return Err(Reply::from(Error::new(ErrorCode::Failed, format!("tab {tab} could not be reached"))));
    };
    // Where the caller runs, not what it says: `from` alone would let anyone claim the terminal.
    let self_read = facts.shell.is_some_and(|shell| peer.runs_in(shell));
    read_check(settings(cx).control_read, self_read).map_err(Reply::from)?;
    Ok(facts)
}
