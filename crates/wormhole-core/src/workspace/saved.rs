//! A window as it is saved: the model's terminals, splits, groups and columns as core's `WindowRecord`
//! (the rows `Store::save_window` writes), and a window made again from one.
//!
//! The record's `layout` is JSON in the shape the GPUI app saves (`SavedLayout`), so a record written by either
//! shell reads in the other. Restoring starts a program for every terminal (`Event::Opened`), in the folder it
//! was last in; what the programs were running is not kept (that is the session hosts' job).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::tab::TabId;
use crate::storage::store::{SessionRecord, SplitRecord, WindowRecord};
use crate::view::layout::Panes;
use crate::view::panes::TabPanes;
use crate::view::sidebar::{Status, WindowSidebar};

use super::event::Event;
use super::model::{Split, WorkspaceModel};
use super::session::Session;

/// A window's layout as saved with it (the app's `window/saved_layout.rs`, key for key).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedLayout {
    pub sidebar_width: f32,
    pub notes_width: f32,
    pub sidebar_open: bool,
    pub notes_open: bool,
    pub right_tab: TabId,
    /// The sidebar's groups, their folds and the list's order.
    pub sidebar: WindowSidebar,
}

impl Default for SavedLayout {
    fn default() -> Self {
        Self::new(Panes::default(), TabId::default(), WindowSidebar::default())
    }
}

impl SavedLayout {
    pub fn new(panes: Panes, right_tab: TabId, sidebar: WindowSidebar) -> Self {
        Self {
            sidebar_width: panes.sidebar_width,
            notes_width: panes.panel_width,
            sidebar_open: panes.sidebar_open,
            notes_open: panes.panel_wanted,
            right_tab,
            sidebar,
        }
    }

    pub fn panes(&self) -> Panes {
        Panes {
            sidebar_open: self.sidebar_open,
            sidebar_width: self.sidebar_width,
            panel_wanted: self.notes_open,
            panel_width: self.notes_width,
        }
        .restored()
    }

    /// A new window's layout, made from another's: the same columns and panel tab, the sidebar as a new window
    /// starts (no groups, nothing folded).
    pub fn for_new_window(json: Option<&str>) -> String {
        let mut layout: Self = json.and_then(|json| serde_json::from_str(json).ok()).unwrap_or_default();
        layout.sidebar = WindowSidebar::default();
        serde_json::to_string(&layout).unwrap_or_default()
    }
}

impl WorkspaceModel {
    /// What a session saves of itself; its notes and tasks are the Notes tab's own rows.
    pub(super) fn session_record(&self, session: &Session) -> SessionRecord {
        SessionRecord {
            id: session.id,
            name: session.custom_name.clone(),
            pinned: session.pinned,
            forget_text: session.forget_text,
            agent_hidden: session.agent_hidden,
            done_unseen: session.facts.status == Status::Done,
            cwd: session.facts.cwd.as_ref().map(|cwd| cwd.to_string_lossy().into_owned()),
            shell: session.shell.clone(),
            split_id: session.split,
            ..Default::default()
        }
    }

    /// This window as `Store::save_window` takes it: terminals in the window's order, the splits they name, the
    /// selected one's place and the layout. `bounds` is the shell's own record of where the window is.
    pub fn window_record(&self, window: i64, bounds: Option<String>) -> WindowRecord {
        let mut sidebar = self.sidebar.clone();
        sidebar.groups.retain_terminals(|id| self.index_of(id).is_some());
        let layout = SavedLayout::new(self.chrome.panes, self.chrome.panel_tab, sidebar);
        WindowRecord {
            id: window,
            active: self.selected_index().unwrap_or(0),
            layout: serde_json::to_string(&layout).ok(),
            bounds,
            sessions: self.sessions.iter().map(|session| self.session_record(session)).collect(),
            splits: self
                .splits
                .iter()
                .map(|split| SplitRecord { id: split.id, layout: Some(split.layout.to_json()) })
                .collect(),
        }
    }

    /// Fill an empty window from `record`: a terminal for each of its sessions (ids kept, each `Opened`), its
    /// splits, groups and columns, and the selection. A window that already has terminals is left alone.
    pub fn restore(&mut self, record: &WindowRecord) -> Vec<Event> {
        if self.sessions.is_empty() && !record.sessions.is_empty() {
            self.take_in(&record.sessions, &record.splits, true);
            if let Some(layout) =
                record.layout.as_deref().and_then(|json| serde_json::from_str::<SavedLayout>(json).ok())
            {
                self.chrome.panes = layout.panes();
                self.chrome.panel_tab = layout.right_tab;
                let alive = self.ids();
                self.sidebar = layout.sidebar;
                self.sidebar.groups.retain_terminals(|id| alive.contains(&id));
            }
            let selected = record.sessions.get(record.active).unwrap_or(&record.sessions[0]).id;
            self.rearranged();
            self.emit(Event::Layout);
            self.emit(Event::Chrome);
            self.select_id(selected);
        }
        self.out.take()
    }

    /// Add `records` to the window, ids kept: `start` makes each a terminal to start a program for (`Opened`),
    /// else they are running already (`Adopted`). Splits whose members are all here come back with their layout.
    pub(super) fn take_in(&mut self, records: &[SessionRecord], splits: &[SplitRecord], start: bool) {
        for record in records {
            let number = self.next_number();
            let mut session = Session::new(record.id, number);
            session.custom_name = record.name.clone();
            session.pinned = record.pinned;
            session.forget_text = record.forget_text;
            session.agent_hidden = record.agent_hidden;
            session.split = record.split_id;
            session.shell.clone_from(&record.shell);
            session.done_unseen = start && record.done_unseen;
            let cwd = record.cwd.as_deref().map(PathBuf::from);
            session.facts.cwd = cwd.clone();
            self.sessions.push(session);
            self.keep_ids_above(record.id);
            self.emit(if start {
                Event::Opened { id: record.id, cwd }
            } else {
                Event::Adopted { id: record.id }
            });
        }
        for split in splits {
            let members: Vec<i64> =
                records.iter().filter(|r| r.split_id == Some(split.id)).map(|r| r.id).collect();
            self.keep_ids_above(split.id);
            match TabPanes::restored(split.layout.as_deref(), &members).filter(|panes| panes.is_split()) {
                Some(layout) => self.splits.push(Split { id: split.id, layout }),
                None => self.forget_split(split.id),
            }
        }
        // A split nobody saved is no split: its members stand alone.
        let known: Vec<_> = self.splits.iter().map(|split| split.id).collect();
        for session in &mut self.sessions {
            if session.split.is_some_and(|split| !known.contains(&split)) {
                session.split = None;
            }
        }
    }

    fn forget_split(&mut self, id: i64) {
        for session in self.sessions.iter_mut().filter(|session| session.split == Some(id)) {
            session.split = None;
        }
    }
}
