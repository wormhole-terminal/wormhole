// The core's tab sets (`Workspace.tab_sets`) as the window's values.

import WorkspaceModel
import WormholeBindings

extension TabSetFacts {
    init(_ core: TabSetsView) {
        self.init(
            sets: core.sets.map { TabSetFacts.Entry(group: $0.group, label: $0.label, members: $0.members.map(Int.init), alert: $0.alert.map(WorkspaceModel.Urgency.init)) },
            current: core.current.map(Int.init), othersAlert: core.othersAlert.map(WorkspaceModel.Urgency.init))
    }
}
