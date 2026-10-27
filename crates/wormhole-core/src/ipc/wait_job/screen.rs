//! The conditions that look at a terminal's screen: text that appears or goes, and quiet.
//!
//! A look happens when the terminal printed (the wake-up the bus gives a waiter, however often it
//! prints), at most every [`LOOK_EVERY`], and once at the start; the screen is copied in one short hold
//! of its lock into buffers the thread keeps, never on the thread that prints or draws. Quiet is a timer
//! over the output revision: it needs no copy at all.

use std::time::{Duration, Instant};

use regex::Regex;
use wormhole_term::{CaptureRange, CaptureSpec, ReadScratch, ScreenReader};

use super::Core;
use crate::ipc::bus::LOOK_EVERY;
use crate::ipc::control::wait::RUN_OUTPUT_BYTES;
use crate::ipc::control::{Error, ErrorCode, TerminalState, WaitPlan, reads_off};

/// What a condition found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Hit {
    Status(TerminalState),
    Text(String),
    Gone,
    Stable,
    BlockDone { block: i64, exit_code: Option<i32> },
}

/// The screen conditions of a plan and what has been seen of the screen so far.
pub(super) struct Look<'a> {
    text: Option<&'a Regex>,
    gone: Option<&'a Regex>,
    stable: Option<Duration>,
    /// The output revision at the last look, and since when it has not moved.
    seen: u64,
    quiet_since: Instant,
    looked_at: Option<Instant>,
    /// The terminal printed since the last look.
    dirty: bool,
    scratch: ReadScratch,
}

/// What a look reads: the live screen (a full-screen program's, while it has the screen), wrapped
/// lines put back together.
const SCREEN: CaptureSpec = CaptureSpec {
    range: CaptureRange::Screen,
    join: true,
    styles: false,
    max_bytes: RUN_OUTPUT_BYTES,
    max_rows: 5000,
};

impl<'a> Look<'a> {
    pub(super) fn new(plan: &'a WaitPlan, now: Instant) -> Self {
        Self {
            text: plan.text.as_ref(),
            gone: plan.gone.as_ref(),
            stable: plan.stable,
            seen: 0,
            quiet_since: now,
            looked_at: None,
            dirty: false,
            scratch: ReadScratch::default(),
        }
    }

    /// The screen as the wait begins: text that is there already, text that is not there, and where
    /// the revision stands. A quiet is counted from here.
    pub(super) fn first(&mut self, core: &Core) -> Result<Option<Hit>, Error> {
        let reader = core.screen()?;
        self.seen = reader.revision();
        self.looked_at = Some(self.quiet_since);
        self.read(core, &reader)
    }

    /// The terminal printed.
    pub(super) fn changed(&mut self) {
        self.dirty = true;
    }

    /// When the thread should wake for the screen, if it should: a look it owes, a quiet that ends.
    pub(super) fn due(&self) -> Option<Instant> {
        let look = self.dirty.then(|| self.looked_at.map_or_else(Instant::now, |at| at + LOOK_EVERY));
        let quiet = self.stable.map(|quiet| self.quiet_since + quiet);
        match (look, quiet) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Look if one is owed and due, and see whether the quiet has lasted.
    pub(super) fn step(&mut self, core: &Core, now: Instant) -> Result<Option<Hit>, Error> {
        let owed = self.dirty && self.looked_at.is_none_or(|at| now >= at + LOOK_EVERY);
        let quiet_over = self.stable.is_some_and(|quiet| now >= self.quiet_since + quiet);
        if !owed && !quiet_over {
            return Ok(None);
        }
        let reader = core.screen()?;
        if owed {
            self.dirty = false;
            self.looked_at = Some(now);
        }
        // The revision is read on both paths: a print the wake-up has not told of yet moves the quiet's
        // start, so it is never over while the terminal is still printing.
        let revision = reader.revision();
        if revision != self.seen {
            self.seen = revision;
            self.quiet_since = now;
        } else if quiet_over {
            return Ok(Some(Hit::Stable));
        }
        if owed { self.read(core, &reader) } else { Ok(None) }
    }

    /// Match the screen's text against the patterns.
    fn read(&mut self, core: &Core, reader: &ScreenReader) -> Result<Option<Hit>, Error> {
        if self.text.is_none() && self.gone.is_none() {
            return Ok(None);
        }
        // Checked when the text is about to be read: the setting may have gone off a moment ago.
        if !core.subscription.may_read(core.terminal) {
            return Err(reads_off());
        }
        let capture = reader
            .capture(&SCREEN, &mut self.scratch)
            .map_err(|problem| Error::new(ErrorCode::Failed, problem.to_string()))?;
        let screen = capture.lines.join("\n");
        if let Some(found) = self.text.and_then(|pattern| pattern.find(&screen)) {
            return Ok(Some(Hit::Text(found.as_str().to_string())));
        }
        if self.gone.is_some_and(|pattern| !pattern.is_match(&screen)) {
            return Ok(Some(Hit::Gone));
        }
        Ok(None)
    }
}
