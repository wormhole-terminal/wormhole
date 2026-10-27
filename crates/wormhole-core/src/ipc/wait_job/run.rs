//! `run`: the wait for the block of a command that was typed at a prompt, and its output.
//!
//! The command's block is told by the shell's own marks, which the window publishes as `block` events
//! (`started`, `finished` with the exit code and the time): the first block after the one the shell had
//! when the command was typed. Its output is read once, when it ends, and only if the caller may read
//! the terminal then.

use std::time::{Duration, Instant};

use wormhole_term::{CaptureRange, CaptureSpec, ReadScratch};

use super::{Core, Leash, closed, gone, timed_out};
use crate::ipc::bus::Next;
use crate::ipc::control::event::{BlockPhase, Body};
use crate::ipc::control::printable::json_line;
use crate::ipc::control::read::REPLY_MAX_BYTES;
use crate::ipc::control::wait::{OUTPUT_HIDDEN_GONE, OUTPUT_HIDDEN_OFF, RUN_OUTPUT_BYTES, Ran};
use crate::ipc::control::{Error, TerminalState};

/// What a run waits for.
pub(super) struct RunWait {
    /// The id of the newest block that had run when the command was typed.
    pub(super) baseline: Option<u64>,
    pub(super) timeout: Duration,
}

/// A block after the baseline is the command's (or a later one's: the baseline is the last that ran).
fn after(baseline: Option<u64>, block: i64) -> bool {
    baseline.is_none_or(|before| i64::try_from(before).is_ok_and(|before| block > before))
}

pub(super) fn run(core: &Core, plan: &RunWait, started: Instant, leash: &mut Leash) -> Result<Ran, Error> {
    let deadline = started + plan.timeout;
    let mut mine: Option<i64> = None;
    loop {
        let now = Instant::now();
        if now >= deadline {
            let words = match mine {
                Some(block) => format!("the command is still running (block {block})"),
                None => "the command did not start".to_string(),
            };
            return Err(timed_out(plan.timeout, words));
        }
        match core.subscription.next(deadline.min(leash.due()).saturating_duration_since(now)) {
            Next::Event(published) => match &published.event.body {
                Body::Block { phase: BlockPhase::Started, block, .. }
                    if mine.is_none() && after(plan.baseline, *block) =>
                {
                    mine = Some(*block);
                }
                Body::Block { phase: BlockPhase::Finished, block, exit_code, took_ms, .. }
                    if after(plan.baseline, *block) && mine.is_none_or(|own| own == *block) =>
                {
                    let took = took_ms.unwrap_or_else(|| millis(started.elapsed()));
                    return Ok(finished(core, *block, *exit_code, took));
                }
                Body::TerminalClosed => return Err(gone(core.terminal, "was closed")),
                Body::Status { status: TerminalState::Exited, .. } => {
                    return Err(gone(core.terminal, "has ended"));
                }
                _ => {}
            },
            Next::Closed(reason) => return Err(closed(reason)),
            Next::Changed | Next::Lagged(_) | Next::Idle => {}
        }
        leash.slipped(Instant::now())?;
    }
}

fn millis(took: Duration) -> u64 {
    u64::try_from(took.as_millis()).unwrap_or(u64::MAX)
}

/// The reply for a command that ended: how, and what it printed if the caller may read it.
fn finished(core: &Core, block: i64, exit_code: Option<i32>, duration_ms: u64) -> Ran {
    let mut ran = Ran {
        terminal: core.terminal,
        block,
        exit_code,
        duration_ms,
        output: None,
        truncated: false,
        output_hidden: None,
    };
    if !core.subscription.may_read(core.terminal) {
        ran.output_hidden = Some(OUTPUT_HIDDEN_OFF.into());
        return ran;
    }
    let spec = CaptureSpec {
        range: CaptureRange::BlockOutput(u64::try_from(block).unwrap_or(0)),
        join: true,
        styles: false,
        max_bytes: RUN_OUTPUT_BYTES,
        max_rows: 5000,
    };
    match core.screen().and_then(|reader| {
        reader.capture(&spec, &mut ReadScratch::default()).map_err(|_| gone(core.terminal, "was closed"))
    }) {
        Ok(capture) => {
            let output = capture.lines.join("\n");
            ran.output = Some(output.trim_end().to_string());
            ran.truncated = capture.truncated;
            fit(&mut ran);
        }
        Err(_) => ran.output_hidden = Some(OUTPUT_HIDDEN_GONE.into()),
    }
    ran
}

/// What the reply has beside the output, with room to spare.
const ENVELOPE: usize = 1024;

/// Keep the reply within [`REPLY_MAX_BYTES`]: invisible characters are written as six-byte escapes, so
/// the output of 256 KiB of them is larger than the reply may be. The oldest text goes first.
pub(super) fn fit(ran: &mut Ran) {
    let limit = REPLY_MAX_BYTES - ENVELOPE;
    while json_line(&*ran).len() > limit {
        let Some(output) = ran.output.as_mut() else { return };
        if output.is_empty() {
            return;
        }
        let mut cut = output.len() / 4;
        while !output.is_char_boundary(cut) {
            cut += 1;
        }
        output.drain(..cut.max(1));
        ran.truncated = true;
    }
}
