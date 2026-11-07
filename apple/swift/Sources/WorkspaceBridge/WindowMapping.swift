// The core's other answers as this window's values: the window's own switches, a refusal, the revisions and the plan of a
// drop (`SnapshotMapping.swift` has the sidebar). Names the two modules share are written with their module.

import WorkspaceModel
import WormholeBindings

/// `panelVisible` is what the person wants: the core's own `panelVisible` is that and room for it, which `layout` says.
extension ChromeState {
    init(_ core: ChromeSnapshot) {
        self.init(
            sidebarVisible: core.sidebarVisible, panelVisible: core.panelWanted, panelTab: WorkspaceModel.PanelTab(core.panelTab),
            search: core.search, overlay: core.overlay.map(WorkspaceModel.Overlay.init), layout: ChromeLayout(core))
    }
}

extension WorkspaceModel.PanelTab {
    init(_ core: TabId) {
        switch core {
        case .notes: self = .notes
        case .history: self = .history
        case .changes: self = .changes
        case .usage: self = .usage
        case .project: self = .project
        case .files: self = .files
        }
    }
}

extension WorkspaceModel.Overlay {
    init(_ core: WormholeBindings.Overlay) {
        switch core {
        case .palette: self = .palette
        case .shortcuts: self = .shortcuts
        }
    }
}

extension WorkspaceModel.Refusal {
    init(_ core: WormholeBindings.Refusal) {
        switch core {
        case .unknown: self = .unknown
        case .tooSmall: self = .tooSmall
        case .unchanged: self = .unchanged
        }
    }
}

extension WorkspaceModel.Revisions {
    /// The layout's counter moves with the chrome's for the window, which pulls the panes with it.
    init(_ core: WormholeBindings.Revisions) {
        self.init(sidebar: core.sidebar, chrome: core.chrome &+ core.layout, terminals: core.terminals)
    }
}

extension WorkspaceModel.DropPlan {
    init(_ core: WormholeBindings.DropPlan) {
        switch core {
        case .move(let landing, let leaveSplit, let withMates):
            self = .move(WorkspaceModel.Landing(landing), leavesSplit: leaveSplit, withMates: withMates)
        case .join(let target): self = .join(Int(target))
        }
    }
}

extension WorkspaceModel.Landing {
    init(_ core: WormholeBindings.Landing) {
        switch core {
        case .top(let before): self = .top(before: before.map(TopItem.init))
        case .group(let group, let before): self = .group(group, before: before.map(Int.init))
        }
    }
}

extension TopItem {
    init(_ core: Item) {
        switch core {
        case .terminal(let id): self = .terminal(Int(id))
        case .group(let id): self = .group(id)
        }
    }
}
