// The sample as a backend, so `--mock`, previews and tests run the same store and views as the real window. It answers
// only what a demo needs: selecting, a new terminal, closing, renaming, pinning, folding and the window's switches. Every
// other action says `unchanged`; a real window's rules are the core's.

import Foundation

@MainActor
public final class MockWorkspaceBackend: WorkspaceBackend {
    public var onWake: (@MainActor () -> Void)?
    private var items: [MockItem]
    private var selected: TerminalID?
    private var window: ChromeState
    private var count: Revisions

    public init(items: [MockItem] = MockWorkspace.items, selected: TerminalID? = 1, chrome: ChromeState = ChromeState()) {
        (self.items, self.selected, window, count) = (items, selected, chrome, Revisions(sidebar: 1, chrome: 1))
    }

    public var live: [TerminalID] { items.flatMap(members).map(\.terminal.id) }
    public func drain() {}
    public func revisions() -> Revisions { count }
    public func chrome() -> ChromeState { window }
    public func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan? { nil }

    public func sidebar() -> SidebarSnapshot {
        SidebarSnapshot(sections: MockWorkspace.sections(items, selected: selected), selected: selected)
    }

    public func apply(_ action: WorkspaceAction) -> Refusal? {
        switch action {
        case .select(let id): guard live.contains(id) else { return .unknown }; selected = id
        case .newTerminal:
            let id = (live.max() ?? 0) + 1
            items.append(.loose(MockTerminal(TerminalSummary(id: id, title: "Terminal \(id)"))))
            selected = id
        case .close(let id): close(id)
        case .rename(let id, let name): edit(id) { $0.terminal.title = name }
        case .togglePin(let id): edit(id) { $0.terminal.isPinned.toggle() }
        case .toggleGroup(let id): editGroup(id) { $0.isFolded.toggle() }
        case .toggleSidebar: window.sidebarVisible.toggle(); count.chrome += 1; return nil
        case .togglePanel: window.panelVisible.toggle(); count.chrome += 1; return nil
        case .showPanelTab(let tab): (window.panelTab, window.panelVisible) = (tab, true); count.chrome += 1; return nil
        case .search(let text): window.search = text; count.chrome += 1; return nil
        case .showOverlay(let overlay): window.overlay = overlay; count.chrome += 1; return nil
        default: return .unchanged
        }
        count.sidebar += 1
        return nil
    }

    private func members(_ item: MockItem) -> [MockTerminal] {
        switch item {
        case .loose(let mock): [mock]
        case .group(_, let list): list
        }
    }

    private func edit(_ id: TerminalID, _ change: (inout MockTerminal) -> Void) {
        items = items.map { item in
            switch item {
            case .loose(var mock) where mock.terminal.id == id: change(&mock); return .loose(mock)
            case .group(let head, var list):
                if let at = list.firstIndex(where: { $0.terminal.id == id }) { change(&list[at]) }
                return .group(head, list)
            default: return item
            }
        }
    }

    private func editGroup(_ id: GroupID, _ change: (inout GroupHead) -> Void) {
        items = items.map { item in
            guard case .group(var head, let list) = item, head.id == id else { return item }
            change(&head)
            return .group(head, list)
        }
    }

    private func close(_ id: TerminalID) {
        let before = live
        items = items.compactMap { item in
            switch item {
            case .loose(let mock): return mock.terminal.id == id ? nil : item
            case .group(let head, let list):
                let rest = list.filter { $0.terminal.id != id }
                return rest.isEmpty && !head.isPinned ? nil : .group(head, rest)
            }
        }
        // The window's rule is the core's; the sample just moves to the neighbour.
        if selected == id, let at = before.firstIndex(of: id) {
            let rest = live
            selected = rest.isEmpty ? nil : rest[min(at, rest.count - 1)]
        }
    }
}
