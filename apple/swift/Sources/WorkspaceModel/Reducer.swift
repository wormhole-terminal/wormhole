// What each action does to the window's state. Pure: the same state and action always give the same
// result, so every rule here is tested without a window.

import Foundation

public func reduce(_ state: WorkspaceState, _ action: WorkspaceAction) -> WorkspaceState {
    var state = state
    switch action {
    case .select(let id):
        // An id nothing has (a terminal that just closed) selects nothing new.
        if state.terminals.contains(where: { $0.id == id }) { state.selected = id }

    case .newTerminal:
        // The id is never reused while its neighbours live; the number in the title follows it.
        let id = (state.terminals.map(\.id).max() ?? 0) + 1
        state.terminals.append(TerminalSummary(id: id, title: "Terminal \(id)"))
        state.selected = id
        // A search that hides the new terminal would leave the selection invisible.
        state.search = ""

    case .close(let id):
        guard state.terminals.contains(where: { $0.id == id }) else { break }
        let listed = SidebarListing.order(of: state)
        state.terminals.removeAll { $0.id == id }
        for index in state.groups.indices { state.groups[index].members.removeAll { $0 == id } }
        if state.selected == id {
            // The row that slides into the closed one's place, else the one above it.
            let remaining = listed.filter { $0 != id }
            let place = listed.firstIndex(of: id) ?? 0
            state.selected = remaining.isEmpty ? nil : remaining[min(place, remaining.count - 1)]
        }

    case .togglePin(let id):
        if let index = state.terminals.firstIndex(where: { $0.id == id }) {
            state.terminals[index].isPinned.toggle()
        }

    case .toggleGroup(let id):
        if let index = state.groups.firstIndex(where: { $0.id == id }) {
            state.groups[index].isCollapsed.toggle()
        }

    case .toggleSidebar:
        state.sidebarVisible.toggle()

    case .togglePanel:
        state.panelVisible.toggle()

    case .showPanelTab(let tab):
        // Asking for a tab is asking to see it.
        state.panelTab = tab
        state.panelVisible = true

    case .search(let text):
        state.search = text
    }
    return state
}
