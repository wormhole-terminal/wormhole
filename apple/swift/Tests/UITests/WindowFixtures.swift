// Windows for the tests: loose terminals as the store would list them.

import WorkspaceModel

extension WorkspaceState {
    /// A state of loose terminals in the order given.
    init(
        terminals: [TerminalSummary], selected: TerminalID?, sidebarVisible: Bool = true, panelVisible: Bool = true,
        panelTab: PanelTab = .changes
    ) {
        self.init(
            sections: MockWorkspace.sections(terminals.map { .loose(MockTerminal($0)) }, selected: selected), selected: selected,
            sidebarVisible: sidebarVisible, panelVisible: panelVisible, panelTab: panelTab)
    }
}
