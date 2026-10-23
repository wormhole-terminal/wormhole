//! What each terminal's row shows, as the window gathers it once per frame from the terminal, its git
//! status and the features (`window::sidebar_host`): the facts core's `Listing` and the rows are built
//! from.
use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::notify::Source;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::sidebar::{RowState, SplitId};

use super::rows;
use crate::settings::settings_in;

/// What a terminal's row shows, gathered once per frame.
pub(crate) struct RowInfo {
    pub(crate) ix: usize,
    pub(crate) id: i64,
    pub(crate) name: SharedString,
    /// The agent in the terminal, if any.
    pub(crate) agent: Option<AgentId>,
    /// What tells the row's state: an agent, a program that reports its own status (it takes an
    /// agent's kind of row, without the logo), or neither (a plain terminal).
    pub(crate) source: Source,
    pub(crate) state: RowState,
    pub(crate) message: Option<String>,
    /// How long the terminal has been in its status.
    pub(crate) since: Duration,
    pub(crate) cwd: Option<PathBuf>,
    /// The repository its folder is a worktree of.
    pub(crate) worktree: Option<PathBuf>,
    /// The command in front of a plain terminal, and how long it has run: only for one that ends
    /// by itself (a build, a test run), since the time an editor or a dev server has been open says
    /// nothing.
    pub(crate) command: Option<String>,
    pub(crate) running: Option<Duration>,
    pub(crate) bell: bool,
    /// The last command failed (a failing exit code) and the next one hasn't started: the row's
    /// cue, whether or not anyone has seen it yet.
    pub(crate) failure: Option<RowFailure>,
    /// The command is done and left output you haven't seen ("new output").
    pub(crate) unread: bool,
    pub(crate) pinned: bool,
    /// The split the terminal is in, if any.
    pub(crate) split: Option<SplitId>,
    /// Its text isn't kept for a restart.
    pub(crate) forget_text: bool,
    /// The changed files the row counts.
    pub(crate) changes: Option<usize>,
    /// The repository the changes are of, so rows of one repository count them once.
    pub(crate) repo: Option<PathBuf>,
    /// What a feature puts on the row besides the count above, through [`RowExtras`]: the notes and
    /// tasks still open, today.
    pub(crate) extra: Option<RowExtra>,
}

/// What a feature puts on a terminal's row: a number the row draws (notes' "3 open", Changes'
/// file count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowExtra {
    pub count: usize,
}

/// The rows as a feature may reach them: each row's id and its one slot for an extra, and nothing
/// else (`RowInfo`'s other fields are the sidebar's). The batch is the window's, so a feature
/// downcasts its entity once and loops here, never once per row.
pub(crate) struct RowExtras<'a> {
    rows: &'a mut [RowInfo],
}

impl<'a> RowExtras<'a> {
    pub(crate) fn new(rows: &'a mut [RowInfo]) -> Self {
        Self { rows }
    }

    /// Each row's terminal id and its slot.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (i64, &mut Option<RowExtra>)> {
        self.rows.iter_mut().map(|row| (row.id, &mut row.extra))
    }
}

#[cfg(test)]
impl RowInfo {
    /// An idle plain terminal's row, for the tests that need one (here and in `feature_tests`).
    pub(crate) fn test_row(id: i64) -> Self {
        Self {
            ix: 0,
            id,
            name: "t".into(),
            agent: None,
            source: Source::Plain,
            state: RowState::Idle,
            message: None,
            since: Duration::from_secs(180),
            cwd: Some(PathBuf::from("/w")),
            worktree: None,
            command: None,
            running: None,
            bell: false,
            failure: None,
            unread: false,
            pinned: false,
            split: None,
            forget_text: false,
            changes: None,
            repo: None,
            extra: None,
        }
    }
}

/// A plain terminal's last failed command, as its row says it ("cargo build · exit 101").
pub(crate) struct RowFailure {
    pub(crate) code: i32,
    pub(crate) command: Option<String>,
    /// How long ago the shell said so.
    pub(crate) since: Duration,
}

impl RowInfo {
    /// A row whose status something tells (an agent or a program that reports) says it in words;
    /// a plain terminal says what runs in it.
    pub(super) fn told(&self) -> bool {
        self.source.told()
    }
}

/// What each row's times read now (compact rows, without a second line, show none).
pub(crate) fn row_times(infos: &[RowInfo], profile: &ProfileId, cx: &App) -> Vec<Option<String>> {
    let compact = !settings_in(profile, cx).density.two_lines();
    infos.iter().map(|info| rows::time_text(info, compact)).collect()
}
