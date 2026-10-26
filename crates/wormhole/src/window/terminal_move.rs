//! Moving terminals from one window to another with their programs running: out of this window
//! ([`Workspace::take_out`]: the terminals leave the list, their views and the PTYs or session hosts behind
//! them stay alive in what comes back) and into another ([`Workspace::receive`], or a window opened for
//! them, `group_ops::move_group_to_window` and `tear_off`).
//!
//! A terminal's view is an entity of the app, not of a window, so it is handed over as it is: the new
//! window makes its pane round it (`terminals::make_pane` takes it from `AppState::adopted` instead of
//! starting a shell), keeps its id (so its notes, tasks and the agent hooks that name it go on), and
//! reads the rest from the record the old window wrote (name, folder, pin, notes and tasks by the
//! features). Nothing is restarted and nothing is read from the database.
//!
//! What a move can't carry: the terminals' place in the sidebar's groups (the new list starts without
//! them) and the old window's view of their git state, which the new window asks again.
use std::collections::HashSet;

use gpui_kit::*;
use wormhole_core::storage::store::{SessionRecord, SplitRecord};
use wormhole_core::view::sidebar::SplitId;

use crate::app_state::app_mut;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;

/// Terminals on their way from one window to another.
pub(crate) struct Departing {
    /// What each one is (its id, name, folder, notes and tasks), in the order they were asked for.
    pub(super) records: Vec<SessionRecord>,
    /// Their views, running.
    pub(super) terminals: Vec<(i64, Entity<TerminalView>)>,
    /// The splits among them (whole splits only: a split with a member staying behind is not one).
    pub(super) splits: Vec<SplitRecord>,
    /// Which of them the old window had selected, by index in `records` (the first when none).
    pub(super) active: usize,
}

impl Workspace {
    /// Take the terminals `ids` (those of them this window has) out of the window and out of its
    /// sidebar's groups, their programs still running. The splits they are all of go with them; a
    /// terminal that should leave its split behind leaves it first (`leave_layout`). A window left with
    /// none gets a new terminal, as when its last one is closed. `None` when this window has none of `ids`.
    pub(super) fn take_out(
        &mut self,
        ids: &[i64],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Departing> {
        let moved: Vec<usize> = ids.iter().filter_map(|&id| self.find_session(id)).collect();
        if moved.is_empty() {
            return None;
        }
        let records: Vec<_> = moved.iter().map(|&ix| self.session_record(ix, cx)).collect();
        let terminals: Vec<_> =
            moved.iter().map(|&ix| (self.sessions[ix].id, self.sessions[ix].terminal().clone())).collect();
        let split_ids: HashSet<SplitId> = moved.iter().filter_map(|&ix| self.sessions[ix].split).collect();
        let splits: Vec<SplitRecord> =
            self.split_records().into_iter().filter(|record| split_ids.contains(&record.id)).collect();
        let selected = self.sessions.get(self.active).map(|session| session.id);
        let active = selected.and_then(|id| records.iter().position(|record| record.id == id)).unwrap_or(0);

        // Out of this window, the running terminals kept alive for the other one.
        let mut gone = moved;
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for ix in gone {
            let left = self.sessions.remove(ix);
            self.features_left(left.id, cx);
        }
        self.splits.retain(|split| !split_ids.contains(&split.id));
        // They go with their terminals: gone here, and told again where they land.
        for &split in &split_ids {
            self.publish_split_gone(split, cx);
        }
        let left_ids: Vec<i64> = records.iter().map(|record| record.id).collect();
        self.edit_sidebar_view(cx, |view| {
            for &id in &left_ids {
                view.groups.leave(id);
            }
        });
        self.active = selected.and_then(|id| self.find_session(id)).unwrap_or(0);
        if self.sessions.is_empty() {
            self.active = usize::MAX;
            self.new_session(window, cx);
        }
        self.dirty = true;
        self.with_sidebar(cx, |sidebar, cx| sidebar.list_rearranged(cx));
        Some(Departing { records, terminals, splits, active })
    }

    /// Take in the terminals that [`take_out`](Self::take_out) of another window gave up: they join the
    /// end of the list, each on its own (a split is made in a window opened for it, not joined), the one
    /// that was selected is selected here and has the keyboard, and this window comes forward.
    pub(crate) fn receive(&mut self, departing: Departing, window: &mut Window, cx: &mut Context<Self>) {
        let Departing { records, terminals, active, .. } = departing;
        app_mut(cx).adopted.extend(terminals);
        let mut shown = None;
        for (at, mut record) in records.into_iter().enumerate() {
            record.split_id = None;
            let ix = self.add_session(Some(record), window, cx);
            if at == active {
                shown = Some(ix);
            }
        }
        self.dirty = true;
        self.with_sidebar(cx, |sidebar, cx| sidebar.list_rearranged(cx));
        crate::background::activate_window(window, cx);
        if let Some(ix) = shown {
            self.activate(ix, window, cx);
        }
        cx.notify();
    }
}
