// The sidebar's seams filled by the workspace's actions: the group's own edits (its menu and popover), and the
// terminals as tabs when the person shows them there instead.

import PanesModel
import PanesUI
import SidebarUI
import WorkspaceModel

extension WindowHub {
    /// A group's name, colour, pin, ungroup and close, as actions on the workspace.
    var groupEdits: GroupEdits {
        GroupEdits(
            rename: { [unowned self] in send(.renameGroup($0, $1)) },
            setColor: { [unowned self] in send(.setGroupColor($0, $1)) },
            togglePin: { [unowned self] in send(.pinGroup($0)) },
            ungroup: { [unowned self] in send(.ungroup($0)) },
            close: { [unowned self] in send(.closeGroup($0)) })
    }

    /// The tab bar for the terminals as they are: the groups as sets, the loose terminals as the last one.
    var tabBar: TabBarModel {
        let state = state
        let grouped = Set(state.groups.flatMap(\.members))
        let sets = TabSets.of(
            groups: state.groups.map { TabSet(groupID: $0.id, label: $0.name, members: $0.members) },
            loose: state.terminals.map(\.id).filter { !grouped.contains($0) })
        let current = state.selected.flatMap { TabSets.current(sets, containing: $0) } ?? 0
        return TabBarModel.make(
            sets: sets, current: current, selected: state.selected,
            terminal: { id in
                let terminal = state.terminals.first { $0.id == id }
                return TabItem(
                    id: id, name: terminal?.title ?? "", activity: terminal?.activity ?? .idle,
                    number: state.terminals.firstIndex { $0.id == id }.map { $0 + 1 })
            },
            pinned: { set in state.groups.first { $0.id == set.groupID }?.isPinned ?? false })
    }

    /// What the bar's controls do (the one-line forwards of `TabBarActions`).
    var tabActions: TabBarActions {
        TabBarActions(
            select: { [unowned self] in send(.select($0)) },
            close: { [unowned self] in actions.closeTerminal($0) },
            newTerminal: { [unowned self] group in
                send(.newTerminal)
                if let group, let id = state.selected { send(.addToGroup(id, group)) }
            },
            openSet: { [unowned self] in openSet($0) },
            newGroup: { [unowned self] in send(.newGroup("")) },
            showGroupsInSidebar: { [unowned self] in actions.showTerminalsInSidebar() })
    }

    private func openSet(_ index: Int) {
        let bar = tabBar
        guard bar.sets.indices.contains(index), let first = bar.sets[index].members.first else { return }
        send(.select(first))
    }
}
