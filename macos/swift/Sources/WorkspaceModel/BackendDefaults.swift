// What a backend with no rules of its own answers to the questions only the core can: nothing. The sample uses these.

import Foundation

extension WorkspaceBackend {
    public func groupDropPlan(group: GroupID, over: DropOver) -> Landing? { nil }
    /// Without a core's list of every terminal, the rows the sidebar lists are all there are.
    public func terminals() -> TerminalsSnapshot {
        TerminalsSnapshot(terminals: sidebar().sections.flatMap(\.rows).map(\.terminal))
    }
    public func panes(width: Double, height: Double) -> PanesState { PanesState() }
    public func paneDropRefusal(dragged: TerminalID, target: TerminalID, zone: Zone) -> Refusal? { .unchanged }
    public func title() -> TitleState? { nil }
    public func edgeDrag(_ edge: ColumnEdge, extent: Double) -> EdgeDragReadout? { nil }
    public func groupFolderOffer(group: GroupID) -> String? { nil }
    public func groupAdmits(_ group: GroupID, terminal: TerminalID) -> Bool { true }
    public func lastSelected(in group: GroupID?) -> TerminalID? { nil }
    public func tabLabels(of ids: [TerminalID]) -> [String]? { nil }
    public func setWindowState(active: Bool, seen: Bool) {}
    public func settingsChanged() {}
    public func hosting(of id: TerminalID) -> HostingState? { nil }
    public func waited(since: UInt64) -> Waited? { nil }
    public func programName(of id: TerminalID) -> String? { nil }
    public func shellName(of id: TerminalID) -> String? { nil }
    public func hostProblem(of id: TerminalID) -> HostProblem? { nil }
    public func reconnect(_ id: TerminalID) {}
    public func endHost(of id: TerminalID, hard: Bool, done: @escaping @MainActor (String?) -> Void) { done(nil) }
    public func startAgain(_ id: TerminalID) {}
    public func finishQuit(endHosts: Bool) {}
    /// The sample saves nothing and runs no programs to hand over.
    public func restore() {}
    public func takeOut(_ ids: [TerminalID]) -> TerminalDeparture? { nil }
    public func receive(_ departure: TerminalDeparture) {}
}
