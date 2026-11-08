// What the keyboard's areas walk over, read from the window's state: the panel's tabs on offer and the status bar's items.

import WorkspaceModel

extension WindowHub {
    /// The panel's tabs on offer, in the bar's order: the core's (a feature that is off brings no tab); all six in the sample.
    var panelTabs: [PanelTab] { state.layout.map { $0.tabs.filter(\.enabled).map(\.tab) } ?? PanelTab.barOrder }

    /// The status bar's items the keys walk: the terminal that needs you, then each count's first terminal.
    var statusItems: [TerminalID] { StatusSummary(state.all).items }
}
