// The core's panes as this window's values (core `PanesSnapshot`), and the zone and direction names both ways.

import WorkspaceModel
import WormholeBindings

extension PanesState {
    init(_ core: PanesSnapshot) {
        self.init(
            shown: core.panes.map(PanePlace.init), unzoomed: core.unzoomed.map(PanePlace.init), focused: core.focused.map(Int.init),
            zoomed: core.zoomed.map(Int.init), dividers: core.dividers.map(PaneDividerState.init))
    }
}

extension PanePlace {
    init(_ core: WormholeBindings.PaneRect) {
        self.init(Int(core.id), PaneRect(Double(core.x), Double(core.y), Double(core.w), Double(core.h)))
    }
}

extension PaneDividerState {
    init(_ core: DividerInfo) {
        self.init(
            path: core.path, index: core.index, axis: core.axis == .row ? .row : .column, position: Double(core.position),
            rect: PaneRect(Double(core.x), Double(core.y), Double(core.w), Double(core.h)))
    }
}

extension WormholeBindings.Zone {
    init(_ zone: WorkspaceModel.Zone) {
        switch zone {
        case .left: self = .left
        case .right: self = .right
        case .top: self = .top
        case .bottom: self = .bottom
        }
    }
}

extension WormholeBindings.Direction {
    init(_ direction: WorkspaceModel.Direction) {
        switch direction {
        case .left: self = .left
        case .right: self = .right
        case .up: self = .up
        case .down: self = .down
        }
    }
}
