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
        state.addTerminal()

    case .close(let id):
        state.close([id])

    case .showOverlay(let overlay):
        state.overlay = overlay

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

    case .rename(let id, let name):
        state.rename(id, to: name)

    case .newGroup(let name):
        state.addGroup(named: name)

    case .addToGroup(let id, let group):
        state.move(id, to: .group(group, before: nil))

    case .moveTerminal(let id, let landing):
        state.move(id, to: landing)

    case .moveGroup(let id, let before):
        state.moveGroup(id, before: before)

    case .splitRight(let id):
        state.splitRight(id)

    case .renameGroup(let id, let name):
        state.renameGroup(id, to: name)

    case .setGroupColor(let id, let color):
        if let index = state.groups.firstIndex(where: { $0.id == id }) { state.groups[index].color = color }

    case .setGroupFolder(let id, let folder):
        state.setGroupFolder(id, to: folder)

    case .pinGroup(let id):
        state.togglePinGroup(id)

    case .ungroup(let id):
        state.ungroup(id)

    case .closeGroup(let id):
        state.closeGroup(id)

    case .removeFromGroup(let id):
        state.removeFromGroup(id)

    case .newTerminalInGroup(let id):
        state.addTerminal(inGroup: id)

    case .newGroupWithTerminal(let id, let name):
        state.addGroup(holding: id, named: name)

    case .joinSplit(let id, let other):
        state.join(id, with: other)

    case .leaveSplit(let id):
        state.leaveSplit(id)

    case .toggleForgetText(let id):
        state.toggle(id, \.forgetsText)

    case .toggleAgentHidden(let id):
        state.toggle(id, \.isHiddenFromAgents)
    }
    return state
}
