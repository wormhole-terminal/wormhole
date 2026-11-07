//! Terminals: making them, selecting them, naming and pinning them.

use std::path::PathBuf;

use crate::view::sidebar::order::shortcut_target;

use super::event::{Event, Refusal};
use super::model::WorkspaceModel;
use super::session::Session;

impl WorkspaceModel {
    pub(super) fn refuse(&mut self) {
        self.emit(Event::Refused(Refusal::Unknown));
    }

    /// A new terminal for the shell to start a program for; it is not selected. Its id.
    pub(super) fn open_terminal(&mut self, cwd: Option<PathBuf>) -> i64 {
        let id = self.alloc_id();
        let number = self.next_number();
        self.sessions.push(Session::new(id, number));
        self.emit(Event::Opened { id, cwd });
        self.rearranged();
        id
    }

    /// The folder a new terminal starts in: where the selected one is, else the configured one.
    pub(super) fn start_folder(&self) -> Option<PathBuf> {
        self.selected_index()
            .and_then(|ix| self.sessions[ix].facts.cwd.clone())
            .or_else(|| self.cfg.start_folder.clone())
    }

    /// A new terminal in the selected one's folder, selected. A search that hides it would leave the
    /// selection out of sight, so the search ends.
    pub(super) fn new_terminal_selected(&mut self) {
        let cwd = self.start_folder();
        let id = self.open_terminal(cwd);
        self.select_id(id);
        if !self.chrome.search.is_empty() {
            self.chrome.search.clear();
            self.emit(Event::Chrome);
        }
    }

    /// Select terminal `id`, and focus its pane when it is in a split (selecting a row of a split
    /// shows the split with that pane focused).
    pub(super) fn select_id(&mut self, id: i64) {
        if self.index_of(id).is_none() {
            return self.refuse();
        }
        let changed = self.selected != Some(id);
        self.selected = Some(id);
        let split = self.index_of(id).and_then(|ix| self.sessions[ix].split);
        if let Some(split) = split.and_then(|split| self.split_mut(split))
            && split.layout.focused() != id
        {
            split.layout.focus(id);
            self.emit(Event::Layout);
        }
        let seen = self.acknowledge_on_screen();
        if changed {
            self.emit(Event::Selected { id: Some(id) });
        }
        if changed || seen {
            // The selected row's marks, and the on-screen mates', follow the selection.
            self.emit(Event::Listing);
        }
    }

    /// ⌘⇧[ and ⌘⇧]: through what the list shows, wrapping round.
    pub(super) fn select_step(&mut self, delta: isize) {
        let Some(active) = self.selected_index() else { return };
        if let Some(next) = self.listing().step(active, delta) {
            let id = self.sessions[next].id;
            self.select_id(id);
        }
    }

    /// ⌘1 to ⌘9: the terminal that holds the number, the last numbered one for a number past them.
    pub(super) fn select_number(&mut self, n: usize) {
        let numbered = self.listing().numbered();
        match shortcut_target(&numbered, n) {
            Some(ix) => {
                let id = self.sessions[ix].id;
                self.select_id(id);
            }
            None => self.refuse(),
        }
    }

    /// A blank name goes back to what runs in the terminal.
    pub(super) fn rename(&mut self, id: i64, name: &str) {
        let Some(ix) = self.index_of(id) else { return self.refuse() };
        let name = name.trim();
        let name = (!name.is_empty()).then(|| name.to_string());
        if self.sessions[ix].custom_name != name {
            self.sessions[ix].custom_name = name;
            self.emit(Event::Listing);
        }
    }

    pub(super) fn toggle_pin(&mut self, id: i64) {
        let Some(ix) = self.index_of(id) else { return self.refuse() };
        let pinned = !self.split_pinned(ix);
        self.set_pinned(id, pinned);
    }

    /// Pin a terminal to the top of its group, or let it go back to its place. A split is pinned or
    /// not as a whole: its members all take the new state.
    pub(super) fn set_pinned(&mut self, id: i64, pinned: bool) {
        let Some(ix) = self.index_of(id) else { return self.refuse() };
        let members = match self.sessions[ix].split {
            Some(split) => self.members(split),
            None => vec![ix],
        };
        for member in members {
            self.sessions[member].pinned = pinned;
        }
        self.rearranged();
    }

    /// Whether the split `ix` is in, or `ix` itself, is pinned: pinning one member pins them all.
    fn split_pinned(&self, ix: usize) -> bool {
        match self.sessions[ix].split {
            Some(id) => self.members(id).into_iter().any(|member| self.sessions[member].pinned),
            None => self.sessions[ix].pinned,
        }
    }
}
