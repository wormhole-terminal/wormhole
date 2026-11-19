// The title row's branch chip: it shows the Changes tab and opens "compare with" (`features/git/changes/def.rs` `branch_chip`).

import WorkspaceModel

extension WindowHub {
    func pickBranch(of terminal: TerminalID) {
        if state.selected != terminal { send(.select(terminal)) }
        send(.showPanelTab(.changes))
        // The picker opens when Changes is on screen: now, or as it comes up. A folder the tab already read as no repository
        // brings no list to open it in, and a request left standing would open it at some later one.
        let changes = panel.changes
        guard !(changes.terminal == terminal && changes.phase == .notRepository) else { return }
        changes.pickerFor = terminal
    }
}
