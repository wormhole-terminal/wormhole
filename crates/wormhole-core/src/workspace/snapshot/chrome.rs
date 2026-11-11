//! The window's chrome as a record: the panel's tabs, the columns' widths and the rules that fit
//! them to the window, the layout presets with whether the selected split has room for each, and
//! the selection's state. Everything a native title bar, toolbar and menu need to draw and enable
//! themselves without a second copy of `view::layout` and `view::panes`.

use crate::config::tab::TabId;
use crate::view::layout::{
    Columns, Edge, PANEL_MAX, PANEL_MIN, PanelToggle, SIDEBAR_MAX, SIDEBAR_MIN, Stop, TERMINAL_MIN,
    WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
};
use crate::view::panes::{Axis, Preset};
use crate::view::sidebar::SidebarNumbers;

use super::super::action::Overlay;
use super::super::model::WorkspaceModel;

/// A tab of the right panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelTab {
    pub tab: TabId,
    /// What a layout saves it as and devctl calls it ("changes").
    pub key: String,
    /// What the tab says.
    pub label: String,
    /// The feature that brings it is on.
    pub enabled: bool,
    /// It is the one open.
    pub selected: bool,
}

/// A layout of View → Layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetInfo {
    pub preset: Preset,
    pub title: String,
    pub slots: u32,
    /// The selected split has the shape of it (the menu's check mark).
    pub current: bool,
    /// The area has room for its panes at their minimum size.
    pub enabled: bool,
}

/// What the panes can do now: the presets and the splits.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutInfo {
    /// In the menu's order.
    pub presets: Vec<PresetInfo>,
    /// The focused pane has room for a split beside it (⌘D) or below it (⇧⌘D).
    pub can_split_row: bool,
    pub can_split_column: bool,
    /// The selected terminal is in a split, with this many panes (1 when it stands alone).
    pub pane_count: u32,
    pub zoomed: bool,
}

/// Where a drag on a column's edge leaves it, before it is let go (core's `layout::resize`).
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeDrag {
    /// Still open: not dragged past its minimum by the slack that hides it.
    pub open: bool,
    pub width: f32,
    /// Why it stopped following the pointer, when it did.
    pub stop: Option<Stop>,
    /// What the readout beside the pointer says ("430 · max", "Close").
    pub readout: String,
}

/// The least and the most each column takes, and the smallest window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnLimits {
    pub sidebar_min: f32,
    pub sidebar_max: f32,
    pub panel_min: f32,
    pub panel_max: f32,
    pub terminal_min: f32,
    pub window_min_width: f32,
    pub window_min_height: f32,
}

/// The window's chrome the list and the panel depend on.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeSnapshot {
    pub revision: u64,
    /// The sidebar is open (its chosen width is kept while it is hidden).
    pub sidebar_visible: bool,
    /// The panel is on screen: wanted, and the window has room for it.
    pub panel_visible: bool,
    pub panel_tab: TabId,
    pub search: String,
    pub overlay: Option<Overlay>,
    /// The person wants the panel, whether or not the window has room now.
    pub panel_wanted: bool,
    /// How the panel's toggle looks: open, closed, or wanted and waiting for room.
    pub panel_toggle: PanelToggle,
    /// The window's width as the shell last said (0: not yet).
    pub window_width: f32,
    /// What the columns take in this window, a hidden one 0 wide.
    pub sidebar_width: f32,
    pub terminal_width: f32,
    pub panel_width: f32,
    /// The widths the person chose, kept while a column gives way or is hidden.
    pub sidebar_chosen_width: f32,
    pub panel_chosen_width: f32,
    /// The widest each may be dragged in this window.
    pub sidebar_max_now: f32,
    pub panel_max_now: f32,
    pub limits: ColumnLimits,
    /// Every tab, in the panel's order.
    pub tabs: Vec<PanelTab>,
    pub layout: LayoutInfo,
    pub selected: Option<i64>,
    pub zoomed: Option<i64>,
    /// ⌘1 to ⌘9 count along the list as shown, not with their terminal.
    pub numbers_follow_list: bool,
}

impl WorkspaceModel {
    pub fn chrome_snapshot(&self) -> ChromeSnapshot {
        let chrome = &self.chrome;
        let panes = &chrome.panes;
        let width = chrome.width();
        let known = chrome.window_width > 0.0;
        let columns = if known {
            panes.columns(width)
        } else {
            Columns {
                sidebar: if panes.sidebar_open { panes.sidebar_width } else { 0.0 },
                terminal: 0.0,
                panel: if panes.panel_wanted { panes.panel_width } else { 0.0 },
            }
        };
        let split =
            self.selected_index().and_then(|ix| self.sessions[ix].split).and_then(|id| self.split_of(id));
        ChromeSnapshot {
            revision: self.revisions().chrome,
            sidebar_visible: panes.sidebar_open,
            panel_visible: panes.panel_visible(width),
            panel_tab: chrome.panel_tab,
            search: chrome.search.clone(),
            overlay: chrome.overlay,
            panel_wanted: panes.panel_wanted,
            panel_toggle: panes.panel_toggle(width),
            window_width: chrome.window_width,
            sidebar_width: columns.sidebar,
            terminal_width: columns.terminal,
            panel_width: columns.panel,
            sidebar_chosen_width: panes.sidebar_width,
            panel_chosen_width: panes.panel_width,
            sidebar_max_now: panes.max_width(Edge::Sidebar, width),
            panel_max_now: panes.max_width(Edge::Panel, width),
            limits: ColumnLimits {
                sidebar_min: SIDEBAR_MIN,
                sidebar_max: SIDEBAR_MAX,
                panel_min: PANEL_MIN,
                panel_max: PANEL_MAX,
                terminal_min: TERMINAL_MIN,
                window_min_width: WINDOW_MIN_WIDTH,
                window_min_height: WINDOW_MIN_HEIGHT,
            },
            tabs: self
                .tabs()
                .into_iter()
                .map(|(tab, enabled)| PanelTab {
                    tab,
                    key: tab.key().to_string(),
                    // text: the tab's name is its key, capitalised, as the panel has always said it.
                    label: capitalised(tab.key()),
                    enabled,
                    selected: tab == chrome.panel_tab,
                })
                .collect(),
            layout: LayoutInfo {
                presets: self.preset_infos(),
                can_split_row: self.selected_index().is_some_and(|ix| self.can_split(ix, Axis::Row)),
                can_split_column: self.selected_index().is_some_and(|ix| self.can_split(ix, Axis::Column)),
                pane_count: split.map_or(1, |split| split.layout.len() as u32),
                zoomed: split.is_some_and(|split| split.layout.zoomed().is_some()),
            },
            selected: self.selected,
            zoomed: split.and_then(|split| split.layout.zoomed()),
            numbers_follow_list: self.cfg.numbers == SidebarNumbers::List,
        }
    }

    /// Where a drag of `edge` with the pointer `extent` from the window edge it hangs on would leave
    /// the column, without moving it: the line a shell draws beside the pointer while it is down.
    pub fn edge_drag(&self, edge: Edge, extent: f32) -> EdgeDrag {
        let mut panes = self.chrome.panes;
        let resize = panes.drag(edge, extent, self.chrome.width());
        EdgeDrag { open: resize.open, width: resize.width, stop: resize.stop, readout: resize.readout(edge) }
    }

    fn preset_infos(&self) -> Vec<PresetInfo> {
        let current = self.current_preset();
        Preset::ALL
            .into_iter()
            .map(|preset| PresetInfo {
                preset,
                title: preset.title().to_string(),
                slots: preset.slots() as u32,
                current: current == Some(preset),
                enabled: self.can_layout(preset),
            })
            .collect()
    }
}

fn capitalised(key: &str) -> String {
    let mut chars = key.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}
