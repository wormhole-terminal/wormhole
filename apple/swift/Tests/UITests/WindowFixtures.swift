// Windows for the tests: loose terminals as the store would list them, over a backend that also keeps splits (the sample
// backend answers none; the core's rules are tested against the core, not rewritten here).

import WorkspaceModel

extension WorkspaceState {
    /// A state of loose terminals in the order given.
    init(
        terminals: [TerminalSummary], selected: TerminalID?, sidebarVisible: Bool = true, panelVisible: Bool = true,
        panelTab: PanelTab = .changes
    ) {
        self.init(
            sections: MockWorkspace.sections(terminals.map { .loose(MockTerminal($0)) }, selected: selected), selected: selected,
            sidebarVisible: sidebarVisible, panelVisible: panelVisible, panelTab: panelTab)
    }
}

/// Loose terminals that can be split, joined and left: the least a pane layout needs from the workspace.
@MainActor
final class SplittingBackend: WorkspaceBackend {
    var onWake: (@MainActor () -> Void)?
    private var terminals: [TerminalSummary]
    private var selected: TerminalID?
    private var count = Revisions(sidebar: 1, chrome: 1)
    var live: [TerminalID] { terminals.map(\.id) }

    init(_ ids: [TerminalID], selected: TerminalID, split: [TerminalID] = []) {
        terminals = ids.map { TerminalSummary(id: $0, title: "t\($0)", splitID: split.contains($0) ? 1 : nil) }
        self.selected = selected
    }

    func drain() {}
    func revisions() -> Revisions { count }
    func chrome() -> ChromeState { ChromeState() }
    func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan? { nil }
    func sidebar() -> WorkspaceModel.SidebarSnapshot {
        WorkspaceModel.SidebarSnapshot(sections: MockWorkspace.sections(terminals.map { .loose(MockTerminal($0)) }, selected: selected), selected: selected)
    }

    func apply(_ action: WorkspaceAction) -> Refusal? {
        switch action {
        case .select(let id): selected = id
        case .splitRight(let anchor):
            let split = terminals.first { $0.id == anchor }?.splitID ?? (terminals.compactMap(\.splitID).max() ?? 0) + 1
            set(anchor, split: split)
            let id = (live.max() ?? 0) + 1
            terminals.append(TerminalSummary(id: id, title: "t\(id)", splitID: split))
            selected = id
        case .joinSplit(let id, let target):
            let split = terminals.first { $0.id == target }?.splitID ?? (terminals.compactMap(\.splitID).max() ?? 0) + 1
            set(target, split: split)
            set(id, split: split)
        case .leaveSplit(let id): set(id, split: nil)
        default: return .unchanged
        }
        count.sidebar += 1
        return nil
    }

    private func set(_ id: TerminalID, split: Int?) {
        if let at = terminals.firstIndex(where: { $0.id == id }) { terminals[at].splitID = split }
    }
}
