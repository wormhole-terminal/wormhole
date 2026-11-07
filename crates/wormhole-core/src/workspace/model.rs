//! The model: one window's terminals and everything the sidebar and the panes decide about them.
//! Its behaviour is in the `impl` blocks of the files beside it, one concern each; this file holds
//! the state, the one entry point (`apply`) and what the others share.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::tab::TabId;
use crate::config::{FeatureId, FeatureMask};
use crate::view::layout::Panes;
use crate::view::layout::title_parts::TitleRow;
use crate::view::panes::{Room, TabPanes, Zone};
use crate::view::sidebar::group_folder::{Fence, Place};
use crate::view::sidebar::order::FrozenNumbers;
use crate::view::sidebar::{ListInput, Listing, NUMBERED, SidebarNumbers, SplitId, Splits, WindowSidebar};

use super::action::{Action, Overlay};
use super::event::{Event, Out, Revisions};
use super::facts::TerminalFacts;
use super::git_feed::{ChangeRules, GitFeed};
use super::ground::Ground;
use super::session::Session;
use super::status::StatusBook;

/// What the shell knows about the machine and the settings (settings join the model later); a few
/// of them change while it runs, each through its own action.
#[derive(Clone, Debug)]
pub struct ModelConfig {
    /// The home folder, for the places on a row's second line (`~/Projects/app`).
    pub home: Option<PathBuf>,
    /// What ⌘1 to ⌘9 count along (Settings → Terminals & sidebar).
    pub numbers: SidebarNumbers,
    /// Where a terminal starts when none is selected to take the folder from.
    pub start_folder: Option<PathBuf>,
    /// What a pane's header adds to the least height of a pane (the grid's minimum sits under it); the
    /// shell's own metric, which the model has no way to know.
    pub pane_header_h: f32,
    /// What a row's ±N counts (Settings → Terminals & sidebar → Changed files).
    pub changes: ChangeRules,
    /// Which features are on: the panel offers the tabs they bring.
    pub features: FeatureMask,
    /// Which parts the title row shows and in what order (`title_row` in `config.json`).
    pub title_row: TitleRow,
    /// What the model asks of the disk (a group's fence); none: no group fences anything.
    pub ground: Option<Arc<dyn Ground>>,
}

impl Default for ModelConfig {
    /// Every feature on, as a first launch of a release has them.
    fn default() -> Self {
        Self {
            home: None,
            numbers: SidebarNumbers::default(),
            start_folder: None,
            pane_header_h: 0.0,
            changes: ChangeRules::default(),
            features: FeatureId::ALL.into_iter().fold(FeatureMask::default(), |mask, id| mask.with(id, true)),
            title_row: TitleRow::default(),
            ground: None,
        }
    }
}

/// A split: its id (one counter with the terminals') and the layout its members share.
#[derive(Clone, Debug)]
pub(super) struct Split {
    pub id: SplitId,
    pub layout: TabPanes,
}

/// The window's chrome that the list and the panel depend on: the side columns as the person left
/// them (core's `view::layout::Panes`) and the window's width, which decides whether the panel has
/// room.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Chrome {
    pub panes: Panes,
    /// The window's width in points; zero until the shell has laid it out, when nothing is refused
    /// for want of room.
    pub window_width: f32,
    pub panel_tab: TabId,
    pub search: String,
    pub overlay: Option<Overlay>,
}

impl Default for Chrome {
    fn default() -> Self {
        Self {
            panes: Panes::default(),
            window_width: 0.0,
            panel_tab: TabId::Changes,
            search: String::new(),
            overlay: None,
        }
    }
}

/// The width the layout rules are asked about while the window's is not known: room for everything.
const UNKNOWN_WIDTH: f32 = 1.0e6;

impl Chrome {
    /// The width to ask `Panes` about.
    pub(super) fn width(&self) -> f32 {
        if self.window_width > 0.0 { self.window_width } else { UNKNOWN_WIDTH }
    }
}

pub struct WorkspaceModel {
    pub(super) cfg: ModelConfig,
    /// In the order they were made (core's rules index terminals by this place).
    pub(super) sessions: Vec<Session>,
    pub(super) splits: Vec<Split>,
    pub(super) sidebar: WindowSidebar,
    pub(super) selected: Option<i64>,
    pub(super) chrome: Chrome,
    /// The area the panes share; zero until the shell has laid it out.
    pub(super) pane_area: (f32, f32),
    next_id: i64,
    next_number: usize,
    /// Bumped when the person rearranges the list, so ⌘ numbers that stay with their terminal are
    /// dealt again (`FrozenNumbers`).
    pub(super) order_rev: u64,
    frozen: RefCell<FrozenNumbers<i64>>,
    /// What the disk said of terminals' places and groups' fences (`fence.rs`).
    /// The terminal last selected in each group (`None`: among those in no group).
    pub(super) last_selected: HashMap<Option<u32>, i64>,
    pub(super) places: RefCell<HashMap<PathBuf, Option<Place>>>,
    pub(super) fences: RefCell<HashMap<String, Option<Fence>>>,
    revisions: Revisions,
    pub(super) out: Out,
    /// One status machine per terminal that has been told anything (`status`).
    pub(super) status: StatusBook,
    /// What git says of each terminal's folder (`git_feed`).
    pub(super) git: GitFeed,
}

impl WorkspaceModel {
    pub fn new(cfg: ModelConfig) -> Self {
        let git = GitFeed::new(cfg.changes);
        Self {
            cfg,
            sessions: Vec::new(),
            splits: Vec::new(),
            sidebar: WindowSidebar::default(),
            selected: None,
            chrome: Chrome::default(),
            pane_area: (0.0, 0.0),
            next_id: 1,
            next_number: 1,
            order_rev: 0,
            frozen: RefCell::default(),
            last_selected: HashMap::new(),
            places: RefCell::default(),
            fences: RefCell::default(),
            revisions: Revisions::default(),
            out: Out::default(),
            status: StatusBook::default(),
            git,
        }
    }

    /// Do what the person did. What changed, as events, for the shell to act on or pull from.
    pub fn apply(&mut self, action: Action) -> Vec<Event> {
        match action {
            Action::Select(id) => self.select_id(id),
            Action::SelectStep(delta) => self.select_step(delta as isize),
            Action::SelectNumber(n) => self.select_number(n as usize),
            Action::NewTerminal => self.new_terminal_selected(),
            Action::Close(id) => self.close_terminal(id),
            Action::Rename { id, name } => self.rename(id, &name),
            Action::SetPinned { id, pinned } => self.set_pinned(id, pinned),
            Action::TogglePin(id) => self.toggle_pin(id),
            Action::SetAgentHidden { id, hidden } => self.set_agent_hidden(id, hidden),
            Action::ToggleAgentHidden(id) => self.toggle_agent_hidden(id),
            Action::SetForgetText { id, forget } => self.set_forget_text(id, forget),
            Action::ToggleForgetText(id) => self.toggle_forget_text(id),
            Action::NewGroup { members } => self.new_group(&members),
            Action::RenameGroup { group, name } => self.rename_group(group, &name),
            Action::ToggleGroup(group) => self.toggle_group(group),
            Action::Ungroup(group) => self.ungroup(group),
            Action::SetGroupPinned { group, pinned } => self.set_group_pinned(group, pinned),
            Action::NewTerminalInNewGroup => self.new_terminal_in_new_group(),
            Action::NewTerminalInGroup(group) => self.new_terminal_in_group(group),
            Action::SetGroupColor { group, color } => self.set_group_color(group, color),
            Action::SetGroupCustomColor { group, r, g, b } => self.set_group_custom_color(group, [r, g, b]),
            Action::SetGroupFolder { group, folder } => self.set_group_folder(group, folder.as_deref()),
            Action::SetOnlyInFolder { group, on } => self.set_only_in_folder(group, on),
            Action::DeclineGroupFolder(group) => self.decline_group_folder(group),
            Action::CloseGroup(group) => self.close_group(group),
            Action::MoveGroupToWindow(group) => self.move_group_to_window(group),
            Action::AddToGroup { id, group } => self.add_to_group(id, group),
            Action::RemoveFromGroup(id) => self.remove_from_group(id),
            Action::DropTerminal { dragged, over } => self.drop_terminal(dragged, over),
            Action::DropGroup { group, over } => self.drop_group(group, over),
            Action::MoveTerminal { id, landing } => self.move_terminal(id, landing),
            Action::MoveTerminalAs { id, landing, leave_split, with_mates } => {
                self.move_terminal_as(id, landing, leave_split, with_mates)
            }
            Action::MoveGroup { group, landing } => self.move_group(group, landing),
            Action::SplitBeside { anchor, axis } => self.split_beside(anchor, axis),
            Action::JoinSplit { id, target, zone } => self.join_split(id, target, zone),
            Action::LeaveSplit(id) => self.leave_split(id),
            Action::ShowBeside { id, other } => self.join_split(other, id, Zone::Right),
            Action::ApplyLayout(preset) => self.apply_layout(preset),
            Action::DragDivider { path, index, position } => self.drag_divider(&path, index, position),
            Action::EqualizeDivider { path, index } => self.equalize_divider(&path, index),
            Action::FocusPane(id) => self.select_id(id),
            Action::ToggleZoom => self.toggle_zoom(),
            Action::ToggleSidebar => self.chrome_change(|c| c.panes.toggle_sidebar()),
            Action::TogglePanel => {
                self.chrome_change(|c| {
                    let width = c.width();
                    c.panes.toggle_panel(width);
                });
            }
            Action::ShowPanelTab(tab) => self.show_panel_tab(tab),
            Action::SetWindowWidth(width) => self.chrome_change(|c| c.window_width = width.max(0.0)),
            Action::DragEdge { edge, extent } => self.chrome_change(|c| {
                let width = c.width();
                c.panes.drag(edge, extent, width);
            }),
            Action::ResetEdge(edge) => self.chrome_change(|c| c.panes.reset(edge)),
            Action::SetTitleRow(parts) => {
                self.cfg.title_row = TitleRow::new(parts);
                self.emit(Event::Chrome);
            }
            Action::SetNumbersFollowList(follow) => self.set_numbers_follow_list(follow),
            Action::SetFeature { feature, on } => self.set_feature(feature, on),
            Action::Search(text) => self.search(text),
            Action::ShowOverlay(overlay) => self.chrome_change(|c| c.overlay = overlay),
            Action::SetPaneArea { width, height } => self.pane_area = (width.max(0.0), height.max(0.0)),
        }
        self.out.take()
    }

    /// The shell learned something of a terminal: replace all its facts. A change that shows on its
    /// row is a `Listing`. Unknown ids are ignored (a terminal that just closed).
    pub fn set_facts(&mut self, id: i64, facts: TerminalFacts) -> Vec<Event> {
        self.update_facts(id, |own| *own = facts)
    }

    /// Change some of a terminal's facts in place (the shell's events arrive one at a time).
    pub fn update_facts(&mut self, id: i64, change: impl FnOnce(&mut TerminalFacts)) -> Vec<Event> {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) {
            let before = session.facts.clone();
            change(&mut session.facts);
            if session.facts != before {
                let moved = session.facts.cwd != before.cwd;
                self.emit(Event::Listing);
                if moved {
                    self.folder_changed(id);
                }
            }
        }
        self.out.take()
    }

    /// The folder a new terminal starts in when none is selected to take it from.
    pub fn set_start_folder(&mut self, folder: Option<PathBuf>) {
        self.cfg.start_folder = folder;
    }

    pub fn selected(&self) -> Option<i64> {
        self.selected
    }

    pub fn revisions(&self) -> Revisions {
        self.revisions
    }

    /// Every terminal's id, in the order they were made.
    pub fn ids(&self) -> Vec<i64> {
        self.sessions.iter().map(|session| session.id).collect()
    }

    pub fn facts(&self, id: i64) -> Option<&TerminalFacts> {
        self.sessions.iter().find(|session| session.id == id).map(|session| &session.facts)
    }

    // ------------------------------------------------------------------ shared by the impls

    /// Tell the shell, and count the change in the part it belongs to.
    pub(super) fn emit(&mut self, event: Event) {
        match event {
            Event::Listing => {
                self.revisions.sidebar += 1;
                self.revisions.terminals += 1;
            }
            Event::Layout => {
                self.revisions.layout += 1;
                self.revisions.terminals += 1;
            }
            Event::Chrome => self.revisions.chrome += 1,
            _ => {}
        }
        self.out.push(event);
    }

    /// A change to the list the person made: the ⌘ numbers are dealt again.
    pub(super) fn rearranged(&mut self) {
        self.order_rev += 1;
        self.emit(Event::Listing);
    }

    pub(super) fn alloc_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(super) fn next_number(&mut self) -> usize {
        let number = self.next_number;
        self.next_number += 1;
        number
    }

    pub(super) fn index_of(&self, id: i64) -> Option<usize> {
        self.sessions.iter().position(|session| session.id == id)
    }

    pub(super) fn selected_index(&self) -> Option<usize> {
        self.selected.and_then(|id| self.index_of(id))
    }

    pub(super) fn split_of(&self, id: SplitId) -> Option<&Split> {
        self.splits.iter().find(|split| split.id == id)
    }

    pub(super) fn split_mut(&mut self, id: SplitId) -> Option<&mut Split> {
        self.splits.iter_mut().find(|split| split.id == id)
    }

    /// The terminals in split `id`, in the window's order.
    pub(super) fn members(&self, id: SplitId) -> Vec<usize> {
        (0..self.sessions.len()).filter(|&ix| self.sessions[ix].split == Some(id)).collect()
    }

    /// Terminal `id` and the rest of its split: they move, join and leave groups together.
    pub(super) fn with_split_mates(&self, id: i64) -> Vec<i64> {
        let Some(ix) = self.index_of(id) else { return Vec::new() };
        match self.sessions[ix].split {
            Some(split) => self.members(split).into_iter().map(|member| self.sessions[member].id).collect(),
            None => vec![id],
        }
    }

    pub(super) fn pinned_flags(&self) -> Vec<bool> {
        self.sessions.iter().map(|session| session.pinned).collect()
    }

    pub(super) fn split_flags(&self) -> Vec<Option<SplitId>> {
        self.sessions.iter().map(|session| session.split).collect()
    }

    pub(super) fn room(&self) -> Room {
        let room = Room::new(self.pane_area.0, self.pane_area.1);
        Room { min_h: room.min_h + self.cfg.pane_header_h, ..room }
    }

    /// The list as the sidebar draws it. Under "Stay with the terminal" the numbers the listing
    /// dealt are replaced by the frozen ones (the app's `Sidebar::freeze_numbers`).
    pub(super) fn listing(&self) -> Listing {
        let (ids, pinned, split_of) = (self.ids(), self.pinned_flags(), self.split_flags());
        let mut listing = Listing::build(&ListInput {
            ids: &ids,
            pinned: &pinned,
            sidebar: &self.sidebar,
            splits: Splits { split_of: &split_of, selected: self.selected_index() },
            numbers: self.cfg.numbers,
        });
        let mut frozen = self.frozen.borrow_mut();
        if self.cfg.numbers != SidebarNumbers::Terminal {
            frozen.reset();
            return listing;
        }
        let mut tag = DefaultHasher::new();
        self.order_rev.hash(&mut tag);
        self.sidebar.folded.hash(&mut tag);
        let window: Vec<(i64, bool)> = self.sessions.iter().map(|s| (s.id, s.pinned)).collect();
        let dealt: Vec<i64> = listing.visible().map(|ix| ids[ix]).collect();
        let held = frozen.settle(&window, tag.finish(), &dealt, NUMBERED);
        listing.set_numbers(held.iter().filter_map(|id| ids.iter().position(|known| known == id)).collect());
        listing
    }

    pub(super) fn chrome_change(&mut self, change: impl FnOnce(&mut Chrome)) {
        let before = self.chrome.clone();
        change(&mut self.chrome);
        if self.chrome != before {
            self.emit(Event::Chrome);
        }
    }

    fn search(&mut self, text: String) {
        if self.chrome.search != text {
            self.chrome.search = text;
            self.emit(Event::Chrome);
            self.emit(Event::Listing);
        }
    }
}
