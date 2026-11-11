// The window's columns and menus as the core fits them (core `ChromeSnapshot`): the widths of the three columns for the
// window's width, whether the panel has room, the tabs the features on offer, and the layouts View > Layout lists. The
// window draws these; it re-derives nothing. A backend with no window width yet (the sample) leaves `ChromeState.layout` nil.

import Foundation

/// The two draggable edges.
public enum ColumnEdge: Sendable, Equatable { case sidebar, panel }

public enum PanelToggle: Sendable, Equatable {
    case open
    /// Closed; opening it makes the sidebar step aside when `hidesSidebar`.
    case closed(hidesSidebar: Bool)
    /// Wanted but without room.
    case waiting
}

/// A tab the panel offers (core `PanelTab`): a tab of a feature that is off is not enabled.
public struct PanelTabItem: Sendable, Equatable {
    public var tab: PanelTab
    public var label: String
    public var enabled: Bool
    public var selected: Bool

    public init(tab: PanelTab, label: String, enabled: Bool = true, selected: Bool = false) {
        (self.tab, self.label, self.enabled, self.selected) = (tab, label, enabled, selected)
    }
}

public enum LayoutPresetID: String, Sendable, CaseIterable { case single, twoColumns, twoRows, grid }

public struct LayoutPreset: Sendable, Equatable {
    public var id: LayoutPresetID
    public var title: String
    public var slots: Int
    public var current: Bool
    public var enabled: Bool

    public init(id: LayoutPresetID, title: String, slots: Int, current: Bool = false, enabled: Bool = true) {
        (self.id, self.title, self.slots, self.current, self.enabled) = (id, title, slots, current, enabled)
    }
}

/// What View > Layout and the split buttons offer (core `LayoutInfo`).
public struct LayoutMenu: Sendable, Equatable {
    public var presets: [LayoutPreset] = []
    public var canSplitRow = false
    public var canSplitColumn = false
    public var paneCount = 1
    public var zoomed = false

    public init(
        presets: [LayoutPreset] = [], canSplitRow: Bool = false, canSplitColumn: Bool = false, paneCount: Int = 1, zoomed: Bool = false
    ) {
        (self.presets, self.canSplitRow, self.canSplitColumn, self.paneCount, self.zoomed) =
            (presets, canSplitRow, canSplitColumn, paneCount, zoomed)
    }
}

/// The column rules' constants (core `view::layout`), for a drag and for the window's least size.
public struct ColumnLimits: Sendable, Equatable {
    public var sidebarMin, sidebarMax, panelMin, panelMax, terminalMin, windowMinWidth, windowMinHeight: Double

    public init(
        sidebarMin: Double, sidebarMax: Double, panelMin: Double, panelMax: Double, terminalMin: Double,
        windowMinWidth: Double, windowMinHeight: Double
    ) {
        (self.sidebarMin, self.sidebarMax, self.panelMin, self.panelMax) = (sidebarMin, sidebarMax, panelMin, panelMax)
        (self.terminalMin, self.windowMinWidth, self.windowMinHeight) = (terminalMin, windowMinWidth, windowMinHeight)
    }
}

/// The three columns' widths for the window's width: a hidden or yielding column is 0.
public struct Columns: Sendable, Equatable {
    public var window, sidebar, terminal, panel: Double
    /// What the person chose, which a narrow window overrules for now.
    public var sidebarChosen, panelChosen: Double
    /// The widest each may be dragged now.
    public var sidebarMaxNow, panelMaxNow: Double
    public var limits: ColumnLimits

    public init(
        window: Double, sidebar: Double, terminal: Double, panel: Double, sidebarChosen: Double, panelChosen: Double,
        sidebarMaxNow: Double, panelMaxNow: Double, limits: ColumnLimits
    ) {
        (self.window, self.sidebar, self.terminal, self.panel) = (window, sidebar, terminal, panel)
        (self.sidebarChosen, self.panelChosen, self.sidebarMaxNow, self.panelMaxNow) = (sidebarChosen, panelChosen, sidebarMaxNow, panelMaxNow)
        self.limits = limits
    }
}

/// Everything of the core's `ChromeSnapshot` beyond the switches.
public struct ChromeLayout: Sendable, Equatable {
    public var columns: Columns
    public var panelToggle: PanelToggle
    public var tabs: [PanelTabItem]
    public var menu: LayoutMenu
    public var zoomed: TerminalID?
    public var numbersFollowList: Bool

    public init(
        columns: Columns, panelToggle: PanelToggle, tabs: [PanelTabItem], menu: LayoutMenu, zoomed: TerminalID? = nil,
        numbersFollowList: Bool = false
    ) {
        (self.columns, self.panelToggle, self.tabs, self.menu) = (columns, panelToggle, tabs, menu)
        (self.zoomed, self.numbersFollowList) = (zoomed, numbersFollowList)
    }
}

/// What a drag of a column's edge would leave, without moving it (core `EdgeDrag`): the readout beside the pointer.
public struct EdgeDragReadout: Sendable, Equatable {
    public var open: Bool
    public var width: Double
    public var readout: String

    public init(open: Bool, width: Double, readout: String) { (self.open, self.width, self.readout) = (open, width, readout) }
}

/// The features a panel tab can come from, switchable at run time (core `FeatureId`).
public enum FeatureKey: String, Sendable, CaseIterable {
    case changes, history, notes, usage, project, worktrees, files
}
