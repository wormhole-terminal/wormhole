// The title row's branch chip: it shows the Changes tab and opens "compare with" (`features/git/changes/def.rs` `branch_chip`).

import WorkspaceModel

extension WindowHub {
    func pickBranch(of terminal: TerminalID) {
        if state.selected != terminal { send(.select(terminal)) }
        send(.showPanelTab(.changes))
        // The picker opens when Changes is on screen: now, or as it comes up.
        panel.changes.pickerRequested = true
    }
}
