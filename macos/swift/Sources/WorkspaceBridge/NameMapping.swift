// The names the window and the core spell differently: panel tabs, group colours, features, layouts, edges and the title
// row's keys. Each is a switch with no default, so a case added on either side fails to compile here.

import WorkspaceModel
import WormholeBindings

extension TabId {
    init(_ tab: WorkspaceModel.PanelTab) {
        switch tab {
        case .notes: self = .notes
        case .changes: self = .changes
        case .history: self = .history
        case .usage: self = .usage
        case .project: self = .project
        case .files: self = .files
        }
    }
}

extension WormholeBindings.Overlay {
    init(_ overlay: WorkspaceModel.Overlay) {
        switch overlay {
        case .palette: self = .palette
        case .shortcuts: self = .shortcuts
        }
    }
}

extension WormholeBindings.GroupColor {
    init(_ colour: WorkspaceModel.GroupColor) {
        switch colour {
        case .grey: self = .grey
        case .blue: self = .blue
        case .red: self = .red
        case .yellow: self = .yellow
        case .green: self = .green
        case .pink: self = .pink
        case .purple: self = .purple
        case .cyan: self = .cyan
        case .orange: self = .orange
        }
    }
}

extension WormholeBindings.Preset {
    init(_ preset: LayoutPresetID) {
        switch preset {
        case .single: self = .single
        case .twoColumns: self = .twoColumns
        case .twoRows: self = .twoRows
        case .grid: self = .grid
        }
    }
}

extension LayoutPresetID {
    init(_ core: WormholeBindings.Preset) {
        switch core {
        case .single: self = .single
        case .twoColumns: self = .twoColumns
        case .twoRows: self = .twoRows
        case .grid: self = .grid
        }
    }
}

extension WormholeBindings.Edge {
    init(_ edge: ColumnEdge) {
        switch edge {
        case .sidebar: self = .sidebar
        case .panel: self = .panel
        }
    }
}

extension FeatureId {
    init(_ key: FeatureKey) {
        switch key {
        case .changes: self = .changes
        case .history: self = .history
        case .notes: self = .notes
        case .usage: self = .usage
        case .project: self = .project
        case .worktrees: self = .worktrees
        case .files: self = .files
        }
    }
}

extension WormholeBindings.TitlePart {
    /// The key `config.json` and the core spell a part with (`branch_status`); nil for a key this core does not know.
    init?(key: String) {
        let parts: [(String, Self)] = [
            ("folder", .folder), ("branch", .branch), ("branch_status", .branchStatus), ("terminal", .terminal),
            ("terminal_status", .terminalStatus), ("space", .space), ("agent_gauge", .agentGauge), ("ports", .ports),
            ("pull_request", .pullRequest), ("open_in", .openIn), ("splits", .splits),
        ]
        guard let found = parts.first(where: { $0.0 == key }) else { return nil }
        self = found.1
    }

    var key: String {
        switch self {
        case .folder: "folder"
        case .branch: "branch"
        case .branchStatus: "branch_status"
        case .terminal: "terminal"
        case .terminalStatus: "terminal_status"
        case .space: "space"
        case .agentGauge: "agent_gauge"
        case .ports: "ports"
        case .pullRequest: "pull_request"
        case .openIn: "open_in"
        case .splits: "splits"
        }
    }
}
