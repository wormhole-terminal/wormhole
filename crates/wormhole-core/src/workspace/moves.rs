//! Terminals moving from one window to another with their programs running (a group's "Move to new
//! window", a terminal torn out, `wh` moving one). The model only does the bookkeeping: what leaves is
//! written down as records (`Departing`), the window closes them as it closes any terminal (the shell, which
//! owns the programs, keeps the running ones for the next window instead of ending them), and the other
//! window takes the records in, ids kept, so notes, tasks and the agent hooks that name a terminal go on.
//! What a move can't carry: the terminals' place in the old window's groups.

use std::collections::HashSet;

use crate::storage::store::{SessionRecord, SplitRecord};

use super::event::Event;
use super::model::WorkspaceModel;

/// Terminals on their way out of a window.
#[derive(Clone, Debug, Default)]
pub struct Departing {
    /// What each one is, in the window's order.
    pub records: Vec<SessionRecord>,
    /// The splits among them: whole splits only, a split with a member staying behind is not one.
    pub splits: Vec<SplitRecord>,
    /// Which of them was selected, by place in `records` (the first when none).
    pub active: usize,
}

impl Departing {
    pub fn ids(&self) -> Vec<i64> {
        self.records.iter().map(|record| record.id).collect()
    }
}

impl WorkspaceModel {
    /// Take the terminals `ids` (those of them this window has) out: written down, then closed as a close
    /// does it (selection and a split's heir by the window's rules; a window left with none starts a new
    /// terminal). `None` when this window has none of them.
    pub fn take_out(&mut self, ids: &[i64]) -> Option<(Departing, Vec<Event>)> {
        let moved: Vec<i64> = self.ids().into_iter().filter(|id| ids.contains(id)).collect();
        if moved.is_empty() {
            return None;
        }
        let whole: HashSet<i64> = self
            .splits
            .iter()
            .filter(|split| {
                self.members(split.id).into_iter().all(|ix| moved.contains(&self.sessions[ix].id))
            })
            .map(|split| split.id)
            .collect();
        let mut records: Vec<SessionRecord> = Vec::new();
        for session in self.sessions.iter().filter(|session| moved.contains(&session.id)) {
            let mut record = self.session_record(session);
            record.split_id = record.split_id.filter(|split| whole.contains(split));
            records.push(record);
        }
        let record = self.window_record(0, None);
        let splits = record.splits.into_iter().filter(|split| whole.contains(&split.id)).collect();
        let active = self.selected.and_then(|id| records.iter().position(|r| r.id == id)).unwrap_or(0);
        for id in moved {
            self.close_terminal(id);
        }
        Some((Departing { records, splits, active }, self.out.take()))
    }

    /// Take in terminals another window gave up: they join the end of the list, ids kept, running (`Adopted`);
    /// the one that was selected there is selected here.
    pub fn receive(&mut self, departing: &Departing) -> Vec<Event> {
        if departing.records.is_empty() {
            return Vec::new();
        }
        self.take_in(&departing.records, &departing.splits, false);
        self.rearranged();
        self.emit(Event::Layout);
        let shown = departing.records.get(departing.active).unwrap_or(&departing.records[0]).id;
        self.select_id(shown);
        self.out.take()
    }
}
