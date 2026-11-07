// replaced by WorkspaceStore at merge: the least state this target needs to run the window now.

import Observation
import WorkspaceModel

@MainActor @Observable
final class LocalStore {
    private(set) var state: WorkspaceState
    private var nextID: TerminalID

    init(terminals: [TerminalSummary], collapsed: Bool) {
        state = WorkspaceState(
            terminals: terminals, selected: terminals.first?.id, sidebarVisible: !collapsed, panelVisible: !collapsed)
        nextID = (terminals.map(\.id).max() ?? 0) + 1
    }

    func send(_ action: WorkspaceAction) {
        switch action {
        case .select(let id) where state.terminals.contains { $0.id == id }: state.selected = id
        case .newTerminal:
            state.terminals.append(TerminalSummary(id: nextID, title: "Terminal \(nextID)"))
            state.selected = nextID
            nextID += 1
        case .close(let id): close(id)
        case .toggleSidebar: state.sidebarVisible.toggle()
        case .togglePanel: state.panelVisible.toggle()
        case .showPanelTab(let tab):
            state.panelTab = tab
            state.panelVisible = true
        default: break
        }
    }

    /// A terminal's own title (OSC 0/2) replaces its row's.
    func setTitle(_ title: String, of id: TerminalID) {
        guard let index = state.terminals.firstIndex(where: { $0.id == id }), !title.isEmpty else { return }
        state.terminals[index].title = title
    }

    private func close(_ id: TerminalID) {
        guard let index = state.terminals.firstIndex(where: { $0.id == id }) else { return }
        state.terminals.remove(at: index)
        if state.selected == id {
            state.selected = state.terminals.isEmpty ? nil : state.terminals[min(index, state.terminals.count - 1)].id
        }
    }
}
