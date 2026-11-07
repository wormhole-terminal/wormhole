// The window's columns, tabs and layouts as the core fitted them (core `ChromeSnapshot`'s fields beyond the switches).

import WorkspaceModel
import WormholeBindings

extension ChromeLayout {
    init(_ core: ChromeSnapshot) {
        self.init(
            columns: Columns(core), panelToggle: WorkspaceModel.PanelToggle(core.panelToggle),
            tabs: core.tabs.map(PanelTabItem.init), menu: LayoutMenu(core.layout), zoomed: core.zoomed.map(Int.init),
            numbersFollowList: core.numbersFollowList)
    }
}

extension Columns {
    init(_ core: ChromeSnapshot) {
        let limits = core.limits
        self.init(
            window: Double(core.windowWidth), sidebar: Double(core.sidebarWidth), terminal: Double(core.terminalWidth),
            panel: Double(core.panelWidth), sidebarChosen: Double(core.sidebarChosenWidth),
            panelChosen: Double(core.panelChosenWidth), sidebarMaxNow: Double(core.sidebarMaxNow),
            panelMaxNow: Double(core.panelMaxNow),
            limits: WorkspaceModel.ColumnLimits(
                sidebarMin: Double(limits.sidebarMin), sidebarMax: Double(limits.sidebarMax), panelMin: Double(limits.panelMin),
                panelMax: Double(limits.panelMax), terminalMin: Double(limits.terminalMin),
                windowMinWidth: Double(limits.windowMinWidth), windowMinHeight: Double(limits.windowMinHeight)))
    }
}

extension WorkspaceModel.PanelToggle {
    init(_ core: WormholeBindings.PanelToggle) {
        switch core {
        case .open: self = .open
        case .closed(let hidesSidebar): self = .closed(hidesSidebar: hidesSidebar)
        case .waiting: self = .waiting
        }
    }
}

extension PanelTabItem {
    init(_ core: WormholeBindings.PanelTab) {
        self.init(tab: WorkspaceModel.PanelTab(core.tab), label: core.label, enabled: core.enabled, selected: core.selected)
    }
}

extension LayoutMenu {
    init(_ core: LayoutInfo) {
        self.init(
            presets: core.presets.map { LayoutPreset(id: LayoutPresetID($0.preset), title: $0.title, slots: Int($0.slots), current: $0.current, enabled: $0.enabled) },
            canSplitRow: core.canSplitRow, canSplitColumn: core.canSplitColumn, paneCount: Int(core.paneCount), zoomed: core.zoomed)
    }
}

extension EdgeDragReadout {
    init(_ core: EdgeDrag) { self.init(open: core.open, width: Double(core.width), readout: core.readout) }
}
