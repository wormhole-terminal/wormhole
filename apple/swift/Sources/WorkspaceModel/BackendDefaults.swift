// What a backend with no rules of its own answers to the questions only the core can: nothing. The sample uses these.

import Foundation

extension WorkspaceBackend {
    public func groupDropPlan(group: GroupID, over: DropOver) -> Landing? { nil }
    /// Without a core's list of every terminal, the rows the sidebar lists are all there are.
    public func terminals() -> TerminalsSnapshot {
        TerminalsSnapshot(terminals: sidebar().sections.flatMap(\.rows).map(\.terminal))
    }
    public func title() -> TitleState? { nil }
    public func edgeDrag(_ edge: ColumnEdge, extent: Double) -> EdgeDragReadout? { nil }
    public func groupFolderOffer(group: GroupID) -> String? { nil }
    public func setWindowState(active: Bool, seen: Bool) {}
}
