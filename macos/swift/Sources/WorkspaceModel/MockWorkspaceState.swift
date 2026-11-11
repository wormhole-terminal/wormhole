// A window of loose terminals, for tests and previews.

extension WorkspaceState {
    /// A window whose terminals are all loose, in this order: the quick way to a state for a test or a preview.
    public init(
        terminals: [TerminalSummary], selected: TerminalID? = nil, sidebarVisible: Bool = true, panelVisible: Bool = true,
        panelTab: PanelTab = .changes, overlay: Overlay? = nil
    ) {
        self.init(
            sections: MockWorkspace.sections(terminals.map { .loose(MockTerminal($0)) }, selected: selected), selected: selected,
            sidebarVisible: sidebarVisible, panelVisible: panelVisible, panelTab: panelTab, overlay: overlay)
    }
}
