// The terminals as tabs when the person shows them there instead of in the sidebar. A group's own edits (rename, pin,
// ungroup, close) are the core's actions, sent by the sidebar itself; the one with no core action yet, a group's colour,
// is left out of its menu and popover (the window hands the sidebar no `GroupEdits`).

import PanesModel
import PanesUI
import SidebarUI
import WorkspaceModel

extension WindowHub {
    /// The tab bar for the terminals as they are: the groups as sets, the loose terminals as the last one.
    var tabBar: TabBarModel {
        let state = state
        // A folded group lists no rows, so its set is empty until it is opened in the sidebar.
        let sets = TabSets.of(
            groups: state.sections.compactMap { section in
                section.head.map { TabSet(groupID: $0.id, label: $0.name, members: section.rows.map(\.id)) }
            },
            loose: state.sections.filter { $0.head == nil }.flatMap(\.rows).map(\.id))
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
            newTerminal: { [unowned self] group in send(group.map { .newTerminalInGroup($0) } ?? .newTerminal) },
            openSet: { [unowned self] in openSet($0) },
            newGroup: { [unowned self] in send(.newGroup([])) },
            showGroupsInSidebar: { [unowned self] in actions.showTerminalsInSidebar() })
    }

    private func openSet(_ index: Int) {
        let bar = tabBar
        guard bar.sets.indices.contains(index), let first = bar.sets[index].members.first else { return }
        send(.select(first))
    }
}
