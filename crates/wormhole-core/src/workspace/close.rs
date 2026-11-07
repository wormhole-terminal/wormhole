//! Closing a terminal: who is selected after it, and what happens to its split.

use crate::view::panes::TabPanes;

use super::event::Event;
use super::model::WorkspaceModel;

/// The selected terminal's place after the terminal at place `closed` is removed from `len`.
/// `None`: it was the only one, so a new one starts. `Some((place, switched))`: the selection is now
/// at `place`, and `switched` says whether that is a different terminal from before; closing another
/// terminal in the background leaves the selected one exactly as it was. (The app's rule, from
/// `window/session.rs`: places are the window's order, not the list's.)
pub fn session_after_close(active: usize, closed: usize, len: usize) -> Option<(usize, bool)> {
    debug_assert!(active < len && closed < len);
    if len < 2 {
        return None;
    }
    Some(match closed.cmp(&active) {
        // Everything after the closed one moves one place up, the selected one with it.
        std::cmp::Ordering::Less => (active - 1, false),
        // The one that slid into its place, or the last one if it was last.
        std::cmp::Ordering::Equal => (active.min(len - 2), true),
        std::cmp::Ordering::Greater => (active, false),
    })
}

impl WorkspaceModel {
    /// Close terminal `id`: its program ends (`Closed`). In a split its pane goes and the others take
    /// its room; one left alone becomes a plain row. When it was selected the keyboard stays in the
    /// split (the layout's next pane), else the one that slid into its place is selected; closing the
    /// only terminal starts a new one.
    pub(super) fn close_terminal(&mut self, id: i64) {
        let Some(ix) = self.index_of(id) else { return self.refuse() };
        let len = self.sessions.len();
        let heir = self.heir_of(ix);
        let was_selected = self.selected == Some(id);
        let after = self.selected_index().and_then(|active| session_after_close(active, ix, len));
        self.leave_layout(ix);
        self.sessions.remove(ix);
        self.sidebar.groups.leave(id);
        self.emit(Event::Closed { id });
        self.rearranged();
        if len == 1 {
            self.selected = None;
            return self.new_terminal_selected();
        }
        if was_selected {
            let next = heir
                .and_then(|heir| self.index_of(heir))
                .or_else(|| after.map(|(place, _)| place))
                .map(|place| self.sessions[place].id);
            self.selected = None;
            if let Some(next) = next {
                self.select_id(next);
            }
        }
    }

    /// The member of the split that takes the selection when `ix` closes: the layout's focus once its
    /// pane is gone.
    fn heir_of(&self, ix: usize) -> Option<i64> {
        let split = self.split_of(self.sessions[ix].split?)?;
        let mut layout: TabPanes = split.layout.clone();
        layout.close(self.sessions[ix].id);
        Some(layout.focused())
    }
}
