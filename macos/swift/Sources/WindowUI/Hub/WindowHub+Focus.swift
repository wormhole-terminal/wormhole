// What the keyboard's areas walk over, read from the window's state: the panel's tabs on offer and the status bar's items.

import WorkspaceModel

extension WindowHub {
    /// The panel's tabs on offer, in the bar's order: the core's (a feature that is off brings no tab; all six in the sample),
    /// less the ones their own rule hides (`TabDef::shown`): Usage without an agent on screen, Project without a folder.
    public var panelTabs: [PanelTab] {
        (state.layout.map { $0.tabs.filter(\.enabled).map(\.tab) } ?? PanelTab.barOrder).filter(isShown)
    }

    private func isShown(_ tab: PanelTab) -> Bool {
        switch tab {
        case .usage: agentOnScreen
        // It stays while it is the tab on show, so a folder not known for a moment does not pull it from under the person.
        case .project: state.panelTab == .project || state.selectedTerminal?.folder != nil
        default: true
        }
    }

    /// A coding agent runs in one of the terminals of the shown split (the Usage tab's reason to exist).
    var agentOnScreen: Bool {
        let shown = Set(state.panes.shown.map(\.id)).union(state.selected.map { [$0] } ?? [])
        return state.all.contains { shown.contains($0.id) && $0.agent != nil }
    }

    /// The tab on show went away (the agent left, the folder is unknown): back to the tab before Usage, else Changes.
    func leaveMissingTab() {
        guard !panelTabs.contains(state.panelTab) else { return }
        send(.showPanelTab(tabBeforeUsage ?? .changes))
    }

    /// The status bar's items the keys walk: the terminal that needs you, then each count's first terminal.
    var statusItems: [TerminalID] { StatusSummary(state.all).items }
}
