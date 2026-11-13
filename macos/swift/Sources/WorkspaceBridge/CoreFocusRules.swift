// The focus rules asked of the core (`focus*` functions, `view::focus`): the window holds the real focus and the facts,
// the core answers where it goes next. The two enums share names, so the core's are written out in full.

import PanesModel
import WorkspaceModel
import WormholeBindings

private typealias Area = WorkspaceModel.FocusArea
private typealias CoreArea = WormholeBindings.FocusArea

public struct CoreFocusRules: FocusRules {
    public init() {}

    private static let areas: [(Area, CoreArea)] = [
        (.titleRow, .titleRow), (.sidebar, .sidebar), (.tabs, .tabs), (.terminal, .terminal),
        (.panelTabs, .panelTabs), (.panelContent, .panelContent), (.statusBar, .statusBar),
    ]
    private static func core(_ area: Area) -> CoreArea { areas.first { $0.0 == area }!.1 }
    private static func area(_ core: CoreArea) -> Area { areas.first { $0.1 == core }!.0 }

    public func step(from: WorkspaceModel.FocusArea?, delta: Int, shown: Set<WorkspaceModel.FocusArea>) -> WorkspaceModel.FocusArea {
        Self.area(focusStep(from: from.map(Self.core), delta: Int32(delta), shown: shown.map(Self.core)))
    }

    public func tabStep(from: WorkspaceModel.FocusStop?, delta: Int, stops: [WorkspaceModel.FocusArea: Int]) -> WorkspaceModel.FocusStop {
        let at = from.map { WormholeBindings.FocusStop(area: Self.core($0.area), index: UInt32($0.index)) }
        let counts = stops.map { AreaStops(area: Self.core($0.key), count: UInt32($0.value)) }
        let to = focusTabStep(from: at, delta: Int32(delta), stops: counts)
        return WorkspaceModel.FocusStop(Self.area(to.area), Int(to.index))
    }

    public func escape(terminal: Bool, popup: Bool, detail: Bool) -> WorkspaceModel.EscapeRung {
        switch focusEscape(state: EscapeState(terminal: terminal, popup: popup, detail: detail)) {
        case .toShell: .toShell
        case .closePopup: .closePopup
        case .closeDetail: .closeDetail
        case .toTerminal: .toTerminal
        }
    }

    public func walk(_ items: [WorkspaceModel.ListItem], from: Int?, _ key: WorkspaceModel.ListKey) -> WorkspaceModel.ListMove {
        let core: WormholeBindings.ListKey =
            switch key {
            case .up: .up
            case .down: .down
            case .left: .left
            case .right: .right
            case .first: .first
            case .last: .last
            }
        let list = items.map { WormholeBindings.ListItem(level: UInt8(clamping: $0.level), fold: $0.isOpen) }
        return switch focusListWalk(items: list, at: from.map(UInt32.init), key: core) {
        case .to(let index): .to(Int(index))
        case .open(let index): .open(Int(index))
        case .fold(let index): .fold(Int(index))
        case .stay: .stay
        }
    }

    public func typeAhead(_ names: [String], from: Int?, _ typed: String) -> Int? {
        focusTypeAhead(names: names, at: from.map(UInt32.init), typed: typed).map(Int.init)
    }

    public func pressFoldsBand(space: Bool) -> Bool { focusPressFoldsBand(space: space) }
}

/// What the core decides about a window as a whole: where the keyboard goes, and what Zen mode shows.
public enum CoreWindowRules {
    public static func install() {
        Focus.install(CoreFocusRules())
        RowMotion.install(CoreRowMotion.tables)
        TabSets.install(CoreTabSetRules())
        StatusRules.install(CoreStatusRules())
        let zen = windowPreset(preset: .zen)
        Zen.install(
            WindowShown(titleRow: zen.titleRow, statusBar: zen.statusBar, minimap: zen.minimap, columnDividers: zen.columnDividers))
    }
}
