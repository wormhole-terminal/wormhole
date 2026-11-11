// A terminal's status changing in the list: which tiles went from one state to another, counted per terminal so the row's
// wash (and anything else that plays once per change) runs from its start each time. A list at rest compares one
// dictionary and nothing else; Reduce Motion and Classic play nothing, which the rows decide from the count.

import SwiftUI
import WorkspaceModel

extension SidebarView {
    var statuses: [TerminalID: TileState] {
        Dictionary(state.terminals.map { ($0.id, appearance.showsStatus ? TileState($0) : .shell) }, uniquingKeysWith: { first, _ in first })
    }

    func noteStatusChanges(from old: [TerminalID: TileState], to new: [TerminalID: TileState]) {
        for (id, status) in new where old[id].map({ $0 != status }) == true { shifts[id, default: 0] += 1 }
    }
}
