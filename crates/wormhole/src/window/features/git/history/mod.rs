//! History: the commits of the terminal's branch, a commit's files and their diffs, and the branch
//! picker that chooses the log. An optional feature (`FeatureId::History`): off, its tab, its key, its
//! menu line and the palette's commits are gone, and Changes, the diff and the worktrees are not.
//!
//! One [`History`] entity per window holds, for each terminal of the window, the log it shows
//! ([`HistoryState`]), and the picker. The window talks to it through the table (`def`); it talks to the
//! window by [`FeatureEvent`]s and never holds the `Workspace`. A commit's file opens its diff in
//! Changes' view over the terminal, so History reaches the window's `Changes` entity through the facts:
//! an optional feature may lean on an always-on one, not the other way round.
//!
//! It polls itself: on the tick while its tab shows, and when the window comes to the front after git
//! was missing. The status the sidebar shows is the window's (`window::repo`), not read here.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;

use super::picker::{BranchPicker, Picker, PickerHost};
use crate::terminal_view::TerminalView;
use crate::window::facts::WindowFacts;
use crate::window::feature::FeatureEvent;

pub(crate) mod api;
mod def;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
#[cfg(test)]
mod flow_tests;
mod keys;
mod refresh;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub(in crate::window) use def::DEF;
use state::{CommitFiles, HistoryRow, HistoryState};
#[cfg(test)]
use state::{FILES_KEPT, Fetched, HistoryKey};

/// The window's History: every terminal's log, by its id, and the branch picker.
pub(in crate::window) struct History {
    of: HashMap<i64, HistoryState>,
    /// The terminal of each session, as the window last lent it: the log is read in the folder it is in
    /// now, which a read that lands later is checked against.
    terminals: HashMap<i64, Entity<TerminalView>>,
    picker: BranchPicker,
    /// The window as the feature last heard it.
    profile: ProfileId,
}

impl EventEmitter<FeatureEvent> for History {}

impl PickerHost for History {
    fn picker(&self) -> &BranchPicker {
        &self.picker
    }

    fn picker_mut(&mut self) -> &mut BranchPicker {
        &mut self.picker
    }

    /// The log of the branch picked, for the terminal the picker was open for.
    fn pick(&mut self, session: i64, branch: Option<String>, cx: &mut Context<Self>) {
        self.of.insert(session, HistoryState::for_rev(branch));
        self.refresh(session, cx);
        cx.notify();
    }
}

impl History {
    pub(in crate::window) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            of: HashMap::new(),
            terminals: HashMap::new(),
            picker: BranchPicker::new(Picker::History, window, cx),
            profile: ProfileId::default_profile(),
        }
    }

    /// Takes in what the window says of itself: whose settings it draws with, and the terminals it has.
    pub(super) fn follow(&mut self, facts: &WindowFacts<'_>) {
        if self.profile != *facts.profile {
            self.profile = facts.profile.clone();
        }
        for session in facts.sessions() {
            // A restart puts a new view under the same id: the log is read in the folder of the one
            // that lives, and the dead one is let go.
            match self.terminals.entry(session.id) {
                Entry::Occupied(mut kept) if *kept.get() != *session.terminal => {
                    kept.insert(session.terminal.clone());
                }
                Entry::Occupied(_) => {}
                Entry::Vacant(slot) => {
                    slot.insert(session.terminal.clone());
                }
            }
        }
    }

    /// A terminal is gone from this window (closed, or moved to another).
    pub(super) fn forget(&mut self, session: i64) {
        self.of.remove(&session);
        self.terminals.remove(&session);
    }

    /// The tick, while the tab shows: the log is read again about every 5 s, at once when the folder or
    /// branch changed or the window found a repository the tab had none for, and not while git is missing (the window coming to the front is the retry) or
    /// the folder is known to be in no repository.
    pub(super) fn tick(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        self.follow(facts);
        let Some(active) = facts.active() else { return };
        if !(facts.panel_shown && facts.tab == wormhole_core::config::TabId::History) {
            return;
        }
        let Some(state) = self.of.get(&active.id) else { return };
        let cwd = active.terminal.read(cx).cwd.as_deref();
        let unanswered = cwd.is_some_and(|cwd| !state.answered_for(cwd));
        let absent = cwd.is_some_and(|cwd| {
            active.git.is_absent_for(cwd)
                && state.root.is_none()
                && state.problem.is_none()
                && state.answered_for(cwd)
        });
        // The window found a repository where the tab had answered "none" (a `git init` since): now,
        // not at the next period.
        let appeared = state.root.is_none() && state.problem.is_none() && active.git.status.is_some();
        if (facts.ticks.is_multiple_of(12) || unanswered || appeared) && !state.waits_for_retry() && !absent {
            self.refresh(active.id, cx);
        }
    }

    /// The window came to the front: coming back after installing git is a retry; polling a missing git
    /// is not.
    pub(super) fn came_to_front(&mut self, facts: &WindowFacts<'_>, cx: &mut Context<Self>) {
        self.follow(facts);
        let Some(active) = facts.active() else { return };
        if self.of.get(&active.id).is_some_and(HistoryState::waits_for_retry) {
            self.refresh(active.id, cx);
        }
    }

    /// The tab was shown: its picker of an earlier visit is gone and the log is read again.
    pub(super) fn shown(&mut self, facts: &WindowFacts<'_>, session: i64, cx: &mut Context<Self>) {
        self.follow(facts);
        self.picker.close();
        self.refresh(session, cx);
        cx.notify();
    }
}

#[cfg(test)]
impl History {
    /// How many commits the log of terminal `session` lists.
    pub(in crate::window) fn commit_count(&self, session: i64) -> usize {
        self.of.get(&session).map_or(0, |state| state.commits.len())
    }

    /// The newest commit of the log of terminal `session`.
    pub(in crate::window) fn first_commit(&self, session: i64) -> Option<wormhole_core::git::Commit> {
        self.of.get(&session)?.commits.first().cloned()
    }
}
