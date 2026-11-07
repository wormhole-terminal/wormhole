// The View menu: the columns, the panel's tabs, the palette and how the window looks.

import AppKit
import PlatformModel
import WindowUI
import WorkspaceModel

extension ActionRouter {
    func viewActions() -> [ActionID: Handler] {
        let hub = app.hub, look = app.look
        return [
            "toggleSidebar": { hub.send(.toggleSidebar) },
            "toggleRightPanel": { hub.send(.togglePanel) },
            "toggleStatusBar": { look.statusBar.toggle() },
            "toggleTerminalTabs": { look.tabsOnTop.toggle() },
            "showChanges": { hub.send(.showPanelTab(.changes)) },
            "showHistory": { hub.send(.showPanelTab(.history)) },
            "showNotes": { hub.send(.showPanelTab(.notes)) },
            "showUsage": { hub.send(.showPanelTab(.usage)) },
            "showProject": { hub.send(.showPanelTab(.project)) },
            "showFiles": { hub.send(.showPanelTab(.files)) },
            "commandPalette": { hub.send(.showOverlay(hub.state.overlay == .palette ? nil : .palette)) },
            "keyboardShortcuts": { hub.send(.showOverlay(hub.state.overlay == .shortcuts ? nil : .shortcuts)) },
            "bigger": { hub.zoomText(step: 1) },
            "smaller": { hub.zoomText(step: -1) },
            "actualSize": { hub.zoomText(step: nil) },
        ]
    }

    func findActions() -> [ActionID: Handler] {
        let hub = app.hub
        return [
            "find": { if !hub.sidebarSearch.revealIfFocused() { hub.find(.open) } },
            "findNext": { hub.find(.next) },
            "findPrevious": { hub.find(.previous) },
            "useSelectionForFind": { hub.find(.useSelection) },
        ]
    }
}
